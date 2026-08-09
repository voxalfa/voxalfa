use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u7, u24, u28},
};
use voxalfa_core::{
    ast::solfa::Note,
    data_types::{Dynamic, Touch, Voice},
    ir::solfa::NoteKind,
    output::{
        dynamics::{DynamicTransition, DynamicTransitionKind},
        evaluator::{PlaybackParams, TimelineEvaluator},
        event::NoteTimeline,
        tempo::TempoTransition,
        voice::{NoteContext, VoiceLine},
    },
};

use crate::{
    BASE_MIDI_KEY, MAX_PAUSE, PPQN,
    error::{ConvertError, Result},
};

#[derive(Debug)]
pub struct TaskResult {
    pub voice_track: Track<'static>,
    pub meta_track: Track<'static>,
    pub ticks: u32,
}

#[derive(Debug)]
pub struct ConverterTask {
    channel: u4,
    voice: Voice,
    track: Track<'static>,
    meta_track: Track<'static>,
    active_note: Option<u7>,
    play_ticks: u32,
    rest_ticks: u32,
    total_ticks: u32,
    meta_ticks: u32,
    pending_notes: Vec<PendingNote>,
    tempo_start_delta: Option<u32>,
    context: TimelineEvaluator,
    saved_params: PlaybackParams,
    slur: bool,
}

impl ConverterTask {
    pub fn new(id: usize, voice: Voice, context: TimelineEvaluator) -> Self {
        let saved_params = context.params.clone();

        Self {
            voice,
            context,
            saved_params,
            channel: u4::from(id as u8),
            track: Track::new(),
            meta_track: Track::new(),
            active_note: None,
            play_ticks: 0,
            rest_ticks: 0,
            total_ticks: 0,
            meta_ticks: 0,
            pending_notes: Vec::new(),
            tempo_start_delta: None,
            slur: false,
        }
    }

    pub fn process(mut self, voice_line: &VoiceLine) -> Result<TaskResult> {
        self.init_meta_track();

        while let Some(ctx) = voice_line.notes.get(self.context.index()) {
            self.handle_events(&voice_line.timeline);

            if self.context.done() {
                break;
            }

            if self.context.jump() {
                continue;
            }

            if !self.context.is_waiting() {
                match ctx.column.note {
                    NoteKind::Note(note) => self.handle_note(note, ctx, voice_line)?,
                    NoteKind::EmptyNote => self.handle_pause(ctx),
                    NoteKind::ProlongedNote => self.prolongate(ctx),
                }

                self.handle_meta_events();
            }

            self.context.step();

            if self.context.index() >= voice_line.notes.len() {
                self.handle_pending_events(&voice_line.timeline);
            }
        }

        Ok(self.finalize())
    }

