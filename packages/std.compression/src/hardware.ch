namespace Std.IO.Compression;
import Std.Platform;
/// <summary>Compression hardware detection hooks (deterministic, overrideable).</summary>
internal static class CompressionHardware
{
    private static bool _initialized;
    private static bool _useAcceleratedCrc;
    public static bool UseAcceleratedCrc32 {
        get {
            EnsureInitialized();
            return _useAcceleratedCrc;
        }
    }
    private static void EnsureInitialized() {
        if (_initialized)
        {
            return;
        }
        _initialized = true;
        let forceScalar = EnvironmentVariables.Get("STD_COMPRESSION_FORCE_SCALAR");
        if (forceScalar != null && forceScalar == "1")
        {
            _useAcceleratedCrc = false;
            return;
        }
        let forceAccel = EnvironmentVariables.Get("STD_COMPRESSION_FORCE_ACCEL");
        if (forceAccel != null && forceAccel == "1")
        {
            _useAcceleratedCrc = true;
            return;
        }
        // Default: scalar path until ISA-specific probes are implemented.
        _useAcceleratedCrc = false;
    }
}
