pub mod error;
pub mod types;

pub use error::{Error, LinkError, LinkErrorKind, Result};
pub use types::{Operator, Token, TokenizerOutput, Verse};

use types::{Slot, UnderfitLine};

#[derive(Debug)]
pub struct Linker<'a> {
    source: &'a str,
    lines: Vec<&'a str>,
    tokenizer: &'a TokenizerOutput,
    cursors: Vec<usize>,
    output: String,
}

impl<'a> Linker<'a> {
    pub fn new(source: &'a str, tokenizer: &'a TokenizerOutput) -> Self {
        Self {
            source,
            lines: source.lines().collect(),
            tokenizer,
            cursors: vec![0; tokenizer.verses.len()],
            output: String::new(),
        }
    }

    pub fn link(mut self) -> Result<String> {
        let mut errors = Vec::new();
        let mut underfit = vec![false; self.tokenizer.verses.len()];

        for line_id in 0..self.lines.len() {
            self.process_line(line_id, &mut errors, &mut underfit)?;
        }

        for (verse_id, verse) in self.tokenizer.verses.iter().enumerate() {
            if underfit[verse_id] {
                continue;
            }

            let extra = verse.tokens.len().saturating_sub(self.cursors[verse_id]);

            if extra > 0 {
                errors.push(LinkErrorKind::Overfit {
                    verse: verse_id + 1,
                    extra,
                });
            }
        }

        if !errors.is_empty() {
            return Err(Error::Link(LinkError::new(errors, self.output)));
        }

        if !self.source.is_empty() && !self.source.ends_with('\n') {
            self.output.pop();
        }

        Ok(self.output)
    }

