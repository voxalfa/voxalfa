use voxalfa_core::{
    ast::solfa::{BaseNote, Note as CoreNote},
    data_types::{
        BaseKey, Dynamic as CoreDynamics, Jump, Key as KeySignature, KeyAccidental, Mark,
        StaticTempo, TimeSignature, Touch, Voice,
    },
    ir::solfa::NoteKind,
    output::{
        event::{Event, EventKind, JumpEvent},
        voice::VoiceLine,
    },
};

use crate::types::*;

const MIDDLE_C: i8 = 60;
const MEASURE_DIVISION: u16 = 480;

pub struct MeasureTask {
    voice: Voice,
    key: KeySignature,
    time: TimeSignature,
    items: Vec<MeasureItem>,
    pending_time: Option<TimeSignature>,
    pending_wedge: bool,
    pending_notation: Option<Notations>,
    result: Vec<Measure>,
    last_note: Option<CoreNote>,
}

impl MeasureTask {
    pub fn new(voice: Voice, initial_key: KeySignature, initial_time: TimeSignature) -> Self {
        Self {
            voice,
            key: initial_key,
            time: initial_time,
            last_note: None,
            pending_time: None,
            pending_wedge: false,
            pending_notation: None,
            items: Vec::new(),
            result: Vec::new(),
        }
    }

    pub fn process(mut self, voice_line: VoiceLine<'_>) -> Vec<Measure> {
        for (note_id, note) in voice_line.notes.iter().enumerate() {
            let events = voice_line.timeline.get_events(note_id);

            if note_id == 0 {
                self.set_initial_attributes();
            }

            if note.measure_start && note_id > 0 {
                self.result.push(Measure {
                    number: self.result.len() + 1,
                    implicit: Choice::No,
                    items: std::mem::take(&mut self.items),
                });
            }

            self.handle_events(events);

            let duration = ((MEASURE_DIVISION * note.column.duration as u16) / note.factor as u16)
                / (4 / self.time.bottom as u16);

            match note.column.note {
                NoteKind::Note(note) => self.handle_note(note, duration, false),
                NoteKind::EmptyNote => self.handle_empty_note(duration),
                NoteKind::ProlongedNote => self.handle_prolonged_note(duration),
            }

            self.handle_pending_updates();

            if note_id + 1 == voice_line.notes.len() {
                self.handle_events(voice_line.timeline.get_events(note_id + 1));
            }
        }

        if !self.items.is_empty() {
            self.result.push(Measure {
                number: self.result.len() + 1,
                implicit: Choice::No,
                items: std::mem::take(&mut self.items),
            });
        }

        self.result
    }

    fn set_initial_attributes(&mut self) {
        self.items.push(MeasureItem::Attributes(Attributes {
            divisions: Some(MEASURE_DIVISION),
            key: Some(Key {
                fifths: resolve_fifths(self.key),
            }),
            time: Some(Time {
                beats: self.time.top,
                beat_type: self.time.bottom,
            }),
            clef: Some(Clef::from_voice(self.voice)),
        }));
    }

