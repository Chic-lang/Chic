namespace Std.Gpu.Launch;
import Std.Core;
public static class KernelLaunch
{
    /// <summary>Validate grid/block dimensions before dispatch.</summary>
    public static void Validate(uint gridX, uint gridY, uint gridZ, uint blockX, uint blockY, uint blockZ) {
        if (gridX == 0u || blockX == 0u)
        {
            throw new Std.ArgumentException("grid/block dimensions must be non-zero");
        }
    }
}
