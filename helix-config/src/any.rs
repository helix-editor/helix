/// this is a reimplementation of dynamic dispatch that only stores the
/// information we need and stores everythin inline. Values that are smaller or
/// the same size as a slice (2 usize) are also stored inline. This avoids
/// significant overallocation when setting lots of simple config
/// options (integers, strings, lists, enums)
use std::any::{Any, TypeId};
use std::mem::{align_of, size_of, MaybeUninit};

pub struct ConfigData {
    data: MaybeUninit<[usize; 2]>,
    ty: TypeId,
    drop_fn: unsafe fn(MaybeUninit<[usize; 2]>),
}

const fn store_inline<T>() -> bool {
    size_of::<T>() <= size_of::<[usize; 2]>() && align_of::<T>() <= align_of::<[usize; 2]>()
}

impl ConfigData {
    unsafe fn drop_impl<T: Any>(mut data: MaybeUninit<[usize; 2]>) {
        if store_inline::<T>() {
            data.as_mut_ptr().cast::<T>().drop_in_place();
        } else {
            let ptr = data.as_mut_ptr().cast::<*mut T>().read();
            drop(Box::from_raw(ptr));
        }
    }

    pub fn get<T: Any>(&self) -> &T {
        assert_eq!(TypeId::of::<T>(), self.ty);
        unsafe {
            if store_inline::<T>() {
                return &*self.data.as_ptr().cast();
            }
            let data: *const T = self.data.as_ptr().cast::<*const T>().read();
            &*data
        }
    }

    /// Gets a cloned copy of the stored value.
    pub fn get_cloned<T: Any + Clone>(&self) -> T {
        self.get::<T>().clone()
    }
    pub fn new<T: Any>(val: T) -> Self {
        let mut data = MaybeUninit::uninit();
        if store_inline::<T>() {
            let data: *mut T = data.as_mut_ptr() as _;
            unsafe {
                data.write(val);
            }
        } else {
            assert!(store_inline::<*const T>());
            let data: *mut *const T = data.as_mut_ptr() as _;
            unsafe {
                data.write(Box::into_raw(Box::new(val)));
            }
        };
        Self {
            data,
            ty: TypeId::of::<T>(),
            drop_fn: ConfigData::drop_impl::<T>,
        }
    }
}

impl Drop for ConfigData {
    fn drop(&mut self) {
        unsafe {
            (self.drop_fn)(self.data);
        }
    }
}

impl std::fmt::Debug for ConfigData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigData").finish_non_exhaustive()
    }
}

unsafe impl Send for ConfigData {}
unsafe impl Sync for ConfigData {}
