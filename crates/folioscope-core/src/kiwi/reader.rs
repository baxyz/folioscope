// SPDX-License-Identifier: AGPL-3.0-or-later

//! A cursor over Kiwi bytes: LEB128 varints, zigzag integers, Kiwi floats, NUL-terminated strings.

use super::{KiwiError, Result};

// TODO(helpers4): move the cursor and the LEB128/zigzag readers to the helpers4 `binary` module.
pub(super) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub(super) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(super) fn byte(&mut self) -> Result<u8> {
        let b = *self.data.get(self.pos).ok_or(KiwiError::Eof(self.pos))?;
        self.pos += 1;
        Ok(b)
    }

    pub(super) fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(len).ok_or(KiwiError::Eof(self.pos))?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or(KiwiError::Eof(self.pos))?;
        self.pos = end;
        Ok(slice)
    }

    pub(super) fn var_uint(&mut self) -> Result<u32> {
        let mut value = 0u32;
        let mut shift = 0;
        loop {
            let byte = self.byte()?;
            value |= u32::from(byte & 0x7f) << shift;
            shift += 7;
            if byte & 0x80 == 0 || shift >= 35 {
                return Ok(value);
            }
        }
    }

    // Kiwi's zigzag on 32 bits: the decoded bits are reinterpreted, so the casts are intended.
    #[allow(clippy::cast_possible_wrap)]
    pub(super) fn var_int(&mut self) -> Result<i32> {
        let v = self.var_uint()?;
        Ok(((v >> 1) as i32) ^ -((v & 1) as i32))
    }

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

    #[allow(clippy::cast_possible_wrap)]
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
        let start = self.pos;
        let len = self.data[start..]
            .iter()
            .position(|&b| b == 0)
            .ok_or(KiwiError::Eof(self.data.len()))?;
        let s = std::str::from_utf8(&self.data[start..start + len])
            .map_err(|_| KiwiError::Utf8(start))?;
        self.pos = start + len + 1;
        Ok(s.to_owned())
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
