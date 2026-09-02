use voxalfa_core::{
    ast::lyrics::LyricOperatorKind,
    output::lyrics::{LyricEvent, LyricVisitor},
};

#[derive(Default)]
pub struct SvgVisitor {
    buffer: String,
}

impl LyricVisitor for SvgVisitor {
    fn get_operator(operator: LyricOperatorKind) -> Option<char> {
        match operator {
            LyricOperatorKind::Space => Some(' '),
            LyricOperatorKind::Concat | LyricOperatorKind::Newline => None,
        }
    }

    fn handle_event(&mut self, event: LyricEvent) {
        match event {
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
