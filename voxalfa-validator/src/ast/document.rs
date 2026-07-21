use crate::ast::{
    body::{Body, Section},
    header::Header,
    symbols::SymbolRef,
    types::{TimeSignature, Voice},
};

#[derive(Debug, Default)]
pub struct Document {
    pub header: Header,
    pub body: Body,
}

impl Document {
    pub fn get_voice(&self, id: usize) -> Option<Voice> {
        self.header
            .metadata
            .voices
            .as_ref()
            .and_then(|v| v.value.get(id))
            .copied()
    }

    pub fn voices(&self) -> Option<&SymbolRef<Vec<Voice>>> {
        self.header.metadata.voices.as_ref()
    }

    pub fn verses(&self) -> Option<&SymbolRef<usize>> {
        self.header.metadata.verses.as_ref()
    }

    pub fn time_signature(&self, section: &Section) -> Option<SymbolRef<TimeSignature>> {
        section
            .params
            .time
            .as_ref()
            .or(self.header.params.time.as_ref())
            .cloned()
    }
}
