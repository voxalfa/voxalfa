#[derive(Debug, Default, Clone)]
pub struct Injection {
    pub(crate) slots: bool,
    pub(crate) metadata: Vec<(String, String)>,
}

impl Injection {
    pub fn slots() -> Self {
        Self {
            slots: true,
            metadata: Vec::new(),
        }
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(injected_key, _)| injected_key == key)
            .map(|(_, value)| value.as_str())
    }
}
