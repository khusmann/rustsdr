use std::io::Error;
use std::pin::Pin;

use rand::Rng;

use tokio::io::stdin;
use tokio::time;

use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

use tokio_util::bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::io::ReaderStream;

#[derive(Copy, Clone)]
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

pub fn convert_fn(input: BitDepth, output: BitDepth) -> impl Fn(Bytes) -> Bytes {
    move |mut v| {
        let mut buf = BytesMut::new();

        match (input, output) {
            (BitDepth::Char, BitDepth::S16) => {
                while v.has_remaining() {
                    let b = v.get_u8();
                    buf.put_i16(
                        ((b as u16 * (u16::MAX / u8::MAX as u16)) as i32 + i16::MIN as i32) as i16,
                    )
                }
            }
            (BitDepth::S16, BitDepth::Char) => {
                while v.has_remaining() {
                    let b = v.get_i16();
                    buf.put_u8(
                        ((b as i32 - i16::MIN as i32) * u8::MAX as i32 / (u16::MAX as i32)) as u8,
                    )
                }
            }
            (BitDepth::Char, BitDepth::Float) => {
                while v.has_remaining() {
                    let b = v.get_u8();
                    buf.put_f32(b as f32 / (u8::MAX as f32) * 2.0 - 1.0)
                }
            }
            (BitDepth::Float, BitDepth::Char) => {
                while v.has_remaining() {
                    let b = v.get_f32();
                    buf.put_u8((b * (u8::MAX as f32) / 2.0 - i8::MIN as f32) as u8)
                }
            }
            (BitDepth::S16, BitDepth::Float) => {
                while v.has_remaining() {
                    let b = v.get_i16();
                    buf.put_f32(b as f32 / (i16::MAX as f32))
                }
            }
            (BitDepth::Float, BitDepth::S16) => {
                while v.has_remaining() {
                    let b = v.get_f32();
                    buf.put_i16((b * (i16::MAX as f32)) as i16)
                }
            }
            (_, _) => {
                while v.has_remaining() {
                    buf.put_u8(v.get_u8());
                }
            }
        }

        buf.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_char_s16() {
        let input = vec![0, 128, 255];
        let output = vec![128, 0, 0, 128, 127, 255];
        let convert = convert_fn(BitDepth::Char, BitDepth::S16);
        let result = convert(Bytes::from(input));
        assert_eq!(result.to_vec(), output);
    }

    #[test]
    fn convert_s16_char() {
        let input = vec![128, 0, 0, 128, 127, 255];
        let output = vec![0, 128, 255];
        let convert = convert_fn(BitDepth::S16, BitDepth::Char);
        let result = convert(Bytes::from(input));
        assert_eq!(result.to_vec(), output);
    }

    #[test]
    fn convert_char_f32() {
        let input = vec![0, 51, 255];
        let output_f32: Vec<f32> = vec![-1.0, -0.6, 1.0];
        let mut output = BytesMut::new();
        for v in output_f32 {
            output.put_f32(v);
        }
        let convert = convert_fn(BitDepth::Char, BitDepth::Float);
        let result = convert(Bytes::from(input));
        assert_eq!(result, output);
    }

    #[test]
    fn convert_f32_char() {
        let input_f32: Vec<f32> = vec![-1.0, -0.6, 1.0];
        let mut input = BytesMut::new();
        for v in input_f32 {
            input.put_f32(v);
        }
        let output = vec![0, 51, 255];
        let convert = convert_fn(BitDepth::Float, BitDepth::Char);
        let result = convert(input.freeze());
        assert_eq!(result.to_vec(), output);
    }

    #[test]
    fn convert_s16_f32() {
        let input = vec![128, 1, 0, 0, 127, 255];
        let output_f32: Vec<f32> = vec![-1.0, 0.0, 1.0];
        let mut output = BytesMut::new();
        for v in output_f32 {
            output.put_f32(v);
        }
        let convert = convert_fn(BitDepth::S16, BitDepth::Float);
        let result = convert(Bytes::from(input));
        assert_eq!(result, output);
    }

    #[test]
    fn convert_f32_s16() {
        let input_f32: Vec<f32> = vec![-1.0, 0.0, 1.0];
        let mut input = BytesMut::new();
        for v in input_f32 {
            input.put_f32(v);
        }
        let output = vec![128, 1, 0, 0, 127, 255];
        let convert = convert_fn(BitDepth::Float, BitDepth::S16);
        let result = convert(input.freeze());
        assert_eq!(result.to_vec(), output);
    }

    fn convert_identity(bd: BitDepth) {
        let input = vec![0, 128, 255];
        let output = vec![0, 128, 255];
        let convert = convert_fn(bd, bd);
        let result = convert(Bytes::from(input));
        assert_eq!(result.to_vec(), output);
    }

    #[test]
    fn convert_char_char() {
        convert_identity(BitDepth::Char);
    }

    #[test]
    fn convert_s16_s16() {
        convert_identity(BitDepth::S16);
    }

    #[test]
    fn convert_float_float() {
        convert_identity(BitDepth::Float);
    }
}
