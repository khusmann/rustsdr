use num_complex::Complex;

pub type ComplexFloat = Complex<f32>;
pub type ComplexS16 = Complex<i16>;
pub type ComplexChar = Complex<u8>;
pub type Float = f32;
pub type S16 = i16;
pub type Char = u8;

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