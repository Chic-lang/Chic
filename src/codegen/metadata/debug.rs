//! Target/format helpers for metadata emission.

use object::{Architecture, BinaryFormat, Endianness};
use target_lexicon::{Architecture as TripleArch, OperatingSystem, Triple};

/// Map a canonical triple to the object format, architecture, and endianness.
pub(crate) fn map_triple(triple: &Triple) -> Option<(BinaryFormat, Architecture, Endianness)> {
    let architecture = match triple.architecture {
        TripleArch::X86_64 => Architecture::X86_64,
        TripleArch::Aarch64(_) => Architecture::Aarch64,
        _ => return None,
    };

    let binary_format = match triple.operating_system {
        OperatingSystem::Darwin(_) | OperatingSystem::MacOSX(_) | OperatingSystem::IOS(_) => {
            BinaryFormat::MachO
        }
        OperatingSystem::Linux
        | OperatingSystem::Unknown
        | OperatingSystem::None_
        | OperatingSystem::Hurd => BinaryFormat::Elf,
        _ => return None,
    };

    let endianness = Endianness::Little;

    Some((binary_format, architecture, endianness))
}

/// Compute the Mach-O build version load command for macOS targets.
pub(crate) fn macos_build_version(os: &OperatingSystem) -> Option<(u32, u32)> {
    match os {
        OperatingSystem::MacOSX(target) | OperatingSystem::Darwin(target) => {
            let encoded = target
                .map(|deploy| {
                    encode_macos_version(
                        u32::from(deploy.major),
                        u32::from(deploy.minor),
                        u32::from(deploy.patch),
                    )
                })
                .unwrap_or_else(|| encode_macos_version(11, 0, 0));
            Some((encoded, encoded))
        }
        _ => None,
    }
}

/// Encode a macOS version triple into the packed representation required by LLVM/object.
pub(crate) fn encode_macos_version(major: u32, minor: u32, patch: u32) -> u32 {
    let major = (major & 0xFFFF).min(0xFFFF);
    let minor = (minor & 0xFF).min(0xFF);
    let patch = (patch & 0xFF).min(0xFF);
    (major << 16) | (minor << 8) | patch
}

/// Select the metadata section name for the requested binary format.
pub(crate) fn section_name_for_format(format: BinaryFormat) -> Vec<u8> {
    match format {
        BinaryFormat::MachO => b"__chxmeta".to_vec(),
        BinaryFormat::Coff => b".chicxmeta".to_vec(),
        _ => b".chic.meta".to_vec(),
    }
}

/// Select the reflection section name for the requested binary format.
pub(crate) fn reflection_section_name_for_format(format: BinaryFormat) -> Vec<u8> {
    match format {
        BinaryFormat::MachO => b"__chxreflect".to_vec(),
        BinaryFormat::Coff => b".chicxreflect".to_vec(),
        _ => b".chic.reflect".to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_version_components() {
        assert_eq!(encode_macos_version(11, 0, 0), 0x000B_0000);
        assert_eq!(encode_macos_version(13, 3, 1), 0x000D_0301);
    }
}
