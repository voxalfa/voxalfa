use crate::{
    ast::symbols::VoiceId,
    data_types::Voice,
    ir::solfa::PulseColumn,
    output::event::{Event, NoteTimeline, Timestamp, get_note_ticks},
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct VoiceSet(u8);

impl VoiceSet {
    pub fn new<T>(voices: T) -> Self
    where
        T: IntoIterator<Item = VoiceId>,
    {
        let mut flags = 0;

        for voice_id in voices.into_iter() {
            flags |= 1 << voice_id as u8;
        }

        Self(flags)
    }

    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

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
                ticks += get_note_ticks(ctx.column.duration, ctx.factor);
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
    pub column: &'a PulseColumn,
    pub factor: u8,
    pub lyric_id: usize,
    pub section_id: usize,
    pub sub_section_id: usize,
}
