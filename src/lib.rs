use std::iter;
use std::pin::Pin;

use tokio_stream::Stream;

pub fn noise() -> Pin<Box<dyn Stream<Item = f32>>> {
    Box::pin(tokio_stream::iter(iter::from_fn(|| {
        Some(rand::random::<f32>())
    })))
}

pub fn tone(freq: &u32, rate: &u32, amplitude: &f32) -> Pin<Box<dyn Stream<Item = f32>>> {
    let sample_period = rate / freq;
    let amplitude = *amplitude;
    let mut curr_phase = 0u32;
    Box::pin(tokio_stream::iter(iter::from_fn(move || {
        curr_phase += 1;
        if curr_phase >= sample_period {
            curr_phase = 0;
        }
        let phase = curr_phase as f32 / sample_period as f32;
        let sample = amplitude * (2.0 * std::f32::consts::PI * phase).sin();
        Some(sample)
    })))
}

pub fn convert_to_i16(v: f32) -> i16 {
    (v * i16::MAX as f32) as i16
}
