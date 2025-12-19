use crate::any::ConfigData;
use crate::validator::Ty;
use crate::Value;

pub trait IntoTy: Clone {
    type Ty: Ty;
    fn into_ty(self) -> Self::Ty;
}

impl<T: Ty> IntoTy for T {
    type Ty = Self;

    fn into_ty(self) -> Self::Ty {
        self
    }
}
impl<T: IntoTy> IntoTy for &[T] {
    type Ty = Box<[T::Ty]>;

    fn into_ty(self) -> Self::Ty {
        self.iter().cloned().map(T::into_ty).collect()
    }
}
impl<T: IntoTy, const N: usize> IntoTy for &[T; N] {
    type Ty = Box<[T::Ty]>;

    fn into_ty(self) -> Self::Ty {
        self.iter().cloned().map(T::into_ty).collect()
    }
}

impl IntoTy for &str {
    type Ty = Box<str>;

    fn into_ty(self) -> Self::Ty {
        self.into()
    }
}

pub(super) fn ty_into_value<T: Ty>(val: &ConfigData) -> Value {
    T::to_value(val.get())
}
