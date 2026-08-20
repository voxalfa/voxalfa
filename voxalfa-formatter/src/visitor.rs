use voxalfa_core::{
    ast::lyrics::{LyricOperatorKind, LyricSpecialChar},
    output::lyrics::{LyricEvent, LyricVisitor},
};

#[derive(Default)]
pub struct FormatterVisitor {
    buffer: String,
}

impl FormatterVisitor {
    fn push_special_char(&mut self, ch: LyricSpecialChar) {
        let value = match ch {
            LyricSpecialChar::Backslash => "&bls",
            LyricSpecialChar::Tilde => "&tld",
            LyricSpecialChar::Backtick => "&btk",
            LyricSpecialChar::LeftChrevron => "&lch",
            LyricSpecialChar::RightChevron => "&rch",
            LyricSpecialChar::Slash => "&sls",
            LyricSpecialChar::LeftParen => "&lpr",
            LyricSpecialChar::RightParen => "&rpr",
            LyricSpecialChar::At => "&atr",
            LyricSpecialChar::Ampersand => "&amp",
            LyricSpecialChar::Semicolumn => "&scl",
            LyricSpecialChar::Dot => "&dot",
        };

        self.buffer.push_str(value);
    }
}

impl LyricVisitor for FormatterVisitor {
    fn get_operator(operator: LyricOperatorKind) -> Option<char> {
        match operator {
            LyricOperatorKind::Space => Some(' '),
            LyricOperatorKind::Concat => Some('_'),
            LyricOperatorKind::Newline => Some('\\'),
        }
    }

    fn handle_event(&mut self, event: LyricEvent) {
        match event {
            LyricEvent::UnderlineStart | LyricEvent::UnderlineEnd => self.buffer.push('`'),
            LyricEvent::GroupStart => self.buffer.push('('),
            LyricEvent::GroupEnd => self.buffer.push(')'),
            LyricEvent::Placeholder => self.buffer.push('~'),
            LyricEvent::Text(text) => self.buffer.push_str(text),
            LyricEvent::SpecialChar(ch) => self.push_special_char(ch),
            LyricEvent::Span(span) if span > 1 => self.buffer.push_str(&format!("@{span}")),
            LyricEvent::Operator(op) if let Some(ch) = Self::get_operator(op) => {
                self.buffer.push(ch)
            }
            _ => {}
        }
    }

    fn into_string(self) -> String {
        self.buffer
    }
}
