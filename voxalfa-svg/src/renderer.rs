use crate::{
    error::{Error, Result},
    fonts::FontInterface,
};

use voxalfa_core::{
    ast::symbols::SymbolRef,
    data_types::TimeSignature,
    ir::{SectionIr, solfa::NoteKind},
    output::{FinalOutput, voice::VoiceSet},
};

#[allow(unused)]
pub struct Renderer<'a> {
    data: FinalOutput,
    font: FontInterface<'a>,
    time: TimeSignature,
}

impl<'a> Renderer<'a> {
    pub fn new(data: FinalOutput) -> Result<Self> {
        let font = FontInterface::new()?;

        let time = *data
            .get_header_params(|p| p.time.as_ref())
            .ok_or(Error::MissingHeaderField("time"))?;

        Ok(Self { data, font, time })
    }

    fn todo(&self) -> () {
        //
    }
}
