use voxalfa_validator::data_types::{
    Dynamic, Key, List, Marker, Tempo, TimeSignature, TimedValue, Voice,
};

pub trait Formattable {
    fn format(&self, embedded: bool) -> String;

    fn is_spaced() -> bool {
        false
    }
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
    fn format(&self, embedded: bool) -> String {
        if embedded {
            format!("{self}")
        } else {
            format!("{{{self}}}")
        }
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
    fn format(&self, embedded: bool) -> String {
        if embedded {
            self.to_string()
        } else {
            format!("{{{self}}}")
        }
    }
}

impl Formattable for Marker {
    fn format(&self, embedded: bool) -> String {
        if embedded {
            self.to_string()
        } else {
            format!("{{{self}}}")
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

impl Formattable for Tempo {
    fn format(&self, embedded: bool) -> String {
        if embedded {
            self.to_string()
        } else {
            format!("{{{self}}}")
        }
    }
}

impl<T: Formattable> Formattable for List<T> {
    fn format(&self, _embedded: bool) -> String {
        if self.len() > 1 {
            let separator = if T::is_spaced() { ", " } else { "," };

            let inner = self
                .iter()
                .map(|t| t.value.format(true))
                .collect::<Vec<_>>()
                .join(separator);

            format!("{{{inner}}}")
        } else {
            self[0].value.format(false) // list should always have one item
        }
    }
}

impl<T: Formattable> Formattable for TimedValue<T> {
    fn format(&self, embedded: bool) -> String {
        let inner = self.value.format(true);

        let value = match (self.start, self.end) {
            (0., None) => inner,
            (start, None) => format!("{inner}:{start}"),
            (start, Some(end)) => format!("{inner}:{start}..{end}"),
        };

        if embedded {
            value
        } else {
            format!("{{{value}}}")
        }
    }

    fn is_spaced() -> bool {
        true
    }
}
