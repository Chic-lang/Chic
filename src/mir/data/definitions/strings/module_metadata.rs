use crate::di::DiManifest;
use crate::frontend::diagnostics::Span;

/// Explicit export declared via `@export`.
#[derive(Debug, Clone)]
pub struct Export {
    pub function: String,
    pub symbol: String,
    pub span: Option<Span>,
}

/// Selected global allocator for the module.
#[derive(Debug, Clone)]
pub struct GlobalAllocator {
    pub type_name: String,
    pub target: Option<String>,
    pub span: Option<Span>,
}

/// Native link dependency declared via `@link("<name>")`.
#[derive(Debug, Clone)]
pub struct LinkLibrary {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdProfile {
    Std,
    NoStd,
}

impl StdProfile {
    #[must_use]
    pub fn is_no_std(self) -> bool {
        matches!(self, Self::NoStd)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdProfileSource {
    Default,
    CrateAttribute,
    NamespaceAttribute,
}

/// Module-level behavioural attributes.
#[derive(Debug, Clone)]
pub struct ModuleAttributes {
    pub std_profile: StdProfile,
    pub std_profile_span: Option<Span>,
    pub std_profile_source: StdProfileSource,
    pub global_allocator: Option<GlobalAllocator>,
    pub di_manifest: DiManifest,
    pub suppress_startup_descriptor: bool,
    pub no_main: bool,
    pub no_main_span: Option<Span>,
    pub link_libraries: Vec<LinkLibrary>,
}

impl Default for ModuleAttributes {
    fn default() -> Self {
        Self {
            std_profile: StdProfile::Std,
            std_profile_span: None,
            std_profile_source: StdProfileSource::Default,
            global_allocator: None,
            di_manifest: DiManifest::default(),
            suppress_startup_descriptor: false,
            no_main: false,
            no_main_span: None,
            link_libraries: Vec::new(),
        }
    }
}

impl ModuleAttributes {
    #[must_use]
    pub fn is_no_std(&self) -> bool {
        self.std_profile.is_no_std()
    }

    #[must_use]
    pub fn is_no_main(&self) -> bool {
        self.no_main
    }
}
