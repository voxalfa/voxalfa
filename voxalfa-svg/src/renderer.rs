use crate::{
    error::{Error, Result},
    fonts::FontInterface,
};

use voxalfa_core::{
    ast::symbols::SymbolRef,
    data_types::TimeSignature,
    ir::SectionIr,
    output::{FinalOutput, voice::VoiceSet},
};

#[allow(unused)]
pub struct Renderer<'a> {
    data: FinalOutput,
    font: FontInterface<'a>,
    time: TimeSignature,
}

#[derive(Debug, Default, Clone)]
struct MeasureUnit {
    index: usize,
    pulse_count: u8,
    // voice_splits: Vec<VoiceSet>,
}

impl MeasureUnit {
    fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl<'a> Renderer<'a> {
    pub fn new(data: FinalOutput) -> Result<Self> {
        let font = FontInterface::new()?;

        let time = *data
            .get_header_params(|p| p.time.as_ref())
            .ok_or(Error::MissingHeaderField("time"))?;

        Ok(Self { data, font, time })
    }

    fn build_measure_units(&self) -> Vec<MeasureUnit> {
        let mut results = Vec::new();
        let mut current_measure = MeasureUnit::default();

        for section in &self.data.body.sections {
            if section.items.len() > 1 {
                unimplemented!("TODO: voice splits");
            }

            let solfa_ref = section.items.first().and_then(|sub| sub.solfa.first());
            let Some(solfa_ref) = solfa_ref else { continue };

            for sub_section in &section.items {
                for (voice_id, solfa) in sub_section.solfa.iter().enumerate() {
                    //
                }
            }
        }

        results
    }
}
