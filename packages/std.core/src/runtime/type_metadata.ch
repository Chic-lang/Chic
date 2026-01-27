namespace Std.Runtime;
import Std.Core;
import Std.Core.Testing;
public enum TypeMetadataStatus
{
    Success = 0, NotFound = 1, InvalidPointer = 2,
}
public enum RuntimeGenericVariance
{
    Invariant = 0, Covariant = 1, Contravariant = 2,
}
@repr(c) public struct VarianceSlice
{
    public * const @readonly @expose_address RuntimeGenericVariance Ptr;
    public usize Len;
}
@repr(c) public struct TypeMetadataRecord
{
    public usize Size;
    public usize Align;
    public isize DropFn;
    public VarianceSlice Variance;
    public uint Flags;
    public init(usize size, usize align, isize dropFn) {
        Size = size;
        Align = align;
        DropFn = dropFn;
        Variance = new VarianceSlice {
            Ptr = null, Len = 0usize
        }
        ;
        Flags = 0u;
    }
}
internal static class TypeMetadataIntrinsics
{
    @extern("C") public static extern TypeMetadataStatus chic_rt_type_metadata(ulong typeId, ref TypeMetadataRecord metadata);
    @extern("C") public static extern usize chic_rt_type_size(ulong typeId);
    @extern("C") public static extern usize chic_rt_type_align(ulong typeId);
    @extern("C") public static extern isize chic_rt_type_drop_glue(ulong typeId);
    @extern("C") public static extern isize chic_rt_type_clone_glue(ulong typeId);
    @extern("C") public static extern isize chic_rt_type_hash_glue(ulong typeId);
    @extern("C") public static extern isize chic_rt_type_eq_glue(ulong typeId);
}
public static class TypeMetadata
{
    public static bool TryGet(ulong typeId, out TypeMetadataRecord metadata) {
        var record = new TypeMetadataRecord(0, 0, 0);
        let status = TypeMetadataIntrinsics.chic_rt_type_metadata(typeId, ref record);
        metadata = record;
        return status == TypeMetadataStatus.Success;
    }
    public static TypeMetadataRecord Resolve(ulong typeId) {
        var metadata = new TypeMetadataRecord(0, 0, 0);
        TypeMetadataIntrinsics.chic_rt_type_metadata(typeId, ref metadata);
        return metadata;
    }
    public static TypeMetadataRecord Resolve <T >() {
        let typeId = __type_id_of <T >();
        return Resolve(typeId);
    }
}
testcase Given_type_metadata_resolve_size_matches_When_executed_Then_type_metadata_resolve_size_matches()
{
    let meta = TypeMetadata.Resolve <int >();
    Assert.That(meta.Size == __sizeof <int >()).IsTrue();
}
testcase Given_type_metadata_resolve_align_matches_When_executed_Then_type_metadata_resolve_align_matches()
{
    let meta = TypeMetadata.Resolve <int >();
    Assert.That(meta.Align == __alignof <int >()).IsTrue();
}
testcase Given_type_metadata_try_get_returns_true_When_executed_Then_type_metadata_try_get_returns_true()
{
    let id = __type_id_of <int >();
    var meta = new TypeMetadataRecord(0usize, 0usize, 0isize);
    let ok = TypeMetadata.TryGet(id, out meta);
    let _ = meta;
    Assert.That(ok).IsTrue();
}
testcase Given_type_metadata_try_get_size_matches_When_executed_Then_type_metadata_try_get_size_matches()
{
    let id = __type_id_of <int >();
    var meta = new TypeMetadataRecord(0usize, 0usize, 0isize);
    let _ = TypeMetadata.TryGet(id, out meta);
    Assert.That(meta.Size == __sizeof <int >()).IsTrue();
}
testcase Given_type_metadata_try_get_missing_returns_false_When_executed_Then_type_metadata_try_get_missing_returns_false()
{
    var meta = new TypeMetadataRecord(0usize, 0usize, 0isize);
    let ok = TypeMetadata.TryGet(0ul, out meta);
    let _ = meta;
    Assert.That(ok).IsFalse();
}
testcase Given_type_metadata_try_get_missing_size_zero_When_executed_Then_type_metadata_try_get_missing_size_zero()
{
    var meta = new TypeMetadataRecord(0usize, 0usize, 0isize);
    let _ = TypeMetadata.TryGet(0ul, out meta);
    Assert.That(meta.Size == 0usize).IsTrue();
}