    fn process_line(
        &mut self,
        line_id: usize,
        errors: &mut Vec<LinkErrorKind>,
        underfit: &mut [bool],
    ) -> Result<()> {
        let line = self.lines[line_id];

        let Some(rest) = line.trim_start().strip_prefix(";; @slots") else {
            self.output.push_str(line);
            self.output.push('\n');
            return Ok(());
        };

        let slots = match rest
            .split_whitespace()
            .map(Slot::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
        {
            Ok(slots) => slots,
            Err(slot) => {
                self.output.push_str(line);
                self.output.push('\n');

                return Err(Error::Link(LinkError::new(
                    vec![LinkErrorKind::UnknownSlot { slot }],
                    std::mem::take(&mut self.output),
                )));
            }
        };

        let is_last_section = !self.lines[line_id + 1..]
            .iter()
            .any(|line| matches!(line.trim(), "--" | "<<" | "=="));

        for (verse_id, failed) in underfit.iter_mut().enumerate() {
            if *failed {
                continue;
            }

            if let Err(kind) = self.process_verse(verse_id, &slots, is_last_section) {
                *failed = true;
                errors.push(kind);
            }
        }

        Ok(())
    }

    fn process_verse(
        &mut self,
        verse_id: usize,
        slots: &[Slot],
        is_last_section: bool,
    ) -> std::result::Result<(), LinkErrorKind> {
        let mut segment = match self.link_slots(slots, verse_id) {
            Ok(segment) => segment,
            Err(underfit) => {
                self.output.push_str(&format!(
                    "[{}] {}\n",
                    verse_id + 1,
                    underfit.content.trim_end()
                ));

                return Err(LinkErrorKind::Underfit {
                    verse: verse_id + 1,
                    missing: underfit.missing,
                });
            }
        };

        if is_last_section {
            segment.truncate(segment.trim_end().len());
        } else if !segment.is_empty() {
            if !segment.ends_with([' ', '_', '\\']) {
                segment.push(' ');
            }

            segment.push_str("@@");
        }

        self.output
            .push_str(&format!("[{}] {segment}\n", verse_id + 1));

        Ok(())
    }

    fn link_slots(
        &mut self,
        slots: &[Slot],
        verse_id: usize,
    ) -> std::result::Result<String, UnderfitLine> {
        let verse = &self.tokenizer.verses[verse_id];
        let cursor = &mut self.cursors[verse_id];

        let mut content = String::new();
        let mut missing = slots.iter().filter(|&slot| *slot == Slot::Token).count();
        let mut slot_id = 0;

        while slot_id < slots.len() {
            let mut span = 1;

            while matches!(slots.get(slot_id + span), Some(Slot::Ext)) {
                span += 1;
            }

            match slots[slot_id] {
                Slot::Token => {
                    let Some(token) = verse.tokens.get(*cursor) else {
                        return Err(UnderfitLine {
                            content: content.trim_end().to_string(),
                            missing,
                        });
                    };

                    missing -= 1;
                    *cursor += 1;
                    content.push_str(&token.format());

                    if span > 1 {
                        content.push_str(&format!("@{span}"));
                    }

                    if let Some(operator) = token.operator {
                        content.push(operator.as_char());
                    }
                }
                Slot::Rest => {
                    content.push('~');

                    if span > 1 {
                        content.push_str(&format!("@{span}"));
                    }

                    content.push(' ');
                }
                Slot::Ext => {}
            }

            slot_id += span;
        }

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::{types::*, *};

    fn token(parts: &[&str], operator: Option<Operator>) -> Token {
        Token {
            parts: parts.iter().map(|part| part.to_string()).collect(),
            operator,
        }
    }

    fn verse(tokens: Vec<Token>) -> Verse {
        Verse { tokens }
    }

    fn output(verses: Vec<Verse>) -> TokenizerOutput {
        TokenizerOutput {
            verses,
            meter: None,
        }
    }

    fn slots(values: &[Slot]) -> Vec<Slot> {
        values.to_vec()
    }

    #[test]
    fn test_link_with_spans() {
        let slots = slots(&[
            Slot::Token,
            Slot::Token,
            Slot::Ext,
            Slot::Rest,
            Slot::Ext,
            Slot::Token,
        ]);

        let tokenizer_output = output(vec![verse(vec![
            token(&["Hel"], Some(Operator::Concat)),
            token(&["lo"], Some(Operator::Space)),
            token(&["world"], Some(Operator::Newline)),
        ])]);

        let mut linker = Linker::new("", &tokenizer_output);
        let content = linker.link_slots(&slots, 0).unwrap();

        assert_eq!(linker.cursors[0], 3);
        assert_eq!(content, "Hel_lo@2 ~@2 world\\");
    }

    #[test]
    fn test_group_parts() {
        let slots = slots(&[Slot::Token]);
        let tokenizer_output = output(vec![verse(vec![token(&["An", "dria", "na"], None)])]);
        let mut linker = Linker::new("", &tokenizer_output);
        let content = linker.link_slots(&slots, 0).unwrap();

        assert_eq!(content, "(An dria na)");
    }

    #[test]
    fn test_underfit() {
        let slots = slots(&[Slot::Token, Slot::Token, Slot::Token]);
        let tokenizer_output = output(vec![verse(vec![token(&["one"], Some(Operator::Space))])]);
        let mut linker = Linker::new("", &tokenizer_output);
        let underfit = linker.link_slots(&slots, 0).unwrap_err();

        assert_eq!(underfit.content, "one");
        assert_eq!(underfit.missing, 2);
    }

    #[test]
    fn test_overfit() {
        let content = ";; @slots <TOKEN>\n";
        let tokenizer_output = output(vec![verse(vec![
            token(&["one"], Some(Operator::Space)),
            token(&["two"], None),
        ])]);

        let error = Linker::new(content, &tokenizer_output).link().unwrap_err();

        assert_eq!(error.partial(), Some("[1] one\n"));
        assert_eq!(error.to_string(), "verse 1: 1 extra token(s)");
    }

    #[test]
    fn test_stream_slots() {
        let content = "\
[S] |d :r ||
;; @slots <TOKEN> <TOKEN>

--

[S] |m :f ||
;; @slots <TOKEN> <TOKEN>
";
        let tokenizer_output = output(vec![verse(vec![
            token(&["a"], Some(Operator::Space)),
            token(&["b"], Some(Operator::Space)),
            token(&["c"], Some(Operator::Space)),
            token(&["d"], None),
        ])]);

        let linked = Linker::new(content, &tokenizer_output).link().unwrap();

        assert_eq!(
            linked,
            "\
[S] |d :r ||
[1] a b @@

--

[S] |m :f ||
[1] c d
"
        );
    }

    #[test]
    fn test_multiple_verses() {
        let content = "[S] |d :r !m :f ||\n;; @slots <TOKEN> <TOKEN> <REST> <TOKEN>\n";

        let tokenizer_output = output(vec![
            verse(vec![
                token(&["do"], Some(Operator::Space)),
                token(&["re"], Some(Operator::Space)),
                token(&["mi"], None),
            ]),
            verse(vec![
                token(&["one"], Some(Operator::Space)),
                token(&["two"], Some(Operator::Space)),
                token(&["three"], None),
            ]),
        ]);

        let linked = Linker::new(content, &tokenizer_output).link().unwrap();

        assert_eq!(
            linked,
            "[S] |d :r !m :f ||\n[1] do re ~ mi\n[2] one two ~ three\n"
        );
    }

    #[test]
    fn test_partial_output() {
        let content = "[S] |d :r ||\n;; @slots <TOKEN> <TOKEN>\n";
        let tokenizer_output = output(vec![verse(vec![token(&["only"], Some(Operator::Space))])]);
        let error = Linker::new(content, &tokenizer_output).link().unwrap_err();

        assert_eq!(error.to_string(), "verse 1: 1 token(s) missing");
        assert_eq!(error.partial(), Some("[S] |d :r ||\n[1] only\n"));
    }

    #[test]
    fn test_all_verse_errors() {
        let content = ";; @slots <TOKEN>\n";
        let tokenizer_output = output(vec![
            verse(vec![
                token(&["one"], Some(Operator::Space)),
                token(&["two"], None),
            ]),
            verse(vec![
                token(&["red"], Some(Operator::Space)),
                token(&["blue"], None),
            ]),
        ]);

        let error = Linker::new(content, &tokenizer_output).link().unwrap_err();

        assert_eq!(
            error.to_string(),
            "verse 1: 1 extra token(s)\nverse 2: 1 extra token(s)"
        );
    }
}
