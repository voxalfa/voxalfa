use std::collections::HashMap;

use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u28},
};
use voxalfa_validator::{
    ast::solfa::Note,
    data_types::{Dynamic, Key, Voice},
    output::{
        NoteContext,
        event::{Event, EventKind, NoteTimeline},
    },
};

use crate::{
    BASE_MIDI_KEY, PPQ,
    dynamics::{DynamicState, DynamicTransition, DynamicTransitionKind},
    error::{ConvertError, Result},
};

#[derive(Debug)]
pub struct VoiceTask {
    index: usize,
    jump: Option<usize>,
    channel: u4,
    params: PlaybackParams,
    voice: Voice,
    track: Track<'static>,
    active_note: Option<u7>,
    pending_ticks: u32,
    pending_notes: Vec<PendingNote>,
    dynamic: DynamicState,
    slur: bool,
    marks: [usize; 3],
    endings_jump: HashMap<usize, usize>,
    jump_table: HashMap<usize, u8>,
}

impl VoiceTask {
    pub fn new(id: usize, voice: Voice, params: PlaybackParams) -> Self {
        Self {
            voice,
            params,
            index: 0,
            channel: u4::from(id as u8),
            track: Track::new(),
            active_note: None,
            pending_ticks: 0,
            pending_notes: Vec::new(),
            dynamic: DynamicState::default(),
            endings_jump: HashMap::new(),
            jump_table: HashMap::new(),
            marks: [0; 3],
            slur: false,
            jump: None,
        }
    }

    pub fn get_midi_note(&self, note: Note) -> Result<u7> {
        let result = BASE_MIDI_KEY + self.params.key.offset() + note.offset() + self.voice.offset();

        if !(0..=127).contains(&result) {
            Err(ConvertError::InvalidMidiKey(result))
        } else {
            Ok(u7::from(result as u8))
        }
    }

    pub fn handle_note(&mut self, note: Note, ctx: &NoteContext<'_>) -> Result<()> {
        if self.dynamic.transition.is_some() {
            let key = self.get_midi_note(note)?;
            let duration = self.get_midi_note_ticks(ctx);

            self.pending_notes.push(PendingNote { key, duration });
        } else {
            self.apply_note_context(ctx);
            self.handle_active_note(0);

            let midi_note = self.get_midi_note(note)?;
            let velocity = self.get_velocity(self.dynamic.current);

            self.note_on(midi_note, velocity);
            self.active_note = Some(midi_note);
            self.pending_ticks = self.get_midi_note_ticks(ctx);
        }

        Ok(())
    }

    pub fn handle_pause(&mut self, ctx: &NoteContext<'_>) {
        self.apply_note_context(ctx);

        let ticks = self.get_midi_note_ticks(ctx);

        if self.handle_active_note(ticks) {
            self.pending_ticks = ticks;
        } else {
            self.pending_ticks += ticks;
        }
    }

    pub fn prolongate(&mut self, ctx: &NoteContext<'_>) {
        let ticks = self.get_midi_note_ticks(ctx);

        if self.dynamic.transition.is_some()
            && let Some(last) = self.pending_notes.last_mut()
        {
            last.duration += ticks;
        } else {
            self.apply_note_context(ctx);
            self.pending_ticks += ticks;
        }
    }

    pub fn finalize(mut self) -> Track<'static> {
        self.handle_active_note(0);

        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        self.track
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn jump(&mut self) -> bool {
        if let Some(pointer) = self.jump.take() {
            self.index = pointer;
            self.handle_active_note(0);
            true
        } else {
            false
        }
    }

    pub fn step(&mut self) {
        self.index += 1;
    }

    pub fn handle_events(&mut self, timeline: &NoteTimeline) {
        let events = timeline.get_events(self.index);

        for event in events {
            self.handle_event(event);
        }

        if self.dynamic.update
            && let Some(transition) = self.dynamic.transition.take()
        {
            self.dynamic.update = false;
            self.handle_pending_notes(transition);
        }
    }

