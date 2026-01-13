#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; interface defaults live in the native runtime."
);

mod native {
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct InterfaceDefaultDescriptor {
        pub implementer: *const u8,
        pub interface: *const u8,
        pub method: *const u8,
        pub symbol: *const (),
    }

    unsafe extern "C" {
        pub fn chic_rt_install_interface_defaults(
            entries: *const InterfaceDefaultDescriptor,
            len: u64,
        );
        pub fn chic_rt_interface_defaults_ptr() -> *const InterfaceDefaultDescriptor;
        pub fn chic_rt_interface_defaults_len() -> u64;
    }

    pub use InterfaceDefaultDescriptor as InterfaceDefaultRecord;
}

pub use native::*;
