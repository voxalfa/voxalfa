use crate::ast::{
    body::Body,
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
            .params
            .voices
            .as_ref()
            .and_then(|v| v.value.get(id))
            .copied()
    }

    pub fn time_signature(&self) -> Option<&SymbolRef<TimeSignature>> {
        self.header.params.time.as_ref()
    }
}
