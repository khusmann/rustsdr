use std::pin::Pin;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

use num_complex::Complex;
use num_traits::Num;

use crate::sampletypes::*;

pub fn from_sample_fn<F, T>(mut f: F, rate: u32, buffer_size: usize) -> impl Stream<Item = Vec<T>>
where
    F: FnMut() -> T,
{
    let delay_period = buffer_size as f32 / rate as f32;
    let interval = time::interval(time::Duration::from_secs_f32(delay_period));
    IntervalStream::new(interval).map(move |_| (0..buffer_size).map(|_| f()).collect())
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

// Type conversions

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

////

#[derive(Copy, Clone)]
pub enum BitDepth {
    Char,
    S16,
    Float,
}

pub enum DynSampleStream<'a> {
    ComplexFloat(Pin<Box<dyn Stream<Item = Vec<ComplexFloat>> + 'a>>),
    ComplexS16(Pin<Box<dyn Stream<Item = Vec<ComplexS16>> + 'a>>),
    ComplexChar(Pin<Box<dyn Stream<Item = Vec<ComplexChar>> + 'a>>),
    Float(Pin<Box<dyn Stream<Item = Vec<Float>> + 'a>>),
    S16(Pin<Box<dyn Stream<Item = Vec<S16>> + 'a>>),
    Char(Pin<Box<dyn Stream<Item = Vec<Char>> + 'a>>),
}

impl<'a> DynSampleStream<'a> {
    pub fn realpart(self) -> DynSampleStream<'a> {
        match self {
            DynSampleStream::ComplexFloat(s) => s.realpart().into_dyn(),
            DynSampleStream::ComplexS16(s) => s.realpart().into_dyn(),
            DynSampleStream::ComplexChar(s) => s.realpart().into_dyn(),
            _ => self,
        }
    }

    pub fn convert(self, to: BitDepth) -> DynSampleStream<'a> {
        match (self, to) {
            (DynSampleStream::ComplexFloat(s), BitDepth::S16) => s.convert_to_s16().into_dyn(),
            (DynSampleStream::ComplexFloat(s), BitDepth::Char) => s.convert_to_char().into_dyn(),
            (DynSampleStream::ComplexS16(s), BitDepth::Float) => s.convert_to_float().into_dyn(),
            (DynSampleStream::ComplexS16(s), BitDepth::Char) => s.convert_to_char().into_dyn(),
            (DynSampleStream::ComplexChar(s), BitDepth::Float) => s.convert_to_float().into_dyn(),
            (DynSampleStream::ComplexChar(s), BitDepth::S16) => s.convert_to_s16().into_dyn(),
            (DynSampleStream::Float(s), BitDepth::S16) => s.convert_to_s16().into_dyn(),
            (DynSampleStream::Float(s), BitDepth::Char) => s.convert_to_char().into_dyn(),
            (DynSampleStream::S16(s), BitDepth::Float) => s.convert_to_float().into_dyn(),
            (DynSampleStream::S16(s), BitDepth::Char) => s.convert_to_char().into_dyn(),
            (DynSampleStream::Char(s), BitDepth::Float) => s.convert_to_float().into_dyn(),
            (DynSampleStream::Char(s), BitDepth::S16) => s.convert_to_s16().into_dyn(),
            (s, _) => s,
        }
    }

    pub fn stream_complex_float(self) -> Pin<Box<dyn Stream<Item = Vec<ComplexFloat>> + 'a>> {
        match self {
            DynSampleStream::ComplexFloat(s) => s,
            _ => panic!("Expected ComplexFloat"),
        }
    }
    pub fn stream_complex_s16(self) -> Pin<Box<dyn Stream<Item = Vec<ComplexS16>> + 'a>> {
        match self {
            DynSampleStream::ComplexS16(s) => s,
            _ => panic!("Expected ComplexS16"),
        }
    }
    pub fn stream_complex_char(self) -> Pin<Box<dyn Stream<Item = Vec<ComplexChar>> + 'a>> {
        match self {
            DynSampleStream::ComplexChar(s) => s,
            _ => panic!("Expected ComplexChar"),
        }
    }
    pub fn stream_float(self) -> Pin<Box<dyn Stream<Item = Vec<Float>> + 'a>> {
        match self {
            DynSampleStream::Float(s) => s,
            _ => panic!("Expected Float"),
        }
    }
    pub fn stream_s16(self) -> Pin<Box<dyn Stream<Item = Vec<S16>> + 'a>> {
        match self {
            DynSampleStream::S16(s) => s,
            _ => panic!("Expected S16"),
        }
    }
    pub fn stream_char(self) -> Pin<Box<dyn Stream<Item = Vec<Char>> + 'a>> {
        match self {
            DynSampleStream::Char(s) => s,
            _ => panic!("Expected Char"),
        }
    }
}

pub trait IntoDynSampleStream<'a, T> {
    fn into_dyn(self) -> DynSampleStream<'a>;
}

impl<'a, St> IntoDynSampleStream<'a, ComplexFloat> for St
where
    St: Stream<Item = Vec<ComplexFloat>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::ComplexFloat(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, ComplexS16> for St
where
    St: Stream<Item = Vec<ComplexS16>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::ComplexS16(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, ComplexChar> for St
where
    St: Stream<Item = Vec<ComplexChar>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::ComplexChar(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, Float> for St
where
    St: Stream<Item = Vec<Float>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::Float(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, S16> for St
where
    St: Stream<Item = Vec<S16>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::S16(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, Char> for St
where
    St: Stream<Item = Vec<Char>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::Char(Box::pin(self))
    }
}
