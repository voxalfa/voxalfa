use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u28},
};
use voxalfa_validator::{
    ast::solfa::Note,
    data_types::{Dynamic, Touch, Voice},
    ir::solfa::PulseColumnKind,
    output::{
        dynamics::{DynamicTransition, DynamicTransitionKind},
        evaluator::TimelineEvaluator,
        event::NoteTimeline,
        voice::{NoteContext, VoiceLine},
    },
};

use crate::{
    BASE_MIDI_KEY, MAX_PAUSE, PPQN,
    error::{ConvertError, Result},
};

#[derive(Debug)]
pub struct VoiceTask {
    channel: u4,
    voice: Voice,
    track: Track<'static>,
    active_note: Option<u7>,
    play_ticks: u32,
    rest_ticks: u32,
    pending_notes: Vec<PendingNote>,
    slur: bool,
    context: TimelineEvaluator,
}

impl VoiceTask {
    pub fn new(id: usize, voice: Voice, context: TimelineEvaluator) -> Self {
        Self {
            voice,
            channel: u4::from(id as u8),
            track: Track::new(),
            active_note: None,
            play_ticks: 0,
            rest_ticks: 0,
            pending_notes: Vec::new(),
            slur: false,
            context,
        }
    }

    pub fn process(mut self, voice_line: &VoiceLine) -> Result<Track<'static>> {
        while let Some(ctx) = voice_line.notes.get(self.context.index()) {
            self.handle_events(&voice_line.timeline);

            if self.context.done() {
                break;
            }

            if self.context.jump() {
                continue;
            }

            if !self.context.is_waiting() {
                match ctx.note.kind {
                    PulseColumnKind::Note(note) => self.handle_note(note, ctx)?,
                    PulseColumnKind::EmptyNote => self.handle_pause(ctx),
                    PulseColumnKind::ProlongedNote(_) => self.prolongate(ctx),
                }
            }

            self.context.step();

            if self.context.index() >= voice_line.notes.len() {
                self.handle_pending_events(&voice_line.timeline);
            }
        }

        Ok(self.finalize())
    }

    fn handle_note(&mut self, note: Note, ctx: &NoteContext<'_>) -> Result<()> {
        let touch = self.context.take_pedning_touch();

        if self.context.dynamic.transition.is_some() {
            let key = self.get_midi_note(note)?;
            let duration = self.get_midi_note_ticks(ctx);

            self.pending_notes.push(PendingNote {
                key,
                duration,
                touch,
            });
        } else {
            self.apply_note_context(ctx);
            self.handle_active_note();

            let raw_duration = self.get_midi_note_ticks(ctx);
            let (play_ticks, rest_ticks) = self.get_touch_ticks(raw_duration, touch);

            let midi_note = self.get_midi_note(note)?;
            let mut velocity = self.get_velocity(self.context.dynamic.current);

            if touch == Some(Touch::Accent) {
                velocity = u8::max(velocity.as_int() + 20, 127).into();
            }

            self.note_on(midi_note, velocity);

            self.active_note = Some(midi_note);
            self.play_ticks = play_ticks;
            self.rest_ticks = rest_ticks;
        }

        Ok(())
    }

    fn handle_pause(&mut self, ctx: &NoteContext<'_>) {
        self.apply_note_context(ctx);
        self.handle_active_note();

        self.rest_ticks += self.get_midi_note_ticks(ctx);
    }

    fn prolongate(&mut self, ctx: &NoteContext<'_>) {
        let ticks = self.get_midi_note_ticks(ctx);

        if self.context.dynamic.transition.is_some()
            && let Some(last) = self.pending_notes.last_mut()
        {
            last.duration += ticks;
        } else {
            self.apply_note_context(ctx);
            self.play_ticks += ticks;
        }
    }

    fn finalize(mut self) -> Track<'static> {
        self.handle_active_note();

        self.track.push(TrackEvent {
            delta: u28::from(self.play_ticks),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        self.track
    }

    fn handle_events(&mut self, timeline: &NoteTimeline) {
        self.context.handle_events(timeline);

        if let Some(transition) = self.context.poll_dynamic_update() {
            self.handle_pending_notes(transition);
        }
    }

    fn handle_pending_events(&mut self, timeline: &NoteTimeline) {
        self.handle_events(timeline);
        self.context.jump();
    }

    fn handle_active_note(&mut self) {
        if let Some(last_note) = self.active_note.take() {
            self.note_off(last_note);
            self.play_ticks = self.rest_ticks;
            self.rest_ticks = 0;
        } else {
            self.play_ticks += self.rest_ticks;
            self.rest_ticks = 0;
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
            let (play_ticks, rest_ticks) = self.get_touch_ticks(note.duration, note.touch);

            self.handle_active_note();
            self.note_on(note.key, velocity);

            self.active_note = Some(note.key);
            self.play_ticks = play_ticks;
            self.rest_ticks = rest_ticks;

            elapsed_ticks += note.duration;
        }
    }

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
            delta: u28::from(self.play_ticks),
            kind: TrackEventKind::Midi {
                channel: self.channel,
                message: MidiMessage::NoteOn { key, vel },
            },
        });
    }

    fn note_off(&mut self, note: u7) {
        self.track.push(TrackEvent {
            delta: u28::from(self.play_ticks),
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
        ((PPQN as u32 * numerator) / denominator) / (4 / self.context.params.quarter_unit)
    }

    fn get_touch_ticks(&self, duration: u32, touch: Option<Touch>) -> (u32, u32) {
        match touch {
            Some(Touch::Staccato) => {
                let play_ticks = duration / 2;
                let rest_ticks = duration - play_ticks;
                (play_ticks, rest_ticks)
            }
            Some(Touch::Fermata) => (duration + (duration / 2), 0),
            _ if self.slur => (duration, 0),
            _ => {
                let rest_ticks = (duration / 10).min(MAX_PAUSE);
                let play_ticks = duration - rest_ticks;

                (play_ticks, rest_ticks)
            }
        }
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
            _ if transition.level == self.context.dynamic.current => {
                Some(self.context.dynamic.current)
            }
            DynamicTransitionKind::Cre => transition.level.get_next(),
            DynamicTransitionKind::Dec => transition.level.get_prev(),
        };

        self.get_velocity(dynamic.unwrap_or(transition.level))
    }

    fn get_midi_note(&self, note: Note) -> Result<u7> {
        let result =
            BASE_MIDI_KEY + self.context.params.key.offset() + note.offset() + self.voice.offset();

        if !(0..=127).contains(&result) {
            Err(ConvertError::InvalidMidiKey(result))
        } else {
            Ok(u7::from(result as u8))
        }
    }
}

#[derive(Debug)]
struct PendingNote {
    key: u7,
    duration: u32,
    touch: Option<Touch>,
}
