use crate::{
    ast::{lyrics::LyricOperatorKind, symbols::VoiceId},
    ir::solfa::PulseColumn,
    output::event::{Event, EventAnchor, NoteTimeline, Timestamp, get_note_ticks},
};

#[derive(Debug)]
pub struct VoiceLine<'a> {
    pub voice_id: VoiceId,
    pub timeline: NoteTimeline,
    pub notes: Vec<NoteContext<'a>>,
}

impl<'a> VoiceLine<'a> {
    pub fn new(
        voice_id: VoiceId,
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
                let note = match event.anchor {
                    EventAnchor::Before => index,
                    EventAnchor::After => index.saturating_sub(1),
                };

                timeline.add_event(note, event.clone());
            }

            if let Some(ctx) = notes.get(index) {
                ticks += get_note_ticks(ctx.column.duration, ctx.factor);
            }
        }

        Self {
            voice_id,
            timeline,
            notes,
        }
    }
}

#[derive(Debug)]
pub struct NoteContext<'a> {
    pub column: &'a PulseColumn,
    pub factor: u8,
    pub lyric_id: usize, // TODO: remove
    pub lyrics: Vec<LyricContext>,
    pub section_id: usize,
    pub sub_section_id: usize,
    pub measure_start: bool,
}

#[derive(Debug)]
pub struct LyricContext {
    pub verse: usize,
    pub sid: usize,
    pub match_kind: LyricMatch,
    pub operator: Option<LyricOperatorKind>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LyricMatch {
    Exact,
    Implicit,
    Extended,
}
