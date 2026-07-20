use voxalfa_validator::ast::types::{Dynamic, Key, TimeSignature, Voice};

pub trait Formattable {
    fn format(&self, embedded: bool) -> String;
}

impl Formattable for String {
    fn format(&self, _embedded: bool) -> String {
        format!("\"{self}\"")
    }
}

impl Formattable for usize {
    fn format(&self, embedded: bool) -> String {
        if embedded {
            self.to_string()
        } else {
            format!("{{{self}}}")
        }
    }
}

impl Formattable for Key {
    fn format(&self, _embedded: bool) -> String {
        format!("{{{self}}}")
    }
}

impl Formattable for TimeSignature {
    fn format(&self, _embedded: bool) -> String {
        format!("{{{},{}}}", self.top, self.bottom)
    }
}

impl Formattable for Voice {
    fn format(&self, embedded: bool) -> String {
        if embedded {
            format!("{self:?}")
        } else {
            format!("{{{self:?}}}")
        }
    }
}

impl Formattable for Dynamic {
    fn format(&self, _embedded: bool) -> String {
        if self.start != self.end {
            format!("{{{},{}}}", self.start, self.end)
        } else {
            format!("{{{}}}", self.start)
        }
    }
}

impl Formattable for bool {
    fn format(&self, embedded: bool) -> String {
        let s = if *self { "true" } else { "false" };

        if embedded {
            s.to_string()
        } else {
            format!("{{{s}}}")
        }
    }
}

impl<T: Formattable> Formattable for Vec<T> {
    fn format(&self, _embedded: bool) -> String {
        if self.len() > 1 {
            let inner = self
                .iter()
                .map(|t| t.format(true))
                .collect::<Vec<_>>()
                .join(",");

            format!("{{{inner}}}")
        } else {
            self[0].format(false) // list should always have one item
        }
    }
}
