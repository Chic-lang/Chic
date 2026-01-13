use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

#[must_use]
pub fn value_const_ptr_from<T>(value: &T) -> ValueConstPtr {
    ValueConstPtr {
        ptr: (value as *const T).cast::<u8>(),
        size: std::mem::size_of::<T>(),
        align: std::mem::align_of::<T>(),
    }
}

#[must_use]
pub fn value_mut_ptr_from<T>(value: &mut T) -> ValueMutPtr {
    ValueMutPtr {
        ptr: (value as *mut T).cast::<u8>(),
        size: std::mem::size_of::<T>(),
        align: std::mem::align_of::<T>(),
    }
}
