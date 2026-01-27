namespace Std.IO.Compression;
import Std.Span;
/// <summary>CRC32 checksum utilities with deterministic hardware dispatch.</summary>
public static class Crc32
{
    private static uint[] _table;
    /// <summary>Computes a CRC32 checksum over the provided data.</summary>
    public static uint Compute(ReadOnlySpan <byte >data) {
        var state = 0xFFFFFFFFu;
        Append(ref state, data);
        return state ^ 0xFFFFFFFFu;
    }
    /// <summary>Appends data to an existing CRC32 state.</summary>
    public static void Append(ref uint state, ReadOnlySpan <byte >data) {
        if (data.Length == 0)
        {
            return;
        }
        state = AppendScalar(state, data, Table);
    }
    private static uint AppendScalar(uint state, ReadOnlySpan <byte >data, uint[] tableArray) {
        let len = data.Length;
        var local = state;
        for (var i = 0usize; i <len; i += 1usize) {
            let idx = CompressionCast.ToUSize((local ^ CompressionCast.ToUInt32(data[i])) & 0xFFu);
            let tableVal = tableArray[CompressionCast.ToInt32(idx)];
            local = (local >> 8) ^ tableVal;
        }
        return local;
    }
    private static uint[] Table {
        get {
            if (_table == null)
            {
                _table = BuildTable();
            }
            return _table;
        }
    }
    private static uint[] BuildTable() {
        var table = new uint[256];
        for (var i = 0; i <256; i += 1) {
            var crc = CompressionCast.ToUInt32(i);
            for (var j = 0; j <8; j += 1) {
                if ( (crc & 1u) != 0u)
                {
                    crc = 0xEDB88320u ^ (crc >> 1);
                }
                else
                {
                    crc >>= 1;
                }
            }
            table[CompressionCast.ToUSize(i)] = crc;
        }
        return table;
    }
}
