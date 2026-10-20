// SPDX-License-Identifier: AGPL-3.0-or-later

//! Schema-driven decoder for the [Kiwi](https://github.com/evanw/kiwi) binary format.
//!
//! A `.fig` file carries its own Kiwi schema, so the decoder is generic: it reads the binary schema,
//! then decodes the message into a dynamic [`Value`] tree.

use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum KiwiError {
    #[error("unexpected end of data at byte {0}")]
    Eof(usize),
    #[error("invalid UTF-8 string at byte {0}")]
    Utf8(usize),
    #[error("unknown definition kind {0}")]
    UnknownKind(u8),
    #[error("unknown type index {0}")]
    UnknownType(i32),
    #[error("no definition named {0:?}")]
    UnknownDefinition(String),
    #[error("field id {id} does not exist in message {message:?}")]
    UnknownField { message: String, id: u32 },
}

type Result<T> = std::result::Result<T, KiwiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Enum,
    Struct,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    Byte,
    Int,
    Uint,
    Float,
    String,
    Int64,
    Uint64,
    Definition(usize),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: Arc<str>,
    /// `None` on enum members, which only carry a `value`.
    pub field_type: Option<FieldType>,
    pub is_array: bool,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub name: Arc<str>,
    pub kind: Kind,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Schema {
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Byte(u8),
    Int(i32),
    Uint(u32),
    Float(f32),
    String(String),
    Int64(i64),
    Uint64(u64),
    Bytes(Vec<u8>),
    /// Enum member name, or its raw number when the schema does not list it.
    Enum(Arc<str>),
    UnknownEnum(u32),
    Array(Vec<Value>),
    Object(Vec<(Arc<str>, Value)>),
}

impl Value {
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            Value::Enum(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::Uint(u) => Some(*u),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> &[Value] {
        match self {
            Value::Array(items) => items,
            _ => &[],
        }
    }
}

impl Schema {
    /// Decodes a binary Kiwi schema (the decompressed chunk 0 of `canvas.fig`).
    ///
    /// # Errors
    ///
    /// Fails on truncated data or on a definition that refers to an unknown type.
    pub fn decode(bytes: &[u8]) -> Result<Schema> {
        let mut r = Reader::new(bytes);
        let count = r.var_uint()?;
        let mut definitions = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let name: Arc<str> = r.string()?.into();
            let kind = match r.byte()? {
                0 => Kind::Enum,
                1 => Kind::Struct,
                2 => Kind::Message,
                k => return Err(KiwiError::UnknownKind(k)),
            };
            let field_count = r.var_uint()?;
            let mut fields = Vec::with_capacity(field_count as usize);
            for _ in 0..field_count {
                let name: Arc<str> = r.string()?.into();
                let raw_type = r.var_int()?;
                let is_array = r.byte()? & 1 == 1;
                let value = r.var_uint()?;
                let field_type = (kind != Kind::Enum).then_some(raw_type);
                fields.push((name, field_type, is_array, value));
            }
            definitions.push((name, kind, fields));
        }

