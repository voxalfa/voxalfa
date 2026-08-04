pub mod builder;
pub mod lyrics;
pub mod solfa;
pub mod utils;

use lyrics::LyricLineIR;
use solfa::SolfaLineIR;

use crate::{
    ast::{
        params::{SectionParams, SubSectionParams},
        symbols::ScopeId,
    },
    data_types::{ExtendedTempo, Mark, Voice},
    ir::solfa::PulseIR,
    output::event::{Event, EventKind, JumpEvent, get_note_ticks},
};

#[derive(Debug, Default)]
pub struct BodyIR {
    pub sections: Vec<SectionIR>,
}

#[derive(Debug, Default)]
pub struct SectionIR {
    pub sid: ScopeId,
    pub items: Vec<SubSectionIR>,
    pub params: SectionParams,
    pub merge: bool,
}

impl SectionIR {
    pub fn get_verses(&self, voice: &Voice) -> Option<&[LyricLineIR]> {
        self.items.iter().find_map(|s| {
            s.solfa
                .iter()
                .any(|s| s.voice == *voice)
                .then_some(s.lyrics.as_slice())
        })
    }

    pub fn start_events(&self) -> Vec<Event> {
        let mut result = Vec::new();
        let params = &self.params;

        if let Some(time) = &self.params.time {
            result.push(Event::with(time.value));
        }

        if let Some(tempo) = &self.params.tempo {
            let event = match tempo.value {
                ExtendedTempo::Progressive(kind) => Event::new(EventKind::TempoStart(kind)),
                ExtendedTempo::Static(tempo) => Event::with(tempo),
            };

            result.push(event);
        }

        if let Some(makr) = &self.params.mark
            && matches!(makr.value, Mark::Coda | Mark::Segno)
        {
            result.push(Event::with(makr.value));
        }

        if let Some(key) = &params.key {
            result.push(Event::with(key.value));
        }

        if let Some(ending) = &params.ending {
            result.push(Event::new(EventKind::EndingStart(ending.value as u8)));
        }

        result
    }

    pub fn end_events(&self) -> Vec<Event> {
        let mut result = Vec::new();
        let params = &self.params;

        if let Some(symbol) = &params.mark
            && matches!(symbol.value, Mark::ToCoda | Mark::Fine)
        {
            result.push(Event::with(symbol.value));
        }

        if let Some(tempo) = &self.params.tempo
            && let ExtendedTempo::Progressive(kind) = tempo.value
        {
            result.push(Event::new(EventKind::TempoEnd(kind)));
        }

        if let Some(ending) = &params.ending {
            result.push(Event::new(EventKind::EndingEnd(ending.value as u8)));
        }

        if let Some(jump) = &params.jump {
            result.push(Event::new(EventKind::Jump(JumpEvent {
                kind: jump.value,
                repeat: params.repeat.as_ref().map(|r| r.value as u8).unwrap_or(1),
            })));
        }

        result
    }
}

#[derive(Debug, Default)]
pub struct SubSectionIR {
    pub sid: ScopeId,
    pub params: SubSectionParams,
    pub views: Vec<PulseView>,
    pub solfa: Vec<SolfaLineIR>,
    pub lyrics: Vec<LyricLineIR>,
}

impl SubSectionIR {
    pub fn width(&self) -> usize {
        self.views.iter().map(|v| v.durations.len()).sum()
    }

    pub fn get_ticks(&self) -> usize {
        get_note_ticks(self.views.len(), 1)
    }
}

#[derive(Debug)]
pub struct PulseView {
    pub durations: Vec<usize>,
    pub factor: usize,
    pub aligned: bool,
}

impl PulseView {
    pub fn new(pulse: &PulseIR) -> Self {
        Self {
            durations: pulse.columns.iter().map(|c| c.duration).collect(),
            factor: pulse.factor,
            aligned: true,
        }
    }

    pub fn add(&mut self, pulse: &PulseIR) {
        let duration_match = pulse
            .columns
            .iter()
            .map(|p| p.duration)
            .zip(&self.durations)
            .all(|(lhs, &rhs)| lhs == rhs);

        if pulse.factor != self.factor || !duration_match {
            self.aligned = false;
            self.factor = 1;
            self.durations = vec![1];
        }
    }

    pub fn resolve_widths(&self, col_width: usize, col_factor: usize) -> Vec<usize> {
        self.durations
            .iter()
            .map(|d| (col_width * d * col_factor) / self.factor)
            .collect()
    }
}
