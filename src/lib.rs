use std::iter;
use std::pin::Pin;

use rand::Rng;
use tokio::io::stdin;

use tokio_stream::Stream;

use tokio_util::io::ReaderStream;

use std::io::Error;
use tokio_util::bytes::{BufMut, Bytes, BytesMut};

pub fn source_noise(buffer_size: usize) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    let mut rng = rand::thread_rng();
    Box::pin(tokio_stream::iter(iter::from_fn(move || {
        let mut buf = BytesMut::with_capacity(buffer_size);
        for _ in 0..buffer_size {
            buf.put_u8(rng.gen());
        }
        Some(Ok(buf.freeze()))
    })))
}

pub fn source_stdin(buffer_size: usize) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    Box::pin(ReaderStream::with_capacity(stdin(), buffer_size))
}

pub fn source_tone(
    freq: &u32,
    rate: &u32,
    amplitude: &f32,
    buffer_size: usize,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    let sample_period = rate / freq;
    let amplitude = *amplitude;
    let mut curr_phase = 0u32;

    Box::pin(tokio_stream::iter(iter::from_fn(move || {
        let mut buf = BytesMut::with_capacity(buffer_size);

        for _ in 0..buffer_size {
            curr_phase += 1;

            let phase = curr_phase as f32 / sample_period as f32;
            let v = amplitude * (2.0 * std::f32::consts::PI * phase).sin();
            let sample = ((v * u8::MAX as f32) + u8::MAX as f32) as u8;
            buf.put_u8(sample)
        }

        if curr_phase >= sample_period {
            curr_phase = 0;
        }

        Some(Ok(buf.freeze()))
    })))
}
