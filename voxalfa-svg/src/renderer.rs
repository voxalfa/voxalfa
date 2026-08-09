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
        let time = data.header.get_params(|p| &p.time);

        match time {
            Some(&time) => Ok(Self { data, font, time }),
            None => Err(Error::MissingHeaderField("time")),
        }
    }

    fn todo(&self) {
        //
    }
}