    fn handle_note(
        &mut self,
        note: Note,
        ctx: &NoteContext<'_>,
        voice_line: &VoiceLine,
    ) -> Result<()> {
        let touch = self.context.take_pending_touch();

        // avoid micro-pauses when followed by rest
        let micro_pause = voice_line
            .notes
            .get(self.context.index() + 1)
            .map(|ctx| !ctx.column.is_empty())
            .unwrap_or(true);

        if self.context.dynamic.transition.is_some() {
            let key = self.get_midi_note(note)?;
            let duration = self.get_midi_note_ticks(ctx);

            self.pending_notes.push(PendingNote {
                key,
                duration,
                touch,
                micro_pause,
            });
        } else {
            self.apply_note_context(ctx);
            self.handle_active_note();

            let raw_duration = self.get_midi_note_ticks(ctx);
            let (play_ticks, rest_ticks) = self.get_touch_ticks(raw_duration, touch, micro_pause);

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

    fn init_meta_track(&mut self) {
        self.update_tempo();
        self.update_time();
    }

    fn push_meta_message(&mut self, kind: MetaMessage<'static>) {
        let delta = u28::from(self.total_ticks - self.meta_ticks);

        self.meta_track.push(TrackEvent {
            delta,
            kind: TrackEventKind::Meta(kind),
        });

        self.meta_ticks = self.total_ticks;
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

    fn finalize(mut self) -> TaskResult {
        self.handle_active_note();

        self.track.push(TrackEvent {
            delta: u28::from(self.play_ticks),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        self.push_meta_message(MetaMessage::EndOfTrack);

        TaskResult {
            voice_track: self.track,
            meta_track: self.meta_track,
            ticks: self.total_ticks,
        }
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
            self.total_ticks += self.play_ticks + self.rest_ticks;
            self.play_ticks = self.rest_ticks;
            self.rest_ticks = 0;
        } else {
            self.total_ticks += self.rest_ticks;
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
            let (play_ticks, rest_ticks) =
                self.get_touch_ticks(note.duration, note.touch, note.micro_pause);

            self.handle_active_note();
            self.note_on(note.key, velocity);

            self.active_note = Some(note.key);
            self.play_ticks = play_ticks;
            self.rest_ticks = rest_ticks;

            elapsed_ticks += note.duration;
        }
    }

    fn handle_meta_events(&mut self) {
        if self.saved_params.time != self.context.params.time {
            self.update_time();
        }

        if self.saved_params.tempo != self.context.params.tempo {
            self.update_tempo();
        }

        if let Some(transition) = self.context.poll_tempo_update() {
            self.emit_progressive_tempo(transition);
        } else if self.context.tempo.transition.is_some() && self.tempo_start_delta.is_none() {
            self.tempo_start_delta = Some(self.total_ticks - self.meta_ticks);
        }
    }

    fn emit_progressive_tempo(&mut self, transition: TempoTransition) {
        let start_delta = self.tempo_start_delta.take().unwrap_or_default();
        let start_bpm = transition.initial_value as f32;
        let target_bpm = start_bpm * transition.kind.ratio();

        let start_tick = self.meta_ticks + start_delta;
        let end_tick = self.total_ticks;
        let duration = end_tick.saturating_sub(start_tick);
        let steps = duration / (PPQN as u32 / 4); // change tempo every 1/16

        for i in 1..=steps {
            let progress = i as f32 / steps as f32;
            let current_bpm = start_bpm + (target_bpm - start_bpm) * progress;
            let current_target_tick = start_tick + (duration * i) / steps;

            let uspq = self.bpm_to_uspq(current_bpm.round() as u16);
            let delta = current_target_tick - self.meta_ticks;

            self.meta_track.push(TrackEvent {
                delta: u28::from(delta),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(uspq)),
            });

            self.meta_ticks = current_target_tick;
        }

        self.saved_params.tempo = self.context.params.tempo;
    }

    fn apply_note_context(&mut self, ctx: &NoteContext<'_>) {
        if ctx.column.underline.left {
            self.slur = true;
        }

        if ctx.column.underline.right {
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

    fn update_tempo(&mut self) {
        let bpm = self.context.params.tempo.bpm();
        let tempo = self.bpm_to_uspq(bpm);

        self.push_meta_message(MetaMessage::Tempo(tempo));
        self.saved_params.tempo = self.context.params.tempo;
    }

    fn update_time(&mut self) {
        let time = self.context.params.time;
        let denominator = (time.bottom as f32).log2() as u8;
        let message = MetaMessage::TimeSignature(time.top, denominator, 24, 8);

        self.push_meta_message(message);
        self.saved_params.time = self.context.params.time;
    }

    fn get_midi_note_ticks(&self, ctx: &NoteContext<'_>) -> u32 {
        let denominator = ctx.factor as u32;
        let numerator = ctx.column.duration as u32;
        let quarter_unit = self.context.params.time.bottom as u32;

        ((PPQN as u32 * numerator) / denominator) / (4 / quarter_unit)
    }

    fn get_touch_ticks(
        &self,
        duration: u32,
        touch: Option<Touch>,
        micro_pause: bool,
    ) -> (u32, u32) {
        match touch {
            Some(Touch::Staccato) => {
                let play_ticks = duration / 2;
                let rest_ticks = duration - play_ticks;
                (play_ticks, rest_ticks)
            }
            Some(Touch::Fermata) => (duration + (duration / 2), duration / 5),
            _ if self.slur || !micro_pause => (duration, 0),
            _ => {
                // apply micro-pauses for a less robotic result
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

    fn bpm_to_uspq(&self, bpm: u16) -> u24 {
        let us_per_quarter = (60_000_000.0 / bpm as f64).round() as u32;
        u24::from(us_per_quarter & 0x00FF_FFFF)
    }
}

#[derive(Debug)]
struct PendingNote {
    key: u7,
    duration: u32,
    touch: Option<Touch>,
    micro_pause: bool,
}
