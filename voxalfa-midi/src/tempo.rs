use midly::{
    MetaMessage, Track, TrackEvent, TrackEventKind,
    num::{u24, u28},
};
use voxalfa_validator::data_types::{Tempo, TimeSignature};

#[derive(Debug, Default)]
pub struct TempoTask {
    pub track: Track<'static>,
    pub delta: u32,
}

impl TempoTask {
    pub fn new(tempo: &Tempo, time: &TimeSignature) -> Self {
        let mut task = Self::default();

        task.handle_tempo(tempo);
        task.handle_signature(time);

        task
    }

    pub fn finalize(mut self) -> Track<'static> {
        self.track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        self.track
    }

    pub fn handle_tempo(&mut self, tempo: &Tempo) {
        let tempo = self.bpm_to_uspq(tempo.bpm());

        self.track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(tempo)),
        });
    }

    pub fn handle_signature(&mut self, time: &TimeSignature) {
        self.track.push(TrackEvent {
            delta: u28::from(self.delta),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                time.top as u8,
                (time.bottom as f32).log2() as u8,
                24,
                8,
            )),
        });
    }

    pub fn handle_ticks(&mut self, ticks: u32) {
        self.delta += ticks;
    }

    fn bpm_to_uspq(&self, bpm: usize) -> u24 {
        let us_per_quarter = (60_000_000.0 / bpm as f64).round() as u32;
        u24::from(us_per_quarter & 0x00FF_FFFF)
    }
}
