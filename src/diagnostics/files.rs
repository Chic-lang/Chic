use std::path::{Path, PathBuf};

/// Identifier for source files used when formatting diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub usize);

impl FileId {
    pub const UNKNOWN: Self = FileId(usize::MAX);
}

impl Default for FileId {
    fn default() -> Self {
        FileId::UNKNOWN
    }
}

/// Captured line/column information (1-based).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub id: FileId,
    pub path: PathBuf,
    pub source: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    #[must_use]
    pub fn new(id: FileId, path: PathBuf, source: String) -> Self {
        let line_starts = compute_line_starts(&source);
        Self {
            id,
            path,
            source,
            line_starts,
        }
    }

    #[must_use]
    pub fn line_col(&self, offset: usize) -> Option<LineCol> {
        if offset > self.source.len() {
            return None;
        }
        let index = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_start = *self.line_starts.get(index)?;
        Some(LineCol {
            line: index + 1,
            column: offset.saturating_sub(line_start) + 1,
        })
    }

    #[must_use]
    pub fn line(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line.saturating_sub(1))?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.source.len());
        self.source.get(start..end)
    }

    /// Start and end byte offsets (exclusive) for a line.
    #[must_use]
    pub fn line_bounds(&self, line: usize) -> Option<(usize, usize)> {
        let start = *self.line_starts.get(line.saturating_sub(1))?;
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.source.len());
        Some((start, end))
    }

    #[must_use]
    pub fn line_range_containing(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        let start_line = self.line_col(start)?.line;
        let end_line = self.line_col(end)?.line;
        Some((start_line, end_line))
    }

    pub(crate) fn update_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
        self.line_starts = compute_line_starts(&self.source);
    }
}

/// Collection of source files used by diagnostics.
#[derive(Clone, Debug, Default)]
pub struct FileCache {
    files: Vec<SourceFile>,
}

impl FileCache {
    pub fn add_file(&mut self, path: impl Into<PathBuf>, source: impl Into<String>) -> FileId {
        let id = FileId(self.files.len());
        let file = SourceFile::new(id, path.into(), source.into());
        self.files.push(file);
        id
    }

    #[must_use]
    pub fn get(&self, file_id: FileId) -> Option<&SourceFile> {
        self.files.get(file_id.0)
    }

    pub fn get_mut(&mut self, file_id: FileId) -> Option<&mut SourceFile> {
        self.files.get_mut(file_id.0)
    }

    pub fn update_source(&mut self, file_id: FileId, source: impl Into<String>) {
        if let Some(file) = self.files.get_mut(file_id.0) {
            file.update_source(source);
        }
    }

    #[must_use]
    pub fn path(&self, file_id: FileId) -> Option<&Path> {
        self.get(file_id).map(|file| file.path.as_path())
    }

    pub fn update_path(&mut self, file_id: FileId, path: impl Into<PathBuf>) {
        if let Some(file) = self.files.get_mut(file_id.0) {
            file.path = path.into();
        }
    }

    #[must_use]
    pub fn line_col(&self, file_id: FileId, offset: usize) -> Option<LineCol> {
        self.get(file_id).and_then(|file| file.line_col(offset))
    }

    #[must_use]
    pub fn find_id_by_path(&self, path: &Path) -> Option<FileId> {
        self.files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.id)
    }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(source.lines().count() + 1);
    starts.push(0);
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            starts.push(idx + ch.len_utf8());
        }
    }
    starts
}
