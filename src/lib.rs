use std::io::Error;
use std::pin::Pin;

use rand::Rng;

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

pub enum SampleStreamRef {
    ComplexFloat(Pin<Box<dyn Stream<Item = ComplexFloatChunk>>>),
    ComplexS16(Pin<Box<dyn Stream<Item = ComplexS16Chunk>>>),
    ComplexChar(Pin<Box<dyn Stream<Item = ComplexCharChunk>>>),
    Float(Pin<Box<dyn Stream<Item = FloatChunk>>>),
    S16(Pin<Box<dyn Stream<Item = S16Chunk>>>),
    Char(Pin<Box<dyn Stream<Item = CharChunk>>>),
}

pub struct Pipeline {
    inner: SampleStreamRef,
}

impl<T> From<T> for Pipeline
where
    T: Stream<Item = ComplexFloatChunk> + 'static,
{
    fn from(s: T) -> Self {
        Self {
            inner: SampleStreamRef::ComplexFloat(Box::pin(s)),
        }
    }
}

impl Pipeline {
    pub fn convert_float(self) -> Self {
        match self.inner {
            SampleStreamRef::ComplexFloat(_) => self,
            SampleStreamRef::Float(_) => self,
            SampleStreamRef::S16(s) => {
                let s = s.map(|chunk| chunk.iter().map(|&v| s16_to_float(v)).collect());
                Self {
                    inner: SampleStreamRef::Float(Box::pin(s)),
                }
            }
            _ => panic!("pipeline is not a complex float stream"),
        }
    }

    pub fn stream_complex_float(self) -> Pin<Box<dyn Stream<Item = ComplexFloatChunk>>> {
        match self.inner {
            SampleStreamRef::ComplexFloat(s) => s,
            _ => panic!("pipeline is not a complex float stream"),
        }
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
