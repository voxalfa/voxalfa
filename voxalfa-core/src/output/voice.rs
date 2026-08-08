use crate::{
    data_types::Voice,
    ir::solfa::PulseColumn,
    output::event::{Event, NoteTimeline, Timestamp, get_note_ticks},
};

#[derive(Debug)]
pub struct VoiceLine<'a> {
    pub voice: Voice,
    pub timeline: NoteTimeline,
    pub notes: Vec<NoteContext<'a>>,
}

impl<'a> VoiceLine<'a> {
    pub fn new(
        voice: Voice,
        notes: Vec<NoteContext<'a>>,
        flat_timeline: Vec<&(Timestamp, Event)>,
    ) -> Self {
        let mut ticks = 0;
        let mut timeline = NoteTimeline::default();

        for index in 0..=notes.len() {
            let events = flat_timeline
                .iter()
                .filter(|(t, _e)| *t == ticks)
                .map(|(_, e)| e);

            for event in events {
                timeline.add_event(index, event.clone());
            }

            if let Some(ctx) = notes.get(index) {
                ticks += get_note_ticks(ctx.note.duration, ctx.factor);
            }
        }

        Self {
            voice,
            timeline,
            notes,
        }
    }
}

#[derive(Debug)]
pub struct NoteContext<'a> {
    pub note: &'a PulseColumn,
    pub factor: usize,
}
