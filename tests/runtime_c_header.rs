#[path = "support/collections_runtime_types.rs"]
mod collections_runtime_types;

use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::mem::{align_of, size_of};
use std::path::PathBuf;
use std::process::Command;

use chic::runtime::{
    ChicArc, ChicRc, ChicStr, ChicString, ChicVec, ChicVecIter, ChicVecView, ChicWeak, ChicWeakRc,
    RegionHandle, ValueConstPtr, ValueMutPtr,
};
use collections_runtime_types::{
    ChicHashMap, ChicHashMapIter, ChicHashSet, ChicHashSetIter, HashMapError, HashSetError,
};

#[test]
fn chic_runtime_header_matches_layout() -> Result<(), Box<dyn Error>> {
    if cfg!(target_env = "msvc") {
        eprintln!("Skipping chic_rt header check on MSVC (__int128 unsupported).");
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let include_dir = manifest_dir.join("runtime").join("include");
    let header = include_dir.join("chic_rt.h");
    assert!(header.exists(), "expected {} to exist", header.display());

    let mut source =
        String::from("#include <stdint.h>\n#include <stddef.h>\n#include \"chic_rt.h\"\n\n");
    write!(
        &mut source,
        "#if CHIC_RT_STRING_INLINE_CAPACITY != {inline_cap}\n#error \"string inline capacity mismatch\"\n#endif\n\n",
        inline_cap = ChicString::INLINE_CAPACITY,
    )?;

    let layouts = [
        ("ChicStr", size_of::<ChicStr>(), align_of::<ChicStr>()),
        (
            "ChicString",
            size_of::<ChicString>(),
            align_of::<ChicString>(),
        ),
        (
            "ValueConstPtr",
            size_of::<ValueConstPtr>(),
            align_of::<ValueConstPtr>(),
        ),
        (
            "ValueMutPtr",
            size_of::<ValueMutPtr>(),
            align_of::<ValueMutPtr>(),
        ),
        (
            "RegionHandle",
            size_of::<RegionHandle>(),
            align_of::<RegionHandle>(),
        ),
        ("ChicVec", size_of::<ChicVec>(), align_of::<ChicVec>()),
        (
            "ChicVecView",
            size_of::<ChicVecView>(),
            align_of::<ChicVecView>(),
        ),
        (
            "ChicVecIter",
            size_of::<ChicVecIter>(),
            align_of::<ChicVecIter>(),
        ),
        (
            "ChicHashSet",
            size_of::<ChicHashSet>(),
            align_of::<ChicHashSet>(),
        ),
        (
            "ChicHashSetIter",
            size_of::<ChicHashSetIter>(),
            align_of::<ChicHashSetIter>(),
        ),
        (
            "ChicHashMap",
            size_of::<ChicHashMap>(),
            align_of::<ChicHashMap>(),
        ),
        (
            "ChicHashMapIter",
            size_of::<ChicHashMapIter>(),
            align_of::<ChicHashMapIter>(),
        ),
        ("ChicArc", size_of::<ChicArc>(), align_of::<ChicArc>()),
        ("ChicWeak", size_of::<ChicWeak>(), align_of::<ChicWeak>()),
        ("ChicRc", size_of::<ChicRc>(), align_of::<ChicRc>()),
        (
            "ChicWeakRc",
            size_of::<ChicWeakRc>(),
            align_of::<ChicWeakRc>(),
        ),
        (
            "HashSetError",
            size_of::<HashSetError>(),
            align_of::<HashSetError>(),
        ),
        (
            "HashMapError",
            size_of::<HashMapError>(),
            align_of::<HashMapError>(),
        ),
    ];
    for (name, size, align) in layouts {
        write!(
            &mut source,
            "_Static_assert(sizeof({name}) == {size}u, \"{name} size mismatch\");\n\
             _Static_assert(_Alignof({name}) == {align}u, \"{name} align mismatch\");\n",
        )?;
    }
    source.push_str("\nint main(void) { return 0; }\n");

    let dir = tempfile::tempdir()?;
    let c_path = dir.path().join("header_check.c");
    std::fs::write(&c_path, source)?;

    let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let status = Command::new(&compiler)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-c")
        .arg(&c_path)
        .arg("-o")
        .arg(dir.path().join("header_check.o"))
        .arg("-I")
        .arg(&include_dir)
        .status()?;
    assert!(
        status.success(),
        "failed to compile chic_rt.h with {compiler}: {status}"
    );

    Ok(())
}