        let count = definitions.len();
        let definitions = definitions
            .into_iter()
            .map(|(name, kind, fields)| {
                let fields = fields
                    .into_iter()
                    .map(|(name, raw_type, is_array, value)| {
                        let field_type = raw_type.map(|t| resolve_type(t, count)).transpose()?;
                        Ok(Field {
                            name,
                            field_type,
                            is_array,
                            value,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Definition { name, kind, fields })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Schema { definitions })
    }

    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.definitions.iter().position(|d| &*d.name == name)
    }

    /// Decodes `bytes` as an instance of the definition named `root`.
    ///
    /// # Errors
    ///
    /// Fails on truncated data, or when the data does not match the schema.
    pub fn decode_root(&self, root: &str, bytes: &[u8]) -> Result<Value> {
        let index = self
            .find(root)
            .ok_or_else(|| KiwiError::UnknownDefinition(root.to_owned()))?;
        self.decode_definition(index, &mut Reader::new(bytes))
    }

    fn decode_definition(&self, index: usize, r: &mut Reader<'_>) -> Result<Value> {
        let def = &self.definitions[index];
        match def.kind {
            Kind::Enum => {
                let raw = r.var_uint()?;
                Ok(def
                    .fields
                    .iter()
                    .find(|f| f.value == raw)
                    .map_or(Value::UnknownEnum(raw), |f| Value::Enum(f.name.clone())))
            }
            Kind::Struct => {
                let mut out = Vec::with_capacity(def.fields.len());
                for field in &def.fields {
                    out.push((field.name.clone(), self.decode_field(field, r)?));
                }
                Ok(Value::Object(out))
            }
            Kind::Message => {
                let mut out = Vec::new();
                loop {
                    let id = r.var_uint()?;
                    if id == 0 {
                        return Ok(Value::Object(out));
                    }
                    let field = def.fields.iter().find(|f| f.value == id).ok_or_else(|| {
                        KiwiError::UnknownField {
                            message: def.name.to_string(),
                            id,
                        }
                    })?;
                    out.push((field.name.clone(), self.decode_field(field, r)?));
                }
            }
        }
    }

    fn decode_field(&self, field: &Field, r: &mut Reader<'_>) -> Result<Value> {
        // Enum members have no type; a struct or message field always has one.
        let field_type = field.field_type.unwrap_or(FieldType::Uint);
        if !field.is_array {
            return self.decode_value(field_type, r);
        }
        if field_type == FieldType::Byte {
            let len = r.var_uint()? as usize;
            return Ok(Value::Bytes(r.take(len)?.to_vec()));
        }
        let len = r.var_uint()? as usize;
        let mut items = Vec::with_capacity(len.min(r.remaining()));
        for _ in 0..len {
            items.push(self.decode_value(field_type, r)?);
        }
        Ok(Value::Array(items))
    }

    fn decode_value(&self, field_type: FieldType, r: &mut Reader<'_>) -> Result<Value> {
        Ok(match field_type {
            FieldType::Bool => Value::Bool(r.byte()? != 0),
            FieldType::Byte => Value::Byte(r.byte()?),
            FieldType::Int => Value::Int(r.var_int()?),
            FieldType::Uint => Value::Uint(r.var_uint()?),
            FieldType::Float => Value::Float(r.var_float()?),
            FieldType::String => Value::String(r.string()?),
            FieldType::Int64 => Value::Int64(r.var_int64()?),
            FieldType::Uint64 => Value::Uint64(r.var_uint64()?),
            FieldType::Definition(index) => self.decode_definition(index, r)?,
        })
    }
}

fn resolve_type(raw: i32, definition_count: usize) -> Result<FieldType> {
    Ok(match raw {
        -1 => FieldType::Bool,
        -2 => FieldType::Byte,
        -3 => FieldType::Int,
        -4 => FieldType::Uint,
        -5 => FieldType::Float,
        -6 => FieldType::String,
        -7 => FieldType::Int64,
        -8 => FieldType::Uint64,
        // Non-negative after the match guard, so the cast is lossless.
        #[allow(clippy::cast_sign_loss)]
        t if t >= 0 && (t as usize) < definition_count => FieldType::Definition(t as usize),
        t => return Err(KiwiError::UnknownType(t)),
    })
}

// TODO(helpers4): move the cursor and the LEB128/zigzag readers to the helpers4 `binary` module.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn byte(&mut self) -> Result<u8> {
        let b = *self.data.get(self.pos).ok_or(KiwiError::Eof(self.pos))?;
        self.pos += 1;
        Ok(b)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(len).ok_or(KiwiError::Eof(self.pos))?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or(KiwiError::Eof(self.pos))?;
        self.pos = end;
        Ok(slice)
    }

    fn var_uint(&mut self) -> Result<u32> {
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
    fn var_int(&mut self) -> Result<i32> {
        let v = self.var_uint()?;
        Ok(((v >> 1) as i32) ^ -((v & 1) as i32))
    }

    fn var_uint64(&mut self) -> Result<u64> {
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
    fn var_int64(&mut self) -> Result<i64> {
        let v = self.var_uint64()?;
        Ok(((v >> 1) as i64) ^ -((v & 1) as i64))
    }

    /// Kiwi stores floats with the exponent moved to the low bits, and a single `0` byte for zero.
    fn var_float(&mut self) -> Result<f32> {
        let first = self.byte()?;
        if first == 0 {
            return Ok(0.0);
        }
        let rest = self.take(3)?;
        let bits = u32::from_le_bytes([first, rest[0], rest[1], rest[2]]);
        Ok(f32::from_bits(bits.rotate_left(23)))
    }

    fn string(&mut self) -> Result<String> {
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
