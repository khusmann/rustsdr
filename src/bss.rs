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
