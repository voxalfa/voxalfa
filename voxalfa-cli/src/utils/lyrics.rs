use voxalfa_core::{
    ast::lyrics::LyricOperatorKind,
    output::lyrics::{LyricEvent, LyricVisitor},
};

#[derive(Debug, Default)]
pub struct CliVisitor {
    buffer: String,
}

impl LyricVisitor for CliVisitor {
    fn get_operator(operator: LyricOperatorKind) -> Option<char> {
        match operator {
            LyricOperatorKind::Space => Some(' '),
            LyricOperatorKind::Concat => None,
            LyricOperatorKind::Newline => Some('\n'),
        }
    }

    fn handle_event(&mut self, event: LyricEvent) {
        match event {
            LyricEvent::UnderlineStart => self.buffer.push_str("\x1b[4m"),
            LyricEvent::UnderlineEnd => self.buffer.push_str("\x1b[24m"),
            LyricEvent::Text(text) => self.buffer.push_str(text),
            LyricEvent::SpecialChar(ch) => self.buffer.push_str(&ch.to_string()),
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
