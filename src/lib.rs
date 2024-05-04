use std::io::Error;
use std::pin::Pin;

use rand::Rng;

use std::ops::{Deref, DerefMut};
use tokio::io::stdin;
use tokio::time;

use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

use num_complex::{Complex, ComplexFloat};
use tokio_util::bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::io::ReaderStream;

use std;

use itertools::Itertools;

use pin_project::pin_project;

type ComplexFloatChunk = Vec<Complex<f32>>;
type ComplexS16Chunk = Vec<Complex<i16>>;
type ComplexCharChunk = Vec<Complex<u8>>;
type FloatChunk = Vec<f32>;
type S16Chunk = Vec<i16>;
type CharChunk = Vec<u8>;

#[derive(Copy, Clone)]
pub enum BitDepth {
    Char,
    S16,
    Float,
}

pub struct Pipeline<T> {
    inner: Pin<Box<dyn Stream<Item = T>>>,
}

impl<T> From<T> for Pipeline<ComplexFloatChunk>
where
    T: Stream<Item = ComplexFloatChunk> + 'static,
{
    fn from(s: T) -> Self {
        Self { inner: Box::pin(s) }
    }
}

impl<T> From<T> for Pipeline<ComplexS16Chunk>
where
    T: Stream<Item = ComplexS16Chunk> + 'static,
{
    fn from(s: T) -> Self {
        Self { inner: Box::pin(s) }
    }
}

impl<T> From<T> for Pipeline<ComplexCharChunk>
where
    T: Stream<Item = ComplexCharChunk> + 'static,
{
    fn from(s: T) -> Self {
        Self { inner: Box::pin(s) }
    }
}

impl<T> Deref for Pipeline<T> {
    type Target = Pin<Box<dyn Stream<Item = T>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Pipeline<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Pipeline<ComplexFloatChunk> {
    pub fn convert_to_float(self) -> Self {
        self
    }
    pub fn convert_to_s16(self) -> Pipeline<ComplexS16Chunk> {
        Pipeline::from(self.inner.map(|chunk| {
            chunk
                .iter()
                .map(|&c| Complex::new(float_to_s16(c.re), float_to_s16(c.im)))
                .collect()
        }))
    }
    pub fn convert_to_char(self) -> Pipeline<ComplexCharChunk> {
        Pipeline::from(self.inner.map(|chunk| {
            chunk
                .iter()
                .map(|&c| Complex::new(float_to_char(c.re), float_to_char(c.im)))
                .collect()
        }))
    }
}

/*
pub fn source_stdin(buffer_size: usize) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    Box::pin(ReaderStream::with_capacity(stdin(), buffer_size))
}
*/
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

pub fn buffered_sample_stream<'a, F, R>(
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

pub fn char_to_s16(v: u8) -> i16 {
    ((v as i32 * u16::MAX as i32 / u8::MAX as i32) + i16::MIN as i32) as i16
}

pub fn s16_to_char(v: i16) -> u8 {
    ((v as i32 - i16::MIN as i32) * u8::MAX as i32 / (u16::MAX as i32)) as u8
}

pub fn char_to_float(v: u8) -> f32 {
    v as f32 / (u8::MAX as f32) * 2.0 - 1.0
}

pub fn float_to_char(v: f32) -> u8 {
    (v * (u8::MAX as f32) / 2.0 - i8::MIN as f32) as u8
}

pub fn s16_to_float(v: i16) -> f32 {
    v as f32 / (i16::MAX as f32)
}

pub fn float_to_s16(v: f32) -> i16 {
    (v * (i16::MAX as f32)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_char_s16() {
        assert_eq!(char_to_s16(0), i16::MIN);
        assert_eq!(char_to_s16(128), 128);
        assert_eq!(char_to_s16(255), i16::MAX);
    }

    #[test]
    fn convert_s16_char() {
        assert_eq!(s16_to_char(i16::MIN), 0);
        assert_eq!(s16_to_char(128), 128);
        assert_eq!(s16_to_char(i16::MAX), 255);
    }

    #[test]
    fn convert_char_float() {
        assert_eq!(char_to_float(0), -1.0);
        assert_eq!(char_to_float(51), -0.6);
        assert_eq!(char_to_float(255), 1.0);
    }

    #[test]
    fn convert_float_char() {
        assert_eq!(float_to_char(-1.0), 0);
        assert_eq!(float_to_char(-0.6), 51);
        assert_eq!(float_to_char(1.0), 255);
    }

    #[test]
    fn convert_s16_float() {
        assert_eq!(s16_to_float(i16::MIN + 1), -1.0);
        assert_eq!(s16_to_float(0), 0.0);
        assert_eq!(s16_to_float(i16::MAX), 1.0);
    }

    #[test]
    fn convert_float_s16() {
        assert_eq!(float_to_s16(-1.0), i16::MIN + 1);
        assert_eq!(float_to_s16(0.0), 0);
        assert_eq!(float_to_s16(1.0), i16::MAX);
    }
}
