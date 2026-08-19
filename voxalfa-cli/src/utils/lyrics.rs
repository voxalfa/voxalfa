use voxalfa_core::{
    ast::lyrics::LyricOperatorKind,
    output::lyrics::{LyricEvent, LyricVisitor},
};

#[derive(Debug, Default)]
pub struct CliVisitor {
    buffer: String,
}

impl LyricVisitor for CliVisitor {
    fn handle_event(&mut self, event: LyricEvent) {
        match event {
            LyricEvent::UnderlineStart => self.buffer.push_str("\x1b[4m"),
            LyricEvent::UnderlineEnd => self.buffer.push_str("\x1b[24m"),
            LyricEvent::Operator(LyricOperatorKind::Space) => self.buffer.push(' '),
            LyricEvent::Operator(LyricOperatorKind::Newline) => self.buffer.push('\n'),
            LyricEvent::Text(text) => self.buffer.push_str(text),
            LyricEvent::SpecialChar(ch) => self.buffer.push_str(&ch.to_string()),
            _ => {}
        }
    }

    fn into_string(self) -> String {
        self.buffer
    }
}
