use num_complex::Complex;

pub trait ConvertSampleFrom<T> {
    fn convert_sample_from(value: T) -> Self;
}

pub trait ConvertSampleInto<T> {
    fn convert_sample_into(self) -> T;
}

impl<T, U> ConvertSampleInto<U> for T
where
    U: ConvertSampleFrom<T>,
{
    fn convert_sample_into(self) -> U {
        U::convert_sample_from(self)
    }
}

impl<T, U> ConvertSampleFrom<Complex<T>> for Complex<U>
where
    U: ConvertSampleFrom<T>,
{
    fn convert_sample_from(value: Complex<T>) -> Self {
        Complex::new(
            U::convert_sample_from(value.re),
            U::convert_sample_from(value.im),
        )
    }
}

impl ConvertSampleFrom<i16> for u8 {
    fn convert_sample_from(value: i16) -> u8 {
        ((value as i32 - i16::MIN as i32) * u8::MAX as i32 / (u16::MAX as i32)) as u8
    }
}

impl ConvertSampleFrom<u8> for i16 {
    fn convert_sample_from(value: u8) -> i16 {
        ((value as i32 * u16::MAX as i32 / u8::MAX as i32) + i16::MIN as i32) as i16
    }
}

impl ConvertSampleFrom<f32> for u8 {
    fn convert_sample_from(value: f32) -> u8 {
        (value * (u8::MAX as f32) / 2.0 - i8::MIN as f32) as u8
    }
}

impl ConvertSampleFrom<u8> for f32 {
    fn convert_sample_from(value: u8) -> f32 {
        value as f32 / (u8::MAX as f32) * 2.0 - 1.0
    }
}

impl ConvertSampleFrom<f32> for i16 {
    fn convert_sample_from(value: f32) -> i16 {
        (value * (i16::MAX as f32)) as i16
    }
}

impl ConvertSampleFrom<i16> for f32 {
    fn convert_sample_from(value: i16) -> f32 {
        value as f32 / (i16::MAX as f32)
    }
}

pub fn convert_sample_buf<T, U>(a: &[T]) -> Vec<U>
where
    T: ConvertSampleInto<U> + Copy,
{
    a.iter().map(|v| (*v).convert_sample_into()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_char_s16() {
        let a = vec![0u8, 128u8, 255u8];
        let b = vec![i16::MIN, 128i16, i16::MAX];
        assert_eq!(a, convert_sample_buf::<i16, u8>(&b));
        assert_eq!(b, convert_sample_buf::<u8, i16>(&a));
    }

    #[test]
    fn convert_char_float() {
        let a = vec![0u8, 51u8, 255u8];
        let b = vec![-1.0f32, -0.6f32, 1.0f32];
        assert_eq!(a, convert_sample_buf::<f32, u8>(&b));
        assert_eq!(b, convert_sample_buf::<u8, f32>(&a));
    }

    #[test]
    fn convert_s16_float() {
        let a = vec![i16::MIN + 1, 0i16, i16::MAX];
        let b = vec![-1.0f32, 0.0f32, 1.0f32];
        assert_eq!(a, convert_sample_buf::<f32, i16>(&b));
        assert_eq!(b, convert_sample_buf::<i16, f32>(&a));
    }
}
