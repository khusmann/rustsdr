use std::io::Error;
use std::pin::Pin;

use rand::Rng;

use std::ops::{Deref, DerefMut};
use tokio::io::stdin;
use tokio::time;

use futures::stream::{Map, Stream, StreamExt};
use tokio_stream::wrappers::IntervalStream;

use num_complex::Complex;
use num_traits::Num;
use tokio_util::bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::io::ReaderStream;

use core::task::{Context, Poll};
use std;

use itertools::Itertools;

use pin_project::pin_project;

type ComplexFloat = Complex<f32>;
type ComplexS16 = Complex<i16>;
type ComplexChar = Complex<u8>;
type Float = f32;
type S16 = i16;
type Char = u8;

#[derive(Copy, Clone)]
pub enum BitDepth {
    Char,
    S16,
    Float,
}

pub trait BufferedSampleStream {
    type Sample;
    fn map_samples<F, R>(self, f: F) -> impl Stream<Item = Vec<R>>
    where
        F: FnMut(Self::Sample) -> R;
}

pub trait RealBufferedSampleStream: BufferedSampleStream {
    fn lift_complex(self) -> impl Stream<Item = Vec<Complex<Self::Sample>>>;
}

pub trait ComplexBufferedSampleStream: BufferedSampleStream {
    type Arg;
    fn realpart(self) -> impl Stream<Item = Vec<Self::Arg>>;
    fn map_sample_args<R, F>(self, f: F) -> impl Stream<Item = Vec<Complex<R>>>
    where
        F: FnMut(Self::Arg) -> R;
}

pub trait ConvertComplexStream<Src> {
    fn convert_to_s16(self) -> impl Stream<Item = Vec<ComplexS16>>;
    fn convert_to_char(self) -> impl Stream<Item = Vec<ComplexChar>>;
    fn convert_to_float(self) -> impl Stream<Item = Vec<ComplexFloat>>;
}

pub trait ConvertRealStream<Src> {
    fn convert_to_s16(self) -> impl Stream<Item = Vec<S16>>;
    fn convert_to_char(self) -> impl Stream<Item = Vec<Char>>;
    fn convert_to_float(self) -> impl Stream<Item = Vec<Float>>;
}

impl<St> ConvertComplexStream<ComplexFloat> for St
where
    St: Stream<Item = Vec<ComplexFloat>>,
{
    fn convert_to_s16(self) -> impl Stream<Item = Vec<ComplexS16>> {
        self.map_sample_args(float_to_s16)
    }

    fn convert_to_char(self) -> impl Stream<Item = Vec<ComplexChar>> {
        self.map_sample_args(float_to_char)
    }

    fn convert_to_float(self) -> impl Stream<Item = Vec<ComplexFloat>> {
        self
    }
}

impl<St> ConvertComplexStream<ComplexS16> for St
where
    St: Stream<Item = Vec<ComplexS16>>,
{
    fn convert_to_float(self) -> impl Stream<Item = Vec<ComplexFloat>> {
        self.map_sample_args(s16_to_float)
    }

    fn convert_to_char(self) -> impl Stream<Item = Vec<ComplexChar>> {
        self.map_sample_args(s16_to_char)
    }

    fn convert_to_s16(self) -> impl Stream<Item = Vec<ComplexS16>> {
        self
    }
}

impl<St> ConvertComplexStream<ComplexChar> for St
where
    St: Stream<Item = Vec<ComplexChar>>,
{
    fn convert_to_float(self) -> impl Stream<Item = Vec<ComplexFloat>> {
        self.map_sample_args(char_to_float)
    }

    fn convert_to_s16(self) -> impl Stream<Item = Vec<ComplexS16>> {
        self.map_sample_args(char_to_s16)
    }

    fn convert_to_char(self) -> impl Stream<Item = Vec<ComplexChar>> {
        self
    }
}

impl<St> ConvertRealStream<Float> for St
where
    St: Stream<Item = Vec<Float>>,
{
    fn convert_to_s16(self) -> impl Stream<Item = Vec<S16>> {
        self.map_samples(float_to_s16)
    }

    fn convert_to_char(self) -> impl Stream<Item = Vec<Char>> {
        self.map_samples(float_to_char)
    }

    fn convert_to_float(self) -> impl Stream<Item = Vec<Float>> {
        self
    }
}

