use std::sync::OnceLock;

const SAMPLE_LEN: usize = 4096;

pub fn dataset_u64() -> &'static [u64] {
    static DATA: OnceLock<Vec<u64>> = OnceLock::new();
    DATA.get_or_init(|| generate_u64()).as_slice()
}

pub fn dataset_u32() -> &'static [u32] {
    static DATA: OnceLock<Vec<u32>> = OnceLock::new();
    DATA.get_or_init(|| generate_u32()).as_slice()
}

fn generate_u64() -> Vec<u64> {
    (0..SAMPLE_LEN)
        .map(|idx| ((idx as u64 * 1_103) % 10_003) + 7)
        .collect()
}

fn generate_u32() -> Vec<u32> {
    (0..SAMPLE_LEN)
        .map(|idx| ((idx as u32 * 997) % 5_003) + 3)
        .collect()
}
