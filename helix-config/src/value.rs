use std::fmt::Display;

use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde::ser::{Error as _, Impossible};
use serde::{Deserialize, Serialize};
use serde_json::{Error, Result};

use crate::Ty;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    List(Vec<Value>),
    Map(Box<IndexMap<Box<str>, Value, ahash::RandomState>>),
    Int(isize),
    Float(f64),
    Bool(bool),
    String(String),
    Null,
}

impl Value {
    pub fn typed<T: Ty>(self) -> anyhow::Result<T> {
        T::from_value(self)
    }

    pub fn append(&mut self, val: Value, depth: usize) {
        match (self, val) {
            (Value::List(dst), Value::List(ref mut val)) => dst.append(val),
            (Value::Map(dst), Value::Map(val)) if depth == 0 || dst.is_empty() => {
                dst.extend(val.into_iter())
            }
            (Value::Map(dst), Value::Map(val)) => {
                dst.reserve(val.len());
                for (k, v) in val.into_iter() {
                    // we don't use the entry api because we want
                    // to maintain the ordering
                    let merged = match dst.shift_remove(&k) {
                        Some(mut old) => {
                            old.append(v, depth - 1);
                            old
                        }
                        None => v,
                    };
                    dst.insert(k, merged);
                }
            }
            (dst, val) => *dst = val,
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_owned())
    }
}

macro_rules! from_int {
    ($($ty: ident),*) => {
        $(
            impl From<$ty> for Value {
                fn from(value: $ty) -> Self {
                    Value::Int(value.try_into().unwrap())
                }
            }
        )*
    };
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        to_value(value).unwrap()
    }
}
impl From<&serde_json::Value> for Value {
    fn from(value: &serde_json::Value) -> Self {
        to_value(value).unwrap()
    }
}

impl From<Value> for serde_json::Value {
    fn from(value: Value) -> Self {
        serde_json::to_value(value).unwrap()
    }
}

from_int!(isize, usize, u32, i32, i16, u16, i8, u8);

pub fn to_value<T>(value: T) -> Result<Value>
where
    T: Serialize,
{
    value.serialize(Serializer)
}

pub fn from_value<T>(value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    // roundtripping trough json is very inefficient *and incorrect* (captures
    // json semantics that don't apply to us)
    // TODO: implement a custom deserializer just like serde_json does
    serde_json::from_value(value.into())
}

// We only use our own error type; no need for From conversions provided by the
// standard library's try! macro. This reduces lines of LLVM IR by 4%.
macro_rules! tri {
    ($e:expr $(,)?) => {
        match $e {
            core::result::Result::Ok(val) => val,
            core::result::Result::Err(err) => return core::result::Result::Err(err),
        }
    };
}

pub struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = Impossible<Value, Error>;
    type SerializeMap = SerializeMap;
    type SerializeStruct = SerializeMap;
    type SerializeStructVariant = Impossible<Value, Error>;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Value> {
        Ok(Value::Bool(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Value> {
        self.serialize_i64(value as i64)
    }

    fn serialize_i64(self, value: i64) -> Result<Value> {
        Ok(Value::Int(value.try_into().unwrap()))
    }

    fn serialize_i128(self, _value: i128) -> Result<Value> {
        unreachable!()
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Value> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Value> {
        Ok(Value::Int(value.try_into().unwrap()))
    }

    fn serialize_u128(self, _value: u128) -> Result<Value> {
        unreachable!()
    }

    #[inline]
    fn serialize_f32(self, float: f32) -> Result<Value> {
        Ok(Value::Float(float as f64))
    }

    #[inline]
    fn serialize_f64(self, float: f64) -> Result<Value> {
        Ok(Value::Float(float))
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Value> {
        let mut s = String::new();
        s.push(value);
        Ok(Value::String(s))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Value> {
        Ok(Value::String(value.into()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
        let vec = value.iter().map(|&b| Value::Int(b.into())).collect();
        Ok(Value::List(vec))
    }

    #[inline]
    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        unimplemented!()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    #[inline]
    fn serialize_none(self) -> Result<Value> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        unimplemented!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap {
            map: IndexMap::default(),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        unreachable!()
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        unreachable!()
    }

    fn collect_str<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Display,
    {
        Ok(Value::String(value.to_string()))
    }
}

pub struct SerializeVec {
    vec: Vec<Value>,
}

impl serde::ser::SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.vec.push(tri!(to_value(value)));
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::List(self.vec))
    }
}

impl serde::ser::SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        serde::ser::SerializeSeq::end(self)
    }
}

pub struct SerializeMap {
    map: IndexMap<Box<str>, Value, ahash::RandomState>,
    next_key: Option<Box<str>>,
}

impl serde::ser::SerializeMap for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = to_value(key)?;
        let Value::String(val) = key else {
            return Err(Error::custom("only string keys are supported"));
        };
        self.next_key = Some(val.into_boxed_str());
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self.next_key.take();
        // Panic because this indicates a bug in the program rather than an
        // expected failure.
        let key = key.expect("serialize_value called before serialize_key");
        self.map.insert(key, tri!(to_value(value)));
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::Map(Box::new(self.map)))
    }
}

impl serde::ser::SerializeStruct for SerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Value> {
        serde::ser::SerializeMap::end(self)
    }
}