impl<St> ConvertRealStream<S16> for St
where
    St: Stream<Item = Vec<S16>>,
{
    fn convert_to_float(self) -> impl Stream<Item = Vec<Float>> {
        self.map_samples(s16_to_float)
    }

    fn convert_to_char(self) -> impl Stream<Item = Vec<Char>> {
        self.map_samples(s16_to_char)
    }

    fn convert_to_s16(self) -> impl Stream<Item = Vec<S16>> {
        self
    }
}

impl<St> ConvertRealStream<Char> for St
where
    St: Stream<Item = Vec<Char>>,
{
    fn convert_to_float(self) -> impl Stream<Item = Vec<Float>> {
        self.map_samples(char_to_float)
    }

    fn convert_to_s16(self) -> impl Stream<Item = Vec<S16>> {
        self.map_samples(char_to_s16)
    }

    fn convert_to_char(self) -> impl Stream<Item = Vec<Char>> {
        self
    }
}

impl<T, St> BufferedSampleStream for St
where
    St: Stream<Item = Vec<T>>,
{
    type Sample = T;

    fn map_samples<F, R>(self, mut f: F) -> impl Stream<Item = Vec<R>>
    where
        F: FnMut(T) -> R,
    {
        self.map(move |chunk| chunk.into_iter().map(|v| f(v)).collect())
    }
}

impl<T, St> RealBufferedSampleStream for St
where
    St: Stream<Item = Vec<T>>,
    T: Num,
{
    fn lift_complex(self) -> impl Stream<Item = Vec<Complex<T>>> {
        self.map_samples(|v| Complex::new(v, T::zero()))
    }
}

impl<T, St> ComplexBufferedSampleStream for St
where
    St: Stream<Item = Vec<Complex<T>>>,
{
    type Arg = T;

    fn realpart(self) -> impl Stream<Item = Vec<T>> {
        self.map_samples(|v| v.re)
    }

    fn map_sample_args<R, F>(self, mut f: F) -> impl Stream<Item = Vec<Complex<R>>>
    where
        F: FnMut(T) -> R,
    {
        self.map_samples(move |v| Complex::new(f(v.re), f(v.im)))
    }
}

pub fn from_sample_fn<F, T>(mut f: F, rate: u32, buffer_size: usize) -> impl Stream<Item = Vec<T>>
where
    F: FnMut() -> T,
{
    let delay_period = buffer_size as f32 / rate as f32;
    let interval = time::interval(time::Duration::from_secs_f32(delay_period));
    IntervalStream::new(interval).map(move |_| (0..buffer_size).map(|_| f()).collect())
}

