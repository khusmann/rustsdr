use num_complex::Complex;

use std;

use rand::Rng;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};
use tokio_util::io::ReaderStream;

use itertools::Itertools;

use tokio_util::bytes::{Buf, Bytes};

use pin_project::pin_project;

type ComplexF32Chunk = Vec<Complex<f32>>;
type ComplexS16Chunk = Vec<Complex<i16>>;
type ComplexCharChunk = Vec<Complex<u8>>;
type F32Chunk = Vec<f32>;
type S16Chunk = Vec<i16>;
type CharChunk = Vec<u8>;

pub fn tone_sample_gen(freq: u32, rate: u32, amplitude: f32) -> impl FnMut() -> Complex<f32> {
    let sample_period = rate / freq;
    let mut curr_phase = 0u32;
    move || {
        let x = 2.0 * std::f32::consts::PI * (curr_phase as f32 / sample_period as f32);
        curr_phase = (curr_phase + 1) % sample_period;
        Complex::new(x.cos(), x.sin()) * amplitude
    }
}

pub fn rand_sample_gen() -> impl FnMut() -> Complex<f32> {
    let mut rng = rand::thread_rng();
    move || Complex::new(rng.gen(), rng.gen())
}

pub fn buffered_gen_stream<'a, F, R>(
    mut f: F,
    rate: u32,
    buffer_size: usize,
) -> impl Stream<Item = Vec<R>>
where
    F: FnMut() -> R,
{
    let delay_period = buffer_size as f32 / rate as f32;
    let interval = time::interval(time::Duration::from_secs_f32(delay_period));
    IntervalStream::new(interval).map(move |_| (0..buffer_size).map(|_| f()).collect())
}
