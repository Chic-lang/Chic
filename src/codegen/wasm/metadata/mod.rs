use crate::error::Error;

use super::encoding::push_string;

/// Construct the metadata payload bytes used inside the custom section.
pub(super) fn build_metadata_payload(kind_label: &str) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::new();
    push_string(&mut payload, "chic.metadata")?;
    let descriptor = format!("target=wasm32;kind={kind_label}");
    push_string(&mut payload, &descriptor)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_metadata_payload_emits_label_and_descriptor() {
        let payload =
            build_metadata_payload("dynamic-library").expect("metadata payload should build");
        // First byte represents the length of the section name (chic.metadata = 15 bytes).
        assert_eq!(payload[0], "chic.metadata".len() as u8);
        assert!(
            payload
                .windows("chic.metadata".len())
                .any(|window| window == b"chic.metadata"),
            "payload should contain the metadata section label"
        );
        assert!(
            payload
                .windows("target=wasm32;kind=dynamic-library".len())
                .any(|window| window == b"target=wasm32;kind=dynamic-library"),
            "payload should embed the descriptor string"
        );
    }

    #[test]
    fn build_metadata_payload_uses_length_prefix_for_descriptor() {
        let payload = build_metadata_payload("executable").expect("metadata payload should build");
        let descriptor = "target=wasm32;kind=executable";
        // The descriptor length byte follows immediately after the label and its contents.
        let descriptor_index = 1 + "chic.metadata".len();
        assert_eq!(
            payload[descriptor_index],
            descriptor.len() as u8,
            "descriptor should be length-prefixed"
        );
        assert!(
            payload
                .windows(descriptor.len())
                .any(|window| window == descriptor.as_bytes()),
            "descriptor bytes should appear in the payload"
        );
    }
}
