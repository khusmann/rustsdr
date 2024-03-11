use std::io::Error;
use std::pin::Pin;

use rand::Rng;

use tokio::io::stdin;
use tokio::time;

use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

use tokio_util::bytes::{BufMut, Bytes, BytesMut};
use tokio_util::io::ReaderStream;

pub enum BitDepth {
    Char,
    S16,
    Float,
}

pub fn source_noise(
    rate: u32,
    buffer_size: usize,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    let delay_period = buffer_size as f32 / rate as f32;

    let interval = time::interval(time::Duration::from_secs_f32(delay_period));

    let mut rng = rand::thread_rng();

    Box::pin(IntervalStream::new(interval).map(move |_| {
        let mut buf = BytesMut::with_capacity(buffer_size);
        for _ in 0..buffer_size {
            buf.put_u8(rng.gen());
        }
        Ok(buf.freeze())
    }))
}

pub fn source_stdin(buffer_size: usize) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    Box::pin(ReaderStream::with_capacity(stdin(), buffer_size))
}

pub fn source_tone(
    freq: u32,
    rate: u32,
    amplitude: f32,
    buffer_size: usize,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>> {
    let sample_period = rate / freq;
    let mut curr_phase = 0u32;

    let delay_period = buffer_size as f32 / rate as f32;

    let interval = time::interval(time::Duration::from_secs_f32(delay_period));

    Box::pin(IntervalStream::new(interval).map(move |_| {
        let mut buf = BytesMut::with_capacity(buffer_size);

        for _ in 0..buffer_size {
            curr_phase += 1;

            let phase = curr_phase as f32 / sample_period as f32;
            let v = amplitude * (2.0 * std::f32::consts::PI * phase).sin();
            let sample = ((v * 0.5) * (u8::MAX as f32)) as u8 + 128;
            buf.put_u8(sample)
        }

        while curr_phase >= sample_period {
            curr_phase -= sample_period;
        }

        Ok(buf.freeze())
    }))
}
