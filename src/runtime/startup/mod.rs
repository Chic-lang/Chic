//! Native program startup integration for Chic-generated binaries.
//!
//! This module defines the metadata contract shared between the LLVM backend
//! and the runtime bootstrap. Code generation emits instances of the structs
//! declared here so the runtime can discover the compiled entry point and
//! testcase inventory without bespoke symbol plumbing.

#![allow(unsafe_code)]

pub const STARTUP_DESCRIPTOR_SYMBOL: &str = "__chic_startup_descriptor";
pub const ENTRY_SYMBOL: &str = "__chic_program_main";
pub const STARTUP_DESCRIPTOR_VERSION: u32 = 1;

pub const ENTRY_FLAG_ASYNC: u32 = 0x0000_0001;
pub const ENTRY_FLAG_RET_I32: u32 = 0x0000_0002;
pub const ENTRY_FLAG_RET_BOOL: u32 = 0x0000_0004;
pub const ENTRY_FLAG_RET_VOID: u32 = 0x0000_0008;
pub const ENTRY_FLAG_PARAM_ARGS: u32 = 0x0000_0100;
pub const ENTRY_FLAG_PARAM_ENV: u32 = 0x0000_0200;

pub const TESTCASE_FLAG_ASYNC: u32 = 0x0000_0001;

/// Descriptor emitted by the code generator describing entry points and tests.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StartupDescriptor {
    pub version: u32,
    pub entry: EntryDescriptor,
    pub tests: TestSuiteDescriptor,
}

/// Recorded information about the compiled `Main` entry point.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EntryDescriptor {
    pub function: *const (),
    pub flags: u32,
    pub reserved: u32,
}

/// Collection of compiled testcases bundled into the artifact.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TestSuiteDescriptor {
    pub cases: *const TestCaseDescriptor,
    pub len: usize,
}

/// Metadata describing a single testcase.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TestCaseDescriptor {
    pub function: *const (),
    pub name_ptr: *const u8,
    pub name_len: usize,
    pub flags: u32,
    pub reserved: u32,
}

impl StartupDescriptor {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            version: STARTUP_DESCRIPTOR_VERSION,
            entry: EntryDescriptor::empty(),
            tests: TestSuiteDescriptor::empty(),
        }
    }
}

impl EntryDescriptor {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            function: core::ptr::null(),
            flags: 0,
            reserved: 0,
        }
    }

    #[must_use]
    pub const fn has_flag(self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }
}

impl TestSuiteDescriptor {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            cases: core::ptr::null(),
            len: 0,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0 || self.cases.is_null()
    }

    #[must_use]
    pub unsafe fn as_slice(&self) -> &[TestCaseDescriptor] {
        if self.is_empty() {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.cases, self.len) }
        }
    }
}

impl TestCaseDescriptor {
    #[must_use]
    pub unsafe fn name(&self) -> Option<&str> {
        if self.name_ptr.is_null() || self.name_len == 0 {
            return None;
        }
        let bytes = unsafe { core::slice::from_raw_parts(self.name_ptr, self.name_len) };
        core::str::from_utf8(bytes).ok()
    }

    #[must_use]
    pub const fn is_async(&self) -> bool {
        (self.flags & TESTCASE_FLAG_ASYNC) != 0
    }
}
