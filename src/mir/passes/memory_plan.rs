//! Graph memory planner stub producing non-overlapping buffer offsets.

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferPlan {
    pub id: String,
    pub size: u64,
    pub offset: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryPlan {
    pub buffers: Vec<BufferPlan>,
}

impl MemoryPlan {
    /// Compute a simple sequential plan given buffer identifiers and sizes.
    #[must_use]
    #[allow(dead_code)]
    pub fn from_sizes(buffers: &[(impl AsRef<str>, u64)]) -> Self {
        let mut plan = MemoryPlan::default();
        let mut cursor = 0u64;
        for (id, size) in buffers {
            let offset = cursor;
            plan.buffers.push(BufferPlan {
                id: id.as_ref().to_string(),
                size: *size,
                offset,
            });
            cursor = cursor.saturating_add(*size);
        }
        plan
    }

    /// Validate that no buffers overlap.
    #[must_use]
    #[allow(dead_code)]
    pub fn validate(&self) -> bool {
        for (idx, lhs) in self.buffers.iter().enumerate() {
            let lhs_end = lhs.offset + lhs.size;
            for rhs in self.buffers.iter().skip(idx + 1) {
                let rhs_end = rhs.offset + rhs.size;
                if lhs.offset < rhs_end && rhs.offset < lhs_end {
                    return false;
                }
            }
        }
        true
    }
}
