//! Metadata payload schema and builder utilities.

use std::fmt::{self, Write};

use crate::chic_kind::ChicKind;
use crate::runtime_package::RuntimeMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataHeader {
    pub(crate) version: String,
    pub(crate) target_requested: String,
    pub(crate) target_canonical: String,
    pub(crate) kind: ChicKind,
    pub(crate) runtime: Option<RuntimeMetadata>,
}

impl MetadataHeader {
    pub(crate) fn new(
        version: impl Into<String>,
        target_requested: impl Into<String>,
        target_canonical: impl Into<String>,
        kind: ChicKind,
        runtime: Option<RuntimeMetadata>,
    ) -> Self {
        Self {
            version: version.into(),
            target_requested: target_requested.into(),
            target_canonical: target_canonical.into(),
            kind,
            runtime,
        }
    }

    fn write_header(&self, out: &mut String) {
        out.push_str("Chic Metadata\n");
        let _ = writeln!(out, "version={}", self.version);
        let _ = writeln!(out, "target-requested={}", self.target_requested);
        let _ = writeln!(out, "target-canonical={}", self.target_canonical);
        let _ = writeln!(out, "kind={}", self.kind.as_str());
        if let Some(runtime) = &self.runtime {
            let _ = writeln!(out, "runtime={}", runtime.identity);
            let _ = writeln!(out, "runtime-abi={}", runtime.abi);
            let _ = writeln!(out, "runtime-hash={}", runtime.manifest_hash);
        }
    }
}

/// Error surfaced when metadata fragments violate the schema contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SchemaError {
    ContainsNull,
}

/// Builder that validates written fragments and accumulates payload text.
#[derive(Debug, Default)]
pub(crate) struct MetadataWriter {
    body: String,
}

impl MetadataWriter {
    pub(crate) fn push_str(&mut self, fragment: &str) {
        Self::validate(fragment).expect("metadata fragments must not contain NUL bytes");
        self.body.push_str(fragment);
    }

    pub(crate) fn push(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let slice = ch.encode_utf8(&mut buf);
        self.push_str(slice);
    }

    pub(crate) fn into_body(self) -> String {
        self.body
    }

    fn validate(fragment: &str) -> Result<(), SchemaError> {
        if fragment.contains('\0') {
            return Err(SchemaError::ContainsNull);
        }
        Ok(())
    }
}

impl Write for MetadataWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Self::validate(s).map_err(|_| fmt::Error)?;
        self.body.push_str(s);
        Ok(())
    }
}

/// Finalised metadata payload combining header and body.
#[derive(Debug, Clone)]
pub(crate) struct MetadataPayload {
    header: MetadataHeader,
    body: String,
}

impl MetadataPayload {
    pub(crate) fn new(header: MetadataHeader, writer: MetadataWriter) -> Self {
        Self {
            header,
            body: writer.into_body(),
        }
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        let mut payload = String::new();
        self.header.write_header(&mut payload);
        payload.push_str(&self.body);
        if !payload.ends_with('\0') {
            payload.push('\0');
        }
        payload.into_bytes()
    }

    #[cfg(test)]
    pub(crate) fn to_text(&self) -> String {
        let mut payload = String::new();
        self.header.write_header(&mut payload);
        payload.push_str(&self.body);
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_writes_in_expected_order() {
        let header = MetadataHeader::new(
            "1.2.3",
            "host-triple",
            "canonical-triple",
            ChicKind::Executable,
            None,
        );
        let mut out = String::new();
        header.write_header(&mut out);
        assert!(out.starts_with("Chic Metadata\nversion=1.2.3\n"));
        assert!(out.contains("target-requested=host-triple"));
        assert!(out.contains("target-canonical=canonical-triple"));
        assert!(out.contains("kind=executable"));
    }

    #[test]
    fn writer_rejects_nul_bytes() {
        let mut writer = MetadataWriter::default();
        let result = writer.write_str("ok");
        assert!(result.is_ok());
        let err = writer.write_str("bad\0fragment");
        assert!(err.is_err());
    }

    #[test]
    fn payload_appends_null_terminator() {
        let header = MetadataHeader::new("0.0.1", "t1", "t1", ChicKind::StaticLibrary, None);
        let mut writer = MetadataWriter::default();
        let _ = write!(writer, "line=value\n");
        let payload = MetadataPayload::new(header, writer);
        let bytes = payload.clone().into_bytes();
        assert_eq!(bytes.last(), Some(&0));
        assert!(payload.to_text().contains("line=value"));
    }
}
