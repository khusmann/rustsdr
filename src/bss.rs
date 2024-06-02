use crate::gen;
use crate::sampletypes::*;

use num_complex::Complex;
use num_traits::{FromBytes, Num, ToBytes};
use std::marker::PhantomData;
use std::pin::Pin;

use tokio::io::stdin;
use tokio::time;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

use tokio_util::bytes::{Buf, BytesMut};
use tokio_util::codec::{Decoder, FramedRead};

struct SampleDecoder<T, const N: usize> {
    _phantom: PhantomData<T>,
}

impl<T, const N: usize> SampleDecoder<T, N> {
    fn new() -> Self {
        SampleDecoder {
            _phantom: PhantomData,
        }
    }
}

impl<T, const N: usize> Decoder for SampleDecoder<T, N>
where
    T: FromBytes<Bytes = [u8; N]>,
{
    type Item = Vec<T>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < N {
            // Not enough data to read type
            return Ok(None);
        }

        let length = (src.len() / N) * N;

        let out = src
            .as_ref()
            .chunks_exact(N)
            .map(|v| FromBytes::from_le_bytes(v.try_into().unwrap()))
            .collect();

        src.advance(length);

        Ok(Some(out))
    }
}

pub trait SampleStream {
    type Sample;

    fn convert_samples<U>(self) -> impl Stream<Item = Vec<U>>
    where
        Self::Sample: ConvertSampleInto<U>;

    fn map_samples<F, R>(self, f: F) -> impl Stream<Item = Vec<R>>
    where
        F: FnMut(Self::Sample) -> R;
}

impl<St, T> SampleStream for St
where
    St: Stream<Item = Vec<T>>,
{
    type Sample = T;
    fn convert_samples<U>(self) -> impl Stream<Item = Vec<U>>
    where
        Self::Sample: ConvertSampleInto<U>,
    {
        self.map_samples(|v| v.convert_sample_into())
    }

    fn map_samples<F, R>(self, mut f: F) -> impl Stream<Item = Vec<R>>
    where
        F: FnMut(T) -> R,
    {
        self.map(move |chunk| chunk.into_iter().map(|v| f(v)).collect())
    }
}

pub trait RealSampleStream: SampleStream {
    const SAMPLE_BYTES: usize;
    fn as_complex(self) -> impl Stream<Item = Vec<Complex<Self::Sample>>>;
    fn serialize(self) -> impl Stream<Item = Vec<u8>>;
}

impl<St, T, const N: usize> RealSampleStream for St
where
    St: SampleStream<Sample = T> + Stream<Item = Vec<T>>,
    T: Num + Copy + ToBytes<Bytes = [u8; N]>,
{
    const SAMPLE_BYTES: usize = N;

    fn as_complex(self) -> impl Stream<Item = Vec<Complex<T>>> {
        self.map_samples(|v| Complex::new(v, T::zero()))
    }

    fn serialize(self) -> impl Stream<Item = Vec<u8>> {
        self.map(|b| {
            b.into_iter()
                .map(|v| v.to_le_bytes().into_iter())
                .flatten()
                .collect()
        })
    }
}

pub trait ComplexSampleStream: SampleStream {
    type Arg;
    const ARG_BYTES: usize;
    fn realpart(self) -> impl Stream<Item = Vec<Self::Arg>>;
    fn impart(self) -> impl Stream<Item = Vec<Self::Arg>>;
    fn serialize(self) -> impl Stream<Item = Vec<u8>>;
}

impl<St, T, const N: usize> ComplexSampleStream for St
where
    St: SampleStream<Sample = Complex<T>> + Stream<Item = Vec<Complex<T>>>,
    T: Num + Copy + ToBytes<Bytes = [u8; N]>,
{
    type Arg = T;
    const ARG_BYTES: usize = N;

    fn realpart(self) -> impl Stream<Item = Vec<T>> {
        self.map_samples(|v| v.re)
    }

    fn impart(self) -> impl Stream<Item = Vec<T>> {
        self.map_samples(|v| v.im)
    }

    fn serialize(self) -> impl Stream<Item = Vec<u8>> {
        self.map(|b| {
            b.into_iter()
                .map(|v| {
                    [
                        v.re.to_le_bytes().into_iter(),
                        v.im.to_le_bytes().into_iter(),
                    ]
                    .into_iter()
                    .flatten()
                })
                .flatten()
                .collect()
        })
    }
}