    pub fn handle_pending_events(&mut self, timeline: &NoteTimeline) {
        self.handle_events(timeline);
        self.jump();
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event.kind {
            EventKind::Key(key) => {
                self.params.key = key;
            }

            EventKind::Dynamic(Dynamic::Cre | Dynamic::Dec)
                if self.dynamic.transition.is_some() =>
            {
                self.dynamic.update = true;
            }

            EventKind::Dynamic(Dynamic::Cre) => {
                self.dynamic.transition = Some(DynamicTransition {
                    level: self.dynamic.current,
                    kind: DynamicTransitionKind::Cre,
                });
            }

            EventKind::Dynamic(Dynamic::Dec) => {
                self.dynamic.transition = Some(DynamicTransition {
                    level: self.dynamic.current,
                    kind: DynamicTransitionKind::Dec,
                });
            }

            EventKind::Dynamic(dynamic) => {
                self.dynamic.current = dynamic;
            }

            EventKind::Mark(mark) => {
                self.marks[mark as usize] = self.index;
            }

            EventKind::Jump(jump) => {
                if !self.jump_table.contains_key(&self.index) {
                    self.jump = Some(self.marks[jump.mark() as usize]);
                    self.jump_table.insert(self.index, 0); // TODO: actual repeat count
                }
            }

            EventKind::EndingStart(id) => {
                if let Some(address) = self.endings_jump.get(&id) {
                    self.index = *address;
                }
            }

            EventKind::EndingEnd(id) => {
                self.endings_jump.insert(id, self.index + 1);
            }
        }
    }

    fn handle_active_note(&mut self, ticks: u32) -> bool {
        if let Some(last_note) = self.active_note.take() {
            self.note_off(last_note);
            self.pending_ticks = ticks;

            true
        } else {
            false
        }
    }

    fn handle_pending_notes(&mut self, transition: DynamicTransition) {
        let notes = std::mem::take(&mut self.pending_notes);

        if notes.is_empty() {
            return;
        }

        let start_vel = u8::from(self.get_velocity(transition.level)) as f32;
        let end_vel = u8::from(self.get_target_velocity(transition)) as f32;

        let total_ticks = notes.iter().map(|n| n.duration).sum::<u32>();
        let mut elapsed_ticks: u32 = 0;

        for note in notes {
            let progress = if total_ticks > 0 {
                elapsed_ticks as f32 / total_ticks as f32
            } else {
                0.0
            };

            let vel_val = (start_vel + (end_vel - start_vel) * progress)
                .round()
                .clamp(0.0, 127.0) as u8;

            let velocity = u7::from(vel_val);

            self.handle_active_note(0);
            self.note_on(note.key, velocity);

            self.active_note = Some(note.key);
            self.pending_ticks = note.duration;

            elapsed_ticks += note.duration;
        }
    }

    // FIXME: figure out a way to apply slurs?
    fn apply_note_context(&mut self, ctx: &NoteContext<'_>) {
        if ctx.note.underline.left {
            self.slur = true;
        }

        if ctx.note.underline.right {
            self.slur = false;
        }
    }

