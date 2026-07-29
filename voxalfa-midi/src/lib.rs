use std::path::PathBuf;

use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u15, u24, u28},
};
use voxalfa_validator::{
    ast::solfa::Note, data_types::Voice, ir::solfa::PulseColumnKind, output::FinalOutput,
};

pub const PPQ: u16 = 480;
pub const BASE_MIDI_KEY: i8 = 60; // middle C

#[allow(unused)]
#[derive(Debug)]
pub struct Converter<'a> {
    source: &'a FinalOutput,
    key_offset: i8,
}

impl<'a> Converter<'a> {
    pub fn new(source: &'a FinalOutput) -> Self {
        Self {
            source,
            key_offset: 0,
        }
    }

    pub fn convert(&mut self, output_path: PathBuf) {
        let mut smf = Smf::new(Header::new(
            Format::Parallel,
            Timing::Metrical(u15::from(PPQ)),
        ));

        let key_def = &self.source.header.params.key;
        let voices_def = &self.source.header.metadata.voices;
        let time_def = &self.source.header.params.time;
        let tempo_def = &self.source.header.params.tempo;

        let Some(key) = key_def else { return };
        let Some(voices) = voices_def else { return };
        let Some(time) = time_def else { return };
        let Some(tempo) = tempo_def else { return };

        self.key_offset = key.value.base.offset();

        let bpm = tempo.value.bpm();
        let numerator = time.value.top;
        let denominator = time.value.bottom;
        let tempo_track = self.create_tempo_track(bpm, numerator, denominator);

        smf.tracks.push(tempo_track);

        // Process each voice into a separate track with its own channel
        for (idx, voice) in voices.value.iter().enumerate() {
            let channel = u4::from((idx % 16) as u8);
            let track = self.process_voice(voice.value, channel);
            smf.tracks.push(track);
        }

        if let Err(err) = smf.save(output_path) {
            eprintln!("Failed to save MIDI file: {err}");
        }
    }

    fn process_voice(&mut self, voice: Voice, channel: u4) -> Track<'static> {
        let mut track = Track::new();

        let mut active_note: Option<u7> = None;
        let mut pending_ticks: u32 = 0;

        for section in self.source.build_voice_sections(voice) {
            for pulse in &section.solfa.pulses {
                let denominator = pulse.factor as u32;

                for column in &pulse.columns {
                    let numerator = column.duration as u32;
                    let ticks = self.calculate_ticks(numerator, denominator);

                    match column.kind {
                        PulseColumnKind::Note(note) => {
                            // 1. Turn off previous note if one was playing
                            if let Some(prev_note) = active_note.take() {
                                track.push(TrackEvent {
                                    delta: u28::from(pending_ticks as u32),
                                    kind: TrackEventKind::Midi {
                                        channel,
                                        message: MidiMessage::NoteOff {
                                            key: prev_note,
                                            vel: u7::from(0),
                                        },
                                    },
                                });
                                pending_ticks = 0;
                            }

                            // 2. Start the new note
                            let midi_note = self.get_midi_note(voice, note);
                            track.push(TrackEvent {
                                delta: u28::from(pending_ticks as u32),
                                kind: TrackEventKind::Midi {
                                    channel,
                                    message: MidiMessage::NoteOn {
                                        key: midi_note,
                                        vel: u7::from(100), // Default velocity
                                    },
                                },
                            });

                            active_note = Some(midi_note);
                            pending_ticks = ticks;
                        }

                        PulseColumnKind::ProlongedNote(_note) => {
                            // Extend the duration of the current active note
                            pending_ticks += ticks;
                        }

                        PulseColumnKind::EmptyNote => {
                            // Turn off active note when reaching a rest
                            if let Some(prev_note) = active_note.take() {
                                track.push(TrackEvent {
                                    delta: u28::from(pending_ticks as u32),
                                    kind: TrackEventKind::Midi {
                                        channel,
                                        message: MidiMessage::NoteOff {
                                            key: prev_note,
                                            vel: u7::from(0),
                                        },
                                    },
                                });
                                pending_ticks = ticks;
                            } else {
                                pending_ticks += ticks;
                            }
                        }
                    }
                }
            }
        }

        // Close last active note at end of track
        if let Some(last_note) = active_note.take() {
            track.push(TrackEvent {
                delta: u28::from(pending_ticks as u32),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOff {
                        key: last_note,
                        vel: u7::from(0),
                    },
                },
            });
            pending_ticks = 0;
        }

        // End track marker
        track.push(TrackEvent {
            delta: u28::from(pending_ticks as u32),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        track
    }

    /// Converts a fractional duration (num/denom) into MIDI ticks
    fn calculate_ticks(&self, numerator: u32, denominator: u32) -> u32 {
        if denominator == 0 {
            return 0;
        }
        // Formula: PPQ * 4 * (numerator / denominator)
        (PPQ as u32 * numerator) / denominator
    }

    fn create_tempo_track(
        &self,
        bpm: usize,
        numerator: usize,
        denominator: usize,
    ) -> Track<'static> {
        let mut track = Track::new();

        let denom_exponent = (denominator as f32).log2() as u8;
        let tempo = self.bpm_to_uspq(bpm);

        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                numerator as u8,
                denom_exponent,
                24,
                8,
            )),
        });

        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(tempo)),
        });

        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        track
    }

    fn get_midi_note(&self, voice: Voice, note: Note) -> u7 {
        let result = BASE_MIDI_KEY + self.key_offset + note.offset() + 12 * voice.octave_offset();

        if !(0..=127).contains(&result) {
            panic!("invalid MIDI KEY: {result}");
        }

        u7::from(result as u8)
    }

    fn bpm_to_uspq(&self, bpm: usize) -> u24 {
        let us_per_quarter = (60_000_000.0 / bpm as f64).round() as u32;
        u24::from((us_per_quarter & 0x00FF_FFFF) as u32)
    }
}
