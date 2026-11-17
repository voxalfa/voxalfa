pub mod error;

mod types;

use quick_xml::se::{to_string, to_string_with_root};
use voxalfa_core::{
    ast::{solfa::BaseNote, symbols::SymbolRef},
    data_types::{BaseKey, Key as KeySignature, KeyAccidental, TimeSignature, Voice},
    ir::solfa::NoteKind,
    output::{FinalOutput, voice::NoteContext},
};

use crate::{
    error::{Error, Result},
    types::{
        Attributes, Choice, Clef, ClefSign, Key, Measure, MeasureItem, MusicXmlVersion, Note, Part,
        PartList, Pitch, ScorePart, ScorePartwise, Step, Time,
    },
};

const MIDDLE_C: i8 = 60;
const MEASURE_DIVISION: u16 = 24;

const HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<!DOCTYPE score-partwise PUBLIC
    "-//Recordare//DTD MusicXML 4.0 Partwise//EN"
    "http://www.musicxml.org/dtds/partwise.dtd">"#;

#[derive(Debug)]
pub struct Converter<'a> {
    source: &'a FinalOutput,
    result: ScorePartwise,
}

impl<'a> Converter<'a> {
    pub fn new(source: &'a FinalOutput) -> Self {
        Self {
            source,
            result: ScorePartwise::default(),
        }
    }

    pub fn convert(mut self) -> Result<String> {
        let params = &self.source.header.params;

        let initial_key = *self.extract_field("key", params.key.as_ref())?;
        let initial_time = *self.extract_field("time", params.time.as_ref())?;
        let voices = self.extract_field("voice", params.voices.as_ref())?;

        for (voice_id, voice) in voices.iter().enumerate() {
            self.result.part_list.score_part.push(ScorePart {
                id: voice_id.to_string(),
                part_name: voice.value.to_string(),
            });

            let voice_line = self.source.build_voice_line(voice_id);

            let mut measures = Vec::new();
            let mut current_items = Vec::new();
            let mut current_key = initial_key;
            let mut current_time = initial_time;

            for (note_id, note) in voice_line.notes.iter().enumerate() {
                if note.measure_start && note_id > 0 {
                    measures.push(Measure {
                        number: measures.len() + 1,
                        implicit: Choice::No,
                        items: std::mem::take(&mut current_items),
                    });
                }

                if note_id == 0 || note.measure_start {
                    current_items.push(MeasureItem::Attributes(Attributes {
                        divisions: MEASURE_DIVISION,
                        key: Key {
                            fifths: get_fifths(current_key),
                        },
                        time: Time {
                            beats: current_time.top,
                            beat_type: current_time.bottom,
                        },
                        clef: Clef {
                            sign: if matches!(voice.value, Voice::B) {
                                ClefSign::F
                            } else {
                                ClefSign::G
                            },
                        },
                    }));
                }

                current_items.push(MeasureItem::Note(resolve_note(
                    note,
                    voice.value,
                    current_key,
                    current_time,
                )));
            }

            if !current_items.is_empty() {
                measures.push(Measure {
                    number: measures.len() + 1,
                    implicit: Choice::No,
                    items: std::mem::take(&mut current_items),
                });
            }

            self.result.part.push(Part {
                id: voice_id.to_string(),
                measure: measures,
            });
        }

        let serialized = to_string_with_root("score-partwise", &self.result)?;
        let xml = format!("{HEADER}\n{serialized}");

        Ok(xml)
    }

    fn extract_field<T>(
        &self,
        name: &'static str,
        param: Option<&'a SymbolRef<T>>,
    ) -> Result<&'a T> {
        param
            .map(|p| &p.value)
            .ok_or(Error::MissingHeaderField(name))
    }
}

fn get_fifths(key: KeySignature) -> i8 {
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
        _ => panic!("unsupported key: {key}"),
    }
}

fn resolve_note(ctx: &NoteContext, voice: Voice, key: KeySignature, time: TimeSignature) -> Note {
    let note = match ctx.column.note {
        NoteKind::Note(note) => note,
        NoteKind::ProlongedNote | NoteKind::EmptyNote => todo!("{ctx:?}"),
    };

    let duration = ((MEASURE_DIVISION * ctx.column.duration as u16) / ctx.factor as u16)
        / (4 / time.bottom as u16);

    let chromatic = MIDDLE_C + note.offset() + key.offset() + voice.offset();
    let octave = (chromatic / 12) - 1;

    let pitch = Pitch {
        step: resolve_step(key.base, note.base),
        alter: 0, // TODO
        octave: octave as u8,
    };

    Note { pitch, duration }
}

fn resolve_step(base_key: BaseKey, note_base: BaseNote) -> Step {
    let key_index = match base_key {
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
