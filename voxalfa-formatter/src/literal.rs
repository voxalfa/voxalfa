use voxalfa_validator::ast::types::{Key, TimeSignature, Voice};

pub trait Formattable {
    fn format(&self) -> String;
}

impl Formattable for String {
    fn format(&self) -> String {
        format!("\"{self}\"")
    }
}

impl Formattable for usize {
    fn format(&self) -> String {
        self.to_string()
    }
}

impl Formattable for Key {
    fn format(&self) -> String {
        self.to_string()
    }
}

impl Formattable for TimeSignature {
    fn format(&self) -> String {
        format!("{{{},{}}}", self.top, self.bottom)
    }
}

impl Formattable for Voice {
    fn format(&self) -> String {
        format!("{self:?}")
    }
}

impl<T: Formattable> Formattable for Vec<T> {
    fn format(&self) -> String {
        if self.len() > 1 {
            let inner = self
                .iter()
                .map(|t| t.format())
                .collect::<Vec<_>>()
                .join(",");

            format!("{{{inner}}}")
        } else {
            self[0].format() // list should always have one item
        }
    }
}
