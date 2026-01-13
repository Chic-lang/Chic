pub mod resolver;
pub mod version;

pub use resolver::{
    ResolveOptions, ResolveOutcome, ResolvedPackage, ResolvedSource, resolve_dependencies,
};
pub use version::{Version, VersionParseError, VersionReq, VersionReqError};
