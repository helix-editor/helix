use std::any::{type_name, Any};
use std::error::Error;
use std::fmt::Debug;
use std::marker::PhantomData;

use anyhow::{bail, ensure, Result};

use crate::any::ConfigData;
use crate::Value;

pub trait Validator: 'static + Debug + Send + Sync {
    fn validate(&self, val: Value) -> Result<ConfigData>;
}

pub trait Ty: Sized + Clone + 'static {
    fn from_value(val: Value) -> Result<Self>;
    fn to_value(&self) -> Value;
}

#[derive(Clone, Copy)]
pub struct IntegerRangeValidator<T> {
    pub min: isize,
    pub max: isize,
    ty: PhantomData<T>,
}
impl<E, T> IntegerRangeValidator<T>
where
    E: Debug,
    T: TryInto<isize, Error = E>,
{
    pub fn new(min: T, max: T) -> Self {
        Self {
            min: min.try_into().unwrap(),
            max: max.try_into().unwrap(),
            ty: PhantomData,
        }
    }
}

impl<T> Debug for IntegerRangeValidator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntegerRangeValidator")
            .field("min", &self.min)
            .field("max", &self.max)
            .field("ty", &type_name::<T>())
            .finish()
    }
}

impl<E, T> IntegerRangeValidator<T>
where
    E: Error + Sync + Send + 'static,
    T: Any + TryFrom<isize, Error = E>,
{
    pub fn validate(&self, val: Value) -> Result<T> {
        let IntegerRangeValidator { min, max, .. } = *self;
        let Value::Int(val) = val else {
            bail!("expected an integer")
        };
        ensure!(
            min <= val && val <= max,
            "expected an integer between {min} and {max} (got {val})",
        );
        Ok(T::try_from(val)?)
    }
}
impl<E, T> Validator for IntegerRangeValidator<T>
where
    E: Error + Sync + Send + 'static,
    T: Any + TryFrom<isize, Error = E> + Send + Sync,
{
    fn validate(&self, val: Value) -> Result<ConfigData> {
        Ok(ConfigData::new(self.validate(val)))
    }
}

macro_rules! integer_tys {
    ($($ty: ident),*) => {
        $(
            impl Ty for $ty {
                fn to_value(&self) -> Value {
                    Value::Int((*self).try_into().unwrap())
                }

                fn from_value(val: Value) -> Result<Self> {
                    IntegerRangeValidator::new($ty::MIN, $ty::MAX).validate(val)
                }
            }
        )*

    };
}

integer_tys! {
    i8, i16, i32, isize,
    u8, u16, u32
}

impl Ty for usize {
    fn to_value(&self) -> Value {
        Value::Int((*self).try_into().unwrap())
    }

    fn from_value(val: Value) -> Result<Self> {
        IntegerRangeValidator::new(0usize, isize::MAX as usize).validate(val)
    }
}

impl Ty for u64 {
    fn to_value(&self) -> Value {
        Value::Int((*self).try_into().unwrap())
    }

    fn from_value(val: Value) -> Result<Self> {
        IntegerRangeValidator::new(0u64, isize::MAX as u64).validate(val)
    }
}

impl Ty for bool {
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_value(val: Value) -> Result<Self> {
        let Value::Bool(val) = val else {
            bail!("expected a boolean")
        };
        Ok(val)
    }
}

impl Ty for Box<str> {
    fn to_value(&self) -> Value {
        Value::String(self.clone().into_string())
    }
    fn from_value(val: Value) -> Result<Self> {
        let Value::String(val) = val else {
            bail!("expected a string")
        };
        Ok(val.into_boxed_str())
    }
}

impl Ty for char {
    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }

    fn from_value(val: Value) -> Result<Self> {
        let Value::String(val) = val else {
            bail!("expected a string")
        };
        ensure!(
            val.chars().count() == 1,
            "expecet a single character (got {val:?})"
        );
        Ok(val.chars().next().unwrap())
    }
}

impl Ty for std::string::String {
    fn to_value(&self) -> Value {
        Value::String(self.clone())
    }
    fn from_value(val: Value) -> Result<Self> {
        let Value::String(val) = val else {
            bail!("expected a string")
        };
        Ok(val)
    }
}

impl<T: Ty> Ty for Option<T> {
    fn to_value(&self) -> Value {
        match self {
            Some(val) => val.to_value(),
            None => Value::Null,
        }
    }

    fn from_value(val: Value) -> Result<Self> {
        if val == Value::Null {
            return Ok(None);
        }
        Ok(Some(T::from_value(val)?))
    }
}

impl<T: Ty> Ty for Box<T> {
    fn from_value(val: Value) -> Result<Self> {
        Ok(Box::new(T::from_value(val)?))
    }

    fn to_value(&self) -> Value {
        T::to_value(self)
    }
}

impl<T: Ty> Ty for indexmap::IndexMap<Box<str>, T, ahash::RandomState> {
    fn from_value(val: Value) -> Result<Self> {
        let Value::Map(map) = val else {
            bail!("expected a map");
        };
        map.into_iter()
            .map(|(k, v)| Ok((k, T::from_value(v)?)))
            .collect()
    }

    fn to_value(&self) -> Value {
        let map = self
            .iter()
            .map(|(k, v)| (k.clone(), v.to_value()))
            .collect();
        Value::Map(Box::new(map))
    }
}

impl<T: Ty> Ty for Box<[T]> {
    fn to_value(&self) -> Value {
        Value::List(self.iter().map(T::to_value).collect())
    }
    fn from_value(val: Value) -> Result<Self> {
        let Value::List(val) = val else {
            bail!("expected a list")
        };
        val.iter().cloned().map(T::from_value).collect()
    }
}

impl Ty for serde_json::Value {
    fn from_value(val: Value) -> Result<Self> {
        Ok(val.into())
    }

    fn to_value(&self) -> Value {
        self.into()
    }
}

pub(super) struct StaticValidator<T: Ty> {
    pub(super) ty: PhantomData<fn(&T)>,
}

impl<T: Ty> Debug for StaticValidator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticValidator")
            .field("ty", &type_name::<T>())
            .finish()
    }
}

impl<T: Ty> Validator for StaticValidator<T> {
    fn validate(&self, val: Value) -> Result<ConfigData> {
        let val = <T as Ty>::from_value(val)?;
        Ok(ConfigData::new(val))
    }
}

pub struct TyValidator<F, T: Ty> {
    pub(super) ty: PhantomData<fn(&T)>,
    f: F,
}

impl<T: Ty, F> Debug for TyValidator<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TyValidator")
            .field("ty", &type_name::<T>())
            .finish()
    }
}

impl<T, F> Validator for TyValidator<F, T>
where
    T: Ty,
    F: Fn(&T) -> anyhow::Result<()> + 'static + Send + Sync,
{
    fn validate(&self, val: Value) -> Result<ConfigData> {
        let val = <T as Ty>::from_value(val)?;
        (self.f)(&val)?;
        Ok(ConfigData::new(val))
    }
}

pub fn ty_validator<T, F>(f: F) -> impl Validator
where
    T: Ty,
    F: Fn(&T) -> anyhow::Result<()> + 'static + Send + Sync,
{
    TyValidator { ty: PhantomData, f }
}

pub fn regex_str_validator() -> impl Validator {
    ty_validator(|val: &Option<crate::String>| {
        if let Some(regex) = val {
            regex_syntax::parse(regex)?;
        }
        Ok(())
    })
}