/*
#[pin_project]
pub struct BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    #[pin]
    stream: St,
}



impl<St, T> BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    pub fn new(stream: St) -> Self {
        Self { stream }
    }

    pub fn map_chunks<R, F>(self, f: F) -> BufferedSampleStream<impl Stream<Item = Vec<R>>, R>
    where
        F: FnMut(Vec<T>) -> Vec<R>,
    {
        BufferedSampleStream::new(self.stream.map(f))
    }

    pub fn map_samples<R, F>(self, mut f: F) -> BufferedSampleStream<impl Stream<Item = Vec<R>>, R>
    where
        F: FnMut(T) -> R,
    {
        self.map_chunks(move |chunk| chunk.into_iter().map(|v| f(v)).collect())
    }
}

impl<St, T> BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
    T: Num,
{
    pub fn lift_complex(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<Complex<T>>>, Complex<T>> {
        self.map_samples(|v| Complex::new(v, T::zero()))
    }
}

impl<St, T> BufferedSampleStream<St, Complex<T>>
where
    St: Stream<Item = Vec<Complex<T>>>,
{
    pub fn map_sample_args<R, F>(
        self,
        mut f: F,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<Complex<R>>>, Complex<R>>
    where
        F: FnMut(T) -> R,
    {
        self.map_samples(move |v| Complex::new(f(v.re), f(v.im)))
    }

    pub fn realpart(self) -> BufferedSampleStream<impl Stream<Item = Vec<T>>, T> {
        self.map_samples(|v| v.re)
    }
}

impl<St> BufferedSampleStream<St, ComplexFloat>
where
    St: Stream<Item = Vec<ComplexFloat>>,
{
    pub fn convert_to_s16(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexS16>>, ComplexS16> {
        self.map_sample_args(float_to_s16)
    }

    pub fn convert_to_char(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexChar>>, ComplexChar> {
        self.map_sample_args(float_to_char)
    }
}

impl<St> BufferedSampleStream<St, ComplexS16>
where
    St: Stream<Item = Vec<ComplexS16>>,
{
    pub fn convert_to_float(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexFloat>>, ComplexFloat> {
        self.map_sample_args(s16_to_float)
    }

    pub fn convert_to_char(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexChar>>, ComplexChar> {
        self.map_sample_args(s16_to_char)
    }
}

impl<St> BufferedSampleStream<St, ComplexChar>
where
    St: Stream<Item = Vec<ComplexChar>>,
{
    pub fn convert_to_float(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexFloat>>, ComplexFloat> {
        self.map_sample_args(char_to_float)
    }

    pub fn convert_to_s16(
        self,
    ) -> BufferedSampleStream<impl Stream<Item = Vec<ComplexS16>>, ComplexS16> {
        self.map_sample_args(char_to_s16)
    }
}

impl<St> BufferedSampleStream<St, Float>
where
    St: Stream<Item = Vec<Float>>,
{
    pub fn convert_to_s16(self) -> BufferedSampleStream<impl Stream<Item = Vec<S16>>, S16> {
        self.map_samples(float_to_s16)
    }

    pub fn convert_to_char(self) -> BufferedSampleStream<impl Stream<Item = Vec<Char>>, Char> {
        self.map_samples(float_to_char)
    }
}

impl<St> BufferedSampleStream<St, S16>
where
    St: Stream<Item = Vec<S16>>,
{
    pub fn convert_to_float(self) -> BufferedSampleStream<impl Stream<Item = Vec<Float>>, Float> {
        self.map_samples(s16_to_float)
    }

    pub fn convert_to_char(self) -> BufferedSampleStream<impl Stream<Item = Vec<Char>>, Char> {
        self.map_samples(s16_to_char)
    }
}

impl<St> BufferedSampleStream<St, Char>
where
    St: Stream<Item = Vec<Char>>,
{
    pub fn convert_to_float(self) -> BufferedSampleStream<impl Stream<Item = Vec<Float>>, Float> {
        self.map_samples(char_to_float)
    }

    pub fn convert_to_s16(self) -> BufferedSampleStream<impl Stream<Item = Vec<S16>>, S16> {
        self.map_samples(char_to_s16)
    }
}

impl<St, T> Stream for BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    type Item = Vec<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

impl<St, T> From<St> for BufferedSampleStream<St, T>
where
    St: Stream<Item = Vec<T>>,
{
    fn from(stream: St) -> Self {
        Self { stream }
    }
}
*/
/*
pub struct Pipeline<T> {
    inner: Pin<Box<dyn Stream<Item = Vec<T>>>>,
}

impl<S, T> From<S> for Pipeline<T>
where
    S: Stream<Item = Vec<T>> + 'static,
{
    fn from(s: S) -> Self {
        Self { inner: Box::pin(s) }
    }
}

impl<T> Deref for Pipeline<T> {
    type Target = Pin<Box<dyn Stream<Item = Vec<T>>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Pipeline<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Pipeline<T>
where
    T: 'static,
{
    pub fn map_chunks<R>(self, f: impl FnMut(Vec<T>) -> Vec<R> + 'static) -> Pipeline<R> {
        Pipeline::from(self.inner.map(f))
    }

    pub fn map_values<R>(self, mut f: impl FnMut(T) -> R + 'static) -> Pipeline<R> {
        self.map_chunks(move |chunk| chunk.into_iter().map(|v| f(v)).collect())
    }
}

impl Pipeline<ComplexFloat> {
    pub fn convert_to_float(self) -> Self {
        self
    }

    pub fn convert_to_s16(self) -> Pipeline<ComplexS16> {
        self.map_values(lift_complex(float_to_s16))
    }

    pub fn convert_to_char(self) -> Pipeline<ComplexChar> {
        self.map_values(lift_complex(float_to_char))
    }
}
*/
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