    fn handle_events<'a>(&'a mut self, events: impl Iterator<Item = &'a Event>) {
        for event in events {
            match event.kind {
                EventKind::Key(key) => self.handle_key_change(key),
                EventKind::Jump(jump) => self.handle_jump_event(jump),
                EventKind::Mark(mark) => self.handle_mark_event(mark),
                EventKind::Dynamic(dynamic) => self.handle_dynamics_event(dynamic),
                EventKind::Tempo(tempo) => self.handle_tempo_event(tempo),
                EventKind::TimeSignature(time) => self.pending_time = Some(time),
                EventKind::Touch(touch) => self.handle_touch(touch),
                // EventKind::EndingStart(_) => todo!(),
                // EventKind::EndingEnd(_) => todo!(),
                // EventKind::TempoStart(progressive_tempo) => todo!(),
                // EventKind::TempoEnd(progressive_tempo) => todo!(),
                _ => {}
            }
        }
    }

    fn handle_key_change(&mut self, key: KeySignature) {
        self.items.push(MeasureItem::Attributes(Attributes {
            key: Some(Key {
                fifths: resolve_fifths(key),
            }),
            ..Default::default()
        }));

        self.key = key;
    }

    fn handle_jump_event(&mut self, jump: JumpEvent) {
        let direction = match jump.kind {
            Jump::DS => Direction::jump("D.S.", Sound::default().dalsegno()),
            Jump::DC => Direction::jump("D.C.", Sound::default().dacapo()),
            Jump::DSC => Direction::jump("D.S. al Coda", Sound::default().dalsegno().tocoda()),
            Jump::DSF => Direction::jump("D.S. al Fine", Sound::default().dalsegno().fine()),
            Jump::DCC => Direction::jump("D.C. al Coda", Sound::default().dacapo().tocoda()),
            Jump::DCF => Direction::jump("D.C. al Fine", Sound::default().dacapo().fine()),
        };

        self.items.push(MeasureItem::Direction(direction));
    }

    fn handle_mark_event(&mut self, mark: Mark) {
        let direction = match mark {
            Mark::ToCoda => {
                Direction::mark(DirectionType::words("To Coda"), Sound::default().tocoda())
            }
            Mark::Segno => Direction::mark(DirectionType::segno(), Sound::default().segno()),
            Mark::Coda => Direction::mark(DirectionType::coda(), Sound::default().coda()),
            Mark::Fine => Direction::mark(DirectionType::words("Fine"), Sound::default().fine()),
        };

        self.items.push(MeasureItem::Direction(direction));
    }

    fn handle_dynamics_event(&mut self, dynamics: CoreDynamics) {
        if matches!(dynamics, CoreDynamics::Cre | CoreDynamics::Dec) {
            let wedge_type = match dynamics {
                _ if self.pending_wedge => WedgeType::Stop,
                CoreDynamics::Cre => WedgeType::Crescendo,
                _ => WedgeType::Decrescendo,
            };

            let direction = Direction {
                direction_type: vec![DirectionType {
                    wedge: Some(Wedge { r#type: wedge_type }),
                    ..Default::default()
                }],
                ..Default::default()
            };

            self.items.push(MeasureItem::Direction(direction));
            self.pending_wedge = !self.pending_wedge;
        } else {
            let dynamics = Dynamics::from(dynamics);
            let velocity = dynamics.velocity();

            let direction = Direction {
                direction_type: vec![DirectionType::dynamics(dynamics)],
                sound: Some(Sound {
                    dynamics: Some(velocity),
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.items.push(MeasureItem::Direction(direction));
        }
    }

    fn handle_touch(&mut self, touch: Touch) {
        let notations = self.pending_notation.get_or_insert_default();

        if touch == Touch::Fermata {
            notations.fermata = Some(());
        } else {
            notations.articulations = Some(Articulations::from(touch).into());
        }
    }

    fn handle_pending_updates(&mut self) {
        if let Some(time) = self.pending_time.take() {
            self.items.push(MeasureItem::Attributes(Attributes {
                time: Some(Time {
                    beats: time.top,
                    beat_type: time.bottom,
                }),
                ..Default::default()
            }));

            self.time = time;
        }
    }

    fn handle_tempo_event(&mut self, tempo: StaticTempo) {
        let direction_type = match tempo {
            StaticTempo::Custom(bpm) => DirectionType {
                metronome: Some(Metronome {
                    beat_unit: NoteType::from_factor(self.time.bottom.into()),
                    per_minute: bpm.to_string(),
                }),
                ..Default::default()
            },
            named_tempo => DirectionType {
                words: Some(Words {
                    content: named_tempo.to_string(),
                    font_style: Some(FontStyle::Italic),
                }),
                ..Default::default()
            },
        };

        self.items.push(MeasureItem::Direction(Direction {
            direction_type: vec![direction_type],
            sound: Some(Sound {
                tempo: Some(tempo.bpm().into()),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }

    fn handle_note(&mut self, note: CoreNote, duration: u16, tied: bool) {
        let semitones = MIDDLE_C + note.offset() + self.key.offset() + self.voice.offset();
        let octave = (semitones / 12) - 1;
        let step = self.resolve_step(note.base);
        let natural_semitones = step.semitone_offset() as i8 + (octave + 1) * 12;
        let alter = semitones - natural_semitones;

        let pitch = Pitch {
            step,
            alter,
            octave: octave as u8,
        };

        let mut note_item = Note {
            pitch: Some(pitch),
            duration,
            ..Default::default()
        };

        note_item.notations = self.pending_notation.take();

        if tied {
            note_item.add_tie(Position::Stop);
        }

        self.items.push(MeasureItem::Note(note_item));
        self.last_note = Some(note);
    }

    fn handle_empty_note(&mut self, duration: u16) {
        let last_note = self.items.last_mut().and_then(|i| match i {
            MeasureItem::Note(note) if note.rest.is_some() => Some(note),
            _ => None,
        });

        if let Some(note) = last_note {
            note.duration += duration;
        } else {
            self.items.push(MeasureItem::Note(Note {
                rest: Some(Rest {}),
                duration,
                ..Default::default()
            }));
        }
    }

    fn handle_prolonged_note(&mut self, duration: u16) {
        let last_note = self.items.iter_mut().rev().find_map(|i| match i {
            MeasureItem::Note(note) => Some(note),
            _ => None,
        });

        if let Some(note) = last_note {
            note.duration += duration;
        } else if let Some(last_measure) = self.result.last_mut() {
            let prev_note = last_measure.items.iter_mut().rev().find_map(|i| match i {
                MeasureItem::Note(note) => Some(note),
                _ => None,
            });

            if let Some((prev_note, source)) = prev_note.zip(self.last_note) {
                prev_note.add_tie(Position::Start);
                self.handle_note(source, duration, true);
            }
        }
    }

    fn resolve_step(&self, note_base: BaseNote) -> Step {
        let key_index = match self.key.base {
            BaseKey::C => 0,
            BaseKey::D => 1,
            BaseKey::E => 2,
            BaseKey::F => 3,
            BaseKey::G => 4,
            BaseKey::A => 5,
            BaseKey::B => 6,
        };

        let note_index = match note_base {
            BaseNote::D => 0,
            BaseNote::R => 1,
            BaseNote::M => 2,
            BaseNote::F => 3,
            BaseNote::S => 4,
            BaseNote::L => 5,
            BaseNote::T => 6,
        };

        Step::from(key_index + note_index)
    }
}

fn resolve_fifths(key: KeySignature) -> i8 {
    match (key.base, key.accidental) {
        (BaseKey::C, KeyAccidental::Flat) => -7,
        (BaseKey::G, KeyAccidental::Flat) => -6,
        (BaseKey::D, KeyAccidental::Flat) => -5,
        (BaseKey::A, KeyAccidental::Flat) => -4,
        (BaseKey::E, KeyAccidental::Flat) => -3,
        (BaseKey::B, KeyAccidental::Flat) => -2,
        (BaseKey::F, KeyAccidental::Neutral) => -1,
        (BaseKey::C, KeyAccidental::Neutral) => 0,
        (BaseKey::G, KeyAccidental::Neutral) => 1,
        (BaseKey::D, KeyAccidental::Neutral) => 2,
        (BaseKey::A, KeyAccidental::Neutral) => 3,
        (BaseKey::E, KeyAccidental::Neutral) => 4,
        (BaseKey::B, KeyAccidental::Neutral) => 5,
        (BaseKey::F, KeyAccidental::Sharp) => 6,
        (BaseKey::C, KeyAccidental::Sharp) => 7,
        _ => panic!("unsupported key: {}", key),
    }
}