    fn note_on(&mut self, key: u7, vel: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
            kind: TrackEventKind::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOn { key, vel },
            },
        });
    }

    fn note_off(&mut self, note: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.pending_ticks),
            kind: TrackEventKind::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOff {
                    key: note,
                    vel: u7::from(0),
                },
            },
        });
    }

    fn get_midi_note_ticks(&self, ctx: &NoteContext<'_>) -> u32 {
        let denominator = ctx.factor as u32;
        let numerator = ctx.note.duration as u32;
        ((PPQ as u32 * numerator) / denominator) / (4 * self.params.quarter_unit)
    }

    fn get_velocity(&self, dynamic: Dynamic) -> u7 {
        match dynamic {
            Dynamic::PPP => u7::from(16),
            Dynamic::PP => u7::from(32),
            Dynamic::P => u7::from(48),
            Dynamic::MP => u7::from(64),
            Dynamic::MF => u7::from(80),
            Dynamic::F => u7::from(96),
            Dynamic::FF => u7::from(112),
            Dynamic::FFF => u7::from(127),
            _ => unreachable!("invalid velicity access"),
        }
    }

    fn get_target_velocity(&self, transition: DynamicTransition) -> u7 {
        let dynamic = match transition.kind {
            _ if transition.level == self.dynamic.current => Some(self.dynamic.current),
            DynamicTransitionKind::Cre => transition.level.get_next(),
            DynamicTransitionKind::Dec => transition.level.get_prev(),
        };

        self.get_velocity(dynamic.unwrap_or(transition.level))
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackParams {
    pub key: Key,
    pub quarter_unit: u32,
}

impl PlaybackParams {
    pub fn new(key: Key, quarter_unit: usize) -> Self {
        Self {
            key,
            quarter_unit: quarter_unit as u32,
        }
    }
}

#[derive(Debug)]
struct PendingNote {
    key: u7,
    duration: u32,
}

#[cfg(test)]
mod tests {
    use midly::num::u7;
    use voxalfa_validator::{
        ast::solfa::Note,
        data_types::{BaseKey, Key, KeyAccidental, Voice},
    };

    use crate::voice::{PlaybackParams, VoiceTask};

    struct TestTask(VoiceTask);

    impl TestTask {
        fn new(base: BaseKey, accidental: KeyAccidental, voice: Voice) -> Self {
            let params = PlaybackParams {
                key: Key { base, accidental },
                quarter_unit: 4,
            };

            Self(VoiceTask::new(voice as usize, voice, params))
        }

        fn inner(&self) -> &VoiceTask {
            &self.0
        }

        fn midi(&self, note_str: &str) -> u7 {
            self.inner()
                .get_midi_note(Note::try_from(note_str).unwrap())
                .expect("invald MIDI key value")
        }
    }

    #[test]
    fn test_nearest_octave_key_resolution() {
        // G Major: G3 (55, distance 5 to C4) is closer than G4 (67, distance 7)
        assert_eq!(
            TestTask::new(BaseKey::G, KeyAccidental::Neutral, Voice::A).midi("d"),
            u7::from(55)
        );

        // F Major: F4 (65, distance 5 to C4) is closer than F3 (53, distance 7)
        assert_eq!(
            TestTask::new(BaseKey::F, KeyAccidental::Neutral, Voice::A).midi("d"),
            u7::from(65)
        );

        // A Major: A3 (57, distance 3 to C4) is closer than A4 (69, distance 9)
        assert_eq!(
            TestTask::new(BaseKey::A, KeyAccidental::Neutral, Voice::A).midi("d"),
            u7::from(57)
        );
    }

    #[test]
    fn test_voice_octave_transposition() {
        // Base anchor for Alto in Key of C is C4 (MIDI 60)
        let alto = TestTask::new(BaseKey::C, KeyAccidental::Neutral, Voice::A);
        assert_eq!(alto.midi("d"), u7::from(60));

        // Soprano (S) should be 1 octave higher (+12 semitones) -> C5 (MIDI 72)
        let soprano = TestTask::new(BaseKey::C, KeyAccidental::Neutral, Voice::S);
        assert_eq!(soprano.midi("d"), u7::from(72));

        // Bass (B) should be 1 octave lower (-12 semitones) -> C3 (MIDI 48)
        let bass = TestTask::new(BaseKey::C, KeyAccidental::Neutral, Voice::B);
        assert_eq!(bass.midi("d"), u7::from(48));
    }

    #[test]
    fn test_voice_octave_transposition_with_nearest_key() {
        // In G Major: Alto anchor is G3 (55)
        let alto_g = TestTask::new(BaseKey::G, KeyAccidental::Neutral, Voice::A);
        assert_eq!(alto_g.midi("d"), u7::from(55));

        // Soprano in G Major -> G4 (55 + 12 = 67)
        let soprano_g = TestTask::new(BaseKey::G, KeyAccidental::Neutral, Voice::S);
        assert_eq!(soprano_g.midi("d"), u7::from(67));

        // Bass in G Major -> G2 (55 - 12 = 43)
        let bass_g = TestTask::new(BaseKey::G, KeyAccidental::Neutral, Voice::B);
        assert_eq!(bass_g.midi("d"), u7::from(43));
    }

    #[test]
    fn test_octave_shifts_with_nearest_key() {
        let task = TestTask::new(BaseKey::G, KeyAccidental::Neutral, Voice::A);

        assert_eq!(task.midi("d"), u7::from(55)); // Base anchor: G3
        assert_eq!(task.midi("d+1"), u7::from(67)); // Shift up: G4
        assert_eq!(task.midi("d-1"), u7::from(43)); // Shift down: G2
    }

    #[test]
    fn test_accidental_keys_nearest_octave() {
        // Ab Major: Ab3 (56) is closer to C4 (60) than Ab4 (68)
        let a_flat = TestTask::new(BaseKey::A, KeyAccidental::Flat, Voice::A);
        assert_eq!(a_flat.midi("d"), u7::from(56));

        // F# Major: F#4 (66) vs F#3 (54)
        let f_sharp = TestTask::new(BaseKey::F, KeyAccidental::Sharp, Voice::A);
        assert_eq!(f_sharp.midi("d"), u7::from(66));
    }

    #[test]
    fn test_scale_degrees_relative_to_nearest_do() {
        let task = TestTask::new(BaseKey::G, KeyAccidental::Neutral, Voice::A);

        assert_eq!(task.midi("d"), u7::from(55)); // Do -> G3
        assert_eq!(task.midi("r"), u7::from(57)); // Re -> A3
        assert_eq!(task.midi("m"), u7::from(59)); // Mi -> B3
        assert_eq!(task.midi("f"), u7::from(60)); // Fa -> C4
        assert_eq!(task.midi("s"), u7::from(62)); // Sol -> D4
        assert_eq!(task.midi("l"), u7::from(64)); // La -> E4
        assert_eq!(task.midi("t"), u7::from(66)); // Ti -> F#4
    }
}
