namespace Std.Traits;
import Std.Runtime;
/// Helpers for working with trait objects at the Chic level.
///
/// The current surface is intentionally small – trait objects are still
/// stabilising and these utilities primarily exist to ensure the standard
/// library exposes a single namespace that future runtime helpers can extend
/// without forcing downstream code to rewrite imports.
public static class Debug
{
    /// Returns a developer-friendly label for a trait object.
    ///
    /// The representation is intentionally lightweight (we do not yet surface
    /// the underlying implementation type) but this keeps `dyn` values usable in
    /// diagnostics and log messages across the toolchain.
    public static string Describe <TTrait >() {
        return StringRuntime.Create();
    }
}
