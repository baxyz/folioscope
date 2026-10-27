// SPDX-License-Identifier: AGPL-3.0-or-later

//! A cursor over Kiwi bytes: LEB128 varints, zigzag integers, Kiwi floats, NUL-terminated strings,
//! on helpers4's `ByteReader`. Kiwi's 64-bit varints and floats have their own encoding, read here.

use helpers4::bytes::ByteReader;

use super::{KiwiError, Result};

pub(super) struct Reader<'a> {
    bytes: ByteReader<'a>,
}

impl<'a> Reader<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            bytes: ByteReader::new(data),
        }
    }

    pub(super) fn remaining(&self) -> usize {
        self.bytes.remaining()
    }

    /// The end of the data, where a read fell short.
    fn eof(&self) -> KiwiError {
        KiwiError::Eof(self.bytes.position())
    }

    pub(super) fn byte(&mut self) -> Result<u8> {
        self.bytes.read_u8().ok_or_else(|| self.eof())
    }

    pub(super) fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        self.bytes.read_bytes(len).ok_or_else(|| self.eof())
    }

    pub(super) fn var_uint(&mut self) -> Result<u32> {
        self.bytes.read_var_u32().ok_or_else(|| self.eof())
    }

    pub(super) fn var_int(&mut self) -> Result<i32> {
        self.bytes.read_var_i32().ok_or_else(|| self.eof())
    }

    /// Kiwi's 64-bit varints are not LEB128: after eight groups of 7 bits, the ninth byte holds
    /// 8 full bits.
    pub(super) fn var_uint64(&mut self) -> Result<u64> {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            let byte = self.byte()?;
            if byte & 0x80 == 0 || shift >= 56 {
                return Ok(value | (u64::from(byte) << shift));
            }
            value |= u64::from(byte & 0x7f) << shift;
            shift += 7;
        }
    }

    #[allow(clippy::cast_possible_wrap)] // zigzag: the decoded bits are reinterpreted
    pub(super) fn var_int64(&mut self) -> Result<i64> {
        let v = self.var_uint64()?;
        Ok(((v >> 1) as i64) ^ -((v & 1) as i64))
    }

    /// Kiwi stores floats with the exponent moved to the low bits, and a single `0` byte for zero.
    pub(super) fn var_float(&mut self) -> Result<f32> {
        let first = self.byte()?;
        if first == 0 {
            return Ok(0.0);
        }
        let rest = self.take(3)?;
        let bits = u32::from_le_bytes([first, rest[0], rest[1], rest[2]]);
        Ok(f32::from_bits(bits.rotate_left(23)))
    }

    pub(super) fn string(&mut self) -> Result<String> {
        let start = self.bytes.position();
        self.bytes
            .read_cstr()
            .map(str::to_owned)
            .ok_or(KiwiError::Utf8(start))
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;

    #[test]
    fn var_uint_reads_leb128() {
        assert_eq!(Reader::new(&[0x00]).var_uint().unwrap(), 0);
        assert_eq!(Reader::new(&[0x7f]).var_uint().unwrap(), 127);
        assert_eq!(Reader::new(&[0x80, 0x01]).var_uint().unwrap(), 128);
        assert_eq!(
            Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x0f])
                .var_uint()
                .unwrap(),
            u32::MAX
        );
    }

    #[test]
    fn var_int_reads_zigzag() {
        assert_eq!(Reader::new(&[0x00]).var_int().unwrap(), 0);
        assert_eq!(Reader::new(&[0x01]).var_int().unwrap(), -1);
        assert_eq!(Reader::new(&[0x02]).var_int().unwrap(), 1);
        assert_eq!(Reader::new(&[0x03]).var_int().unwrap(), -2);
    }

    #[test]
    fn var_float_reads_kiwi_floats() {
        assert_eq!(Reader::new(&[0x00]).var_float().unwrap().to_bits(), 0);
        // 1.0f32 is 0x3f80_0000; Kiwi rotates it right by 23 bits before writing it little-endian.
        let bits = 1.0f32.to_bits().rotate_right(23).to_le_bytes();
        assert!((Reader::new(&bits).var_float().unwrap() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn string_reads_null_terminated_utf8() {
        let mut r = Reader::new(b"caf\xc3\xa9\0next\0");
        assert_eq!(r.string().unwrap(), "café");
        assert_eq!(r.string().unwrap(), "next");
    }
}
