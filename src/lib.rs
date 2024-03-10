use std::iter;
use std::pin::Pin;

use tokio_stream::Stream;

pub fn noise() -> Pin<Box<dyn Stream<Item = f32>>> {
    Box::pin(tokio_stream::iter(iter::from_fn(|| {
        Some(rand::random::<f32>())
    })))
}

pub fn tone(freq: &u32, rate: &u32, amplitude: &f32) -> Pin<Box<dyn Stream<Item = f32>>> {
    Box::pin(tokio_stream::iter(ToneIter::new(*freq, *rate, *amplitude)))
}

struct ToneIter {
    sample_period: u32,
    curr_phase: u32,
    amplitude: f32,
}

impl ToneIter {
    pub fn new(freq: u32, rate: u32, amplitude: f32) -> Self {
        let sample_period = rate / freq;
        Self {
            sample_period,
            amplitude,
            curr_phase: 0,
        }
    }
}

impl Iterator for ToneIter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.curr_phase += 1;
        if self.curr_phase >= self.sample_period {
            self.curr_phase = 0;
        }
        let phase = self.curr_phase as f32 / self.sample_period as f32;
        let sample = self.amplitude * (2.0 * std::f32::consts::PI * phase).sin();
        Some(sample)
    }
}
