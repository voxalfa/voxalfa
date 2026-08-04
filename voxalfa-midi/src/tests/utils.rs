use midly::{MidiMessage, Track, TrackEventKind, num::u7};
use voxalfa_validator::MultiStepValidator;

use crate::Converter;

pub fn run_snapshot(source_name: &str, content: &str) {
    let mut validator = MultiStepValidator::init().unwrap();
    let output = validator.analyze(content);

    assert!(!output.has_error(), "{:?}", output.diagnostics);

    let converter = Converter::new(&output);
    let smf = converter.convert().unwrap();

    let output = format!(
        "FILE: {}\n\n=== SOURCE ===\n{}\n\n=== MIDI TRACKS ===\n{}",
        source_name,
        content.trim(),
        format_midi_tracks(&smf.tracks)
    );

    insta::assert_snapshot!(source_name, output);
}

fn midi_key_to_name(key: u7) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (u8::from(key) / 12) as i8 - 1;
    let note = names[(u8::from(key) % 12) as usize];
    format!("{}{}", note, octave)
}

pub fn format_midi_tracks(tracks: &[Track<'static>]) -> String {
    let mut output = String::new();

    for (track_idx, track) in tracks.iter().enumerate() {
        if track_idx > 0 {
            output.push('\n');
        }

        let track_label = if track_idx == 0 {
            "Track 0 [Tempo / Meta]".to_string()
        } else {
            format!("Track {track_idx}")
        };

        output.push_str(&format!("--- {track_label} ---\n"));

        let mut total_ticks: u64 = 0;

        for (event_idx, event) in track.iter().enumerate() {
            let delta = event.delta.as_int() as u64;
            total_ticks += delta;

            match &event.kind {
                TrackEventKind::Midi { channel, message } => match message {
                    MidiMessage::NoteOn { key, vel } => {
                        let midi_key = midi_key_to_name(*key);

                        output.push_str(&format!(
                            "[{:05} t / +{:03} t] Event {:02} | Ch {:02} | NoteOn  key: {:03} ({}) vel: {:03}\n",
                            total_ticks, delta, event_idx, channel, key, midi_key, vel
                        ));
                    }
                    MidiMessage::NoteOff { key, vel } => {
                        let midi_key = midi_key_to_name(*key);

                        output.push_str(&format!(
                            "[{:05} t / +{:03} t] Event {:02} | Ch {:02} | NoteOff key: {:03} ({}) vel: {:03}\n",
                            total_ticks, delta, event_idx, channel, key, midi_key, vel
                        ));
                    }
                    _ => {
                        output.push_str(&format!(
                            "[{:05} t / +{:03} t] Event {:02} | Ch {:02} | MidiOther: {:?}\n",
                            total_ticks, delta, event_idx, channel, message
                        ));
                    }
                },
                TrackEventKind::Meta(meta) => {
                    output.push_str(&format!(
                        "[{:05} t / +{:03} t] Event {:02} | Meta: {:?}\n",
                        total_ticks, delta, event_idx, meta
                    ));
                }
                _ => {}
            }
        }
    }

    output
}