pub fn stream_sample_fn<F, T>(
    mut f: F,
    rate: u32,
    sample_buffer_size: usize,
) -> impl Stream<Item = Vec<T>>
where
    F: FnMut() -> T,
{
    let delay_period = sample_buffer_size as f32 / rate as f32;
    let interval = time::interval(time::Duration::from_secs_f32(delay_period));
    IntervalStream::new(interval).map(move |_| (0..sample_buffer_size).map(|_| f()).collect())
}

pub fn stream_stdin_samples<T, const N: usize>(buffer_size: usize) -> impl Stream<Item = Vec<T>>
where
    T: FromBytes<Bytes = [u8; N]>,
{
    FramedRead::with_capacity(stdin(), SampleDecoder::<T, N>::new(), buffer_size).map(|v| {
        let out = v.unwrap();
        //eprintln!("got {:?} samples", out.len());
        out
    })
}

#[derive(Copy, Clone, Debug)]
pub enum BitDepth {
    Char,
    S16,
    Float,
}

#[derive(Copy, Clone, Debug)]
pub enum NumType {
    Real,
    Complex,
}
pub enum DynSampleStream<'a> {
    ComplexFloat(Pin<Box<dyn Stream<Item = Vec<Complex<f32>>> + 'a>>),
    ComplexS16(Pin<Box<dyn Stream<Item = Vec<Complex<i16>>> + 'a>>),
    ComplexChar(Pin<Box<dyn Stream<Item = Vec<Complex<u8>>> + 'a>>),
    Float(Pin<Box<dyn Stream<Item = Vec<f32>> + 'a>>),
    S16(Pin<Box<dyn Stream<Item = Vec<i16>> + 'a>>),
    Char(Pin<Box<dyn Stream<Item = Vec<u8>> + 'a>>),
}

impl<'a> DynSampleStream<'a> {
    pub fn source_noise(
        rate: u32,
        sample_buffer_size: usize,
        num_type: NumType,
        bit_depth: BitDepth,
    ) -> DynSampleStream<'a> {
        let stream = stream_sample_fn(gen::rand_fn(), rate, sample_buffer_size).into_dyn();

        let stream = match num_type {
            NumType::Real => stream.realpart(),
            NumType::Complex => stream,
        };

        stream.convert_samples(bit_depth)
    }

    pub fn source_tone(
        freq: u32,
        rate: u32,
        amplitude: f32,
        sample_buffer_size: usize,
        num_type: NumType,
        bit_depth: BitDepth,
    ) -> DynSampleStream<'a> {
        let stream = stream_sample_fn(
            gen::tone_fn(freq, rate, amplitude),
            rate,
            sample_buffer_size,
        )
        .into_dyn();

        let stream = match num_type {
            NumType::Real => stream.realpart(),
            NumType::Complex => stream,
        };

        stream.convert_samples(bit_depth)
    }

    pub fn source_stdin(
        sample_buffer_size: usize,
        num_type: NumType,
        bit_depth: BitDepth,
    ) -> DynSampleStream<'a> {
        match (num_type, bit_depth) {
            (NumType::Real, BitDepth::Float) => {
                stream_stdin_samples::<f32, 4>(sample_buffer_size).into_dyn()
            }
            (NumType::Real, BitDepth::S16) => {
                stream_stdin_samples::<i16, 2>(sample_buffer_size).into_dyn()
            }
            (NumType::Real, BitDepth::Char) => {
                stream_stdin_samples::<u8, 1>(sample_buffer_size).into_dyn()
            }
            _ => panic!("Type not supported"),
        }
    }

    pub fn convert_samples(self, bit_depth: BitDepth) -> DynSampleStream<'a> {
        match (self, bit_depth) {
            (DynSampleStream::ComplexFloat(s), BitDepth::Float) => s.into_dyn(),
            (DynSampleStream::ComplexS16(s), BitDepth::Float) => {
                s.convert_samples::<Complex<f32>>().into_dyn()
            }
            (DynSampleStream::ComplexChar(s), BitDepth::Float) => {
                s.convert_samples::<Complex<f32>>().into_dyn()
            }
            (DynSampleStream::Float(s), BitDepth::Float) => s.into_dyn(),
            (DynSampleStream::S16(s), BitDepth::Float) => s.convert_samples::<f32>().into_dyn(),
            (DynSampleStream::Char(s), BitDepth::Float) => s.convert_samples::<f32>().into_dyn(),
            (DynSampleStream::ComplexFloat(s), BitDepth::S16) => {
                s.convert_samples::<Complex<i16>>().into_dyn()
            }
            (DynSampleStream::ComplexS16(s), BitDepth::S16) => s.into_dyn(),
            (DynSampleStream::ComplexChar(s), BitDepth::S16) => {
                s.convert_samples::<Complex<i16>>().into_dyn()
            }
            (DynSampleStream::Float(s), BitDepth::S16) => s.convert_samples::<i16>().into_dyn(),
            (DynSampleStream::S16(s), BitDepth::S16) => s.into_dyn(),
            (DynSampleStream::Char(s), BitDepth::S16) => s.convert_samples::<i16>().into_dyn(),
            (DynSampleStream::ComplexFloat(s), BitDepth::Char) => {
                s.convert_samples::<Complex<u8>>().into_dyn()
            }
            (DynSampleStream::ComplexS16(s), BitDepth::Char) => {
                s.convert_samples::<Complex<u8>>().into_dyn()
            }
            (DynSampleStream::ComplexChar(s), BitDepth::Char) => s.into_dyn(),
            (DynSampleStream::Float(s), BitDepth::Char) => s.convert_samples::<u8>().into_dyn(),
            (DynSampleStream::S16(s), BitDepth::Char) => s.convert_samples::<u8>().into_dyn(),
            (DynSampleStream::Char(s), BitDepth::Char) => s.into_dyn(),
        }
    }
    pub fn realpart(self) -> DynSampleStream<'a> {
        match self {
            DynSampleStream::ComplexFloat(s) => s.realpart().into_dyn(),
            DynSampleStream::ComplexS16(s) => s.realpart().into_dyn(),
            DynSampleStream::ComplexChar(s) => s.realpart().into_dyn(),
            _ => panic!("stream is not complex"),
        }
    }
    pub fn impart(self) -> DynSampleStream<'a> {
        match self {
            DynSampleStream::ComplexFloat(s) => s.impart().into_dyn(),
            DynSampleStream::ComplexS16(s) => s.impart().into_dyn(),
            DynSampleStream::ComplexChar(s) => s.impart().into_dyn(),
            _ => panic!("stream is not complex"),
        }
    }
    pub fn as_complex(self) -> DynSampleStream<'a> {
        match self {
            DynSampleStream::Float(s) => s.as_complex().into_dyn(),
            DynSampleStream::S16(s) => s.as_complex().into_dyn(),
            DynSampleStream::Char(s) => s.as_complex().into_dyn(),
            _ => panic!("stream is not real"),
        }
    }
    pub fn serialize(self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + 'a>> {
        match self {
            DynSampleStream::ComplexFloat(s) => Box::pin(s.serialize()),
            DynSampleStream::ComplexS16(s) => Box::pin(s.serialize()),
            DynSampleStream::ComplexChar(s) => Box::pin(s.serialize()),
            DynSampleStream::Float(s) => Box::pin(s.serialize()),
            DynSampleStream::S16(s) => Box::pin(s.serialize()),
            DynSampleStream::Char(s) => Box::pin(s.serialize()),
        }
    }
}

pub trait IntoDynSampleStream<'a, T> {
    fn into_dyn(self) -> DynSampleStream<'a>;
}

impl<'a, St> IntoDynSampleStream<'a, Complex<f32>> for St
where
    St: Stream<Item = Vec<Complex<f32>>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::ComplexFloat(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, Complex<i16>> for St
where
    St: Stream<Item = Vec<Complex<i16>>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::ComplexS16(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, Complex<u8>> for St
where
    St: Stream<Item = Vec<Complex<u8>>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::ComplexChar(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, f32> for St
where
    St: Stream<Item = Vec<f32>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::Float(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, i16> for St
where
    St: Stream<Item = Vec<i16>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::S16(Box::pin(self))
    }
}

impl<'a, St> IntoDynSampleStream<'a, u8> for St
where
    St: Stream<Item = Vec<u8>> + 'a,
{
    fn into_dyn(self) -> DynSampleStream<'a> {
        DynSampleStream::Char(Box::pin(self))
    }
}
