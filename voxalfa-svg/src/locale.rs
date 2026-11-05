#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Fr,
    Mg,
}

impl Locale {
    pub fn from_language(language: Option<&str>) -> Self {
        let language = language.unwrap_or("en").trim().to_ascii_lowercase();
        let primary = language.split(['-', '_']).next().unwrap_or("");

        match primary {
            "fr" | "fra" | "fre" => Self::Fr,
            "mg" | "mlg" => Self::Mg,
            _ => Self::En,
        }
    }

    pub fn key_label(self, key: &str) -> String {
        let prefix = match self {
            Self::En => "Do is",
            Self::Fr => "Do est",
            Self::Mg => "Dô dia",
        };

        format!("{prefix} {key}")
    }
}
