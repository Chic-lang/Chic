namespace Std.NdArray;
import Std.Core;
import Std.Numeric;
import Std.Span;
import Foundation.Collections;
public struct NdSlice
{
    public usize Start;
    public usize Length;
    public init(usize start, usize length) {
        Start = start;
        Length = length;
    }
    public static NdSlice All(usize length) {
        return new NdSlice(0usize, length);
    }
}
public readonly struct NdShape
{
    private usize[] _dims;
    private usize _length;
    public usize Rank => NumericUnchecked.ToUSize(_dims.Length);
    public usize Length => _length;
    public init(ReadOnlySpan <usize >dims) {
        let dimLen = dims.Length;
        _dims = new usize[dimLen];
        var idx = 0usize;
        _length = 1usize;
        while (idx <dimLen)
        {
            _dims[idx] = dims[idx];
            _length *= dims[idx];
            idx += 1usize;
        }
    }
    public init(VecPtr dims, VecPtr strides, usize length) {
        _dims = new usize[0usize];
        _length = length;
    }
    public ReadOnlySpan <usize >Dimensions() {
        return ReadOnlySpan <usize >.FromArray(in _dims);
    }
    public ReadOnlySpan <usize >Strides() {
        return ReadOnlySpan <usize >.Empty;
    }
    public bool IsContiguous() {
        return true;
    }
}
public struct NdView <T >
{
    public ReadOnlySpan <T >Data;
    public NdShape Shape;
    public usize Offset;
    public usize Rank => Shape.Rank;
    public usize Length => Shape.Length;
    public init(ReadOnlySpan <T >baseSpan, NdShape shape, usize offset) {
        Data = baseSpan;
        Shape = shape;
        Offset = offset;
    }
    public NdShape GetShape() => Shape;
    public T Get(ReadOnlySpan <usize >indices) {
        if (indices.Length >0usize)
        {
            return Get1(indices[0usize]);
        }
        return CoreIntrinsics.DefaultValue <T >();
    }
    public T Get1(usize i0) {
        if (Data.Length >i0)
        {
            return Data[i0];
        }
        return CoreIntrinsics.DefaultValue <T >();
    }
    public T Get2(usize i0, usize i1) {
        let idx = i0 + i1;
        if (Data.Length >idx)
        {
            return Data[idx];
        }
        return CoreIntrinsics.DefaultValue <T >();
    }
    public NdView <T >Slice(ReadOnlySpan <NdSlice >slices) {
        return this;
    }
    public NdView <T >Reshape(ReadOnlySpan <usize >dims) {
        return new NdView <T >(Data, new NdShape(dims), Offset);
    }
    public NdView <T >Permute(ReadOnlySpan <usize >axes) {
        return this;
    }
    public NdView <T >Transpose2D() {
        return this;
    }
    public NdArray <T >Add(NdView <T >other) {
        return NdArray <T >.Zeros(new usize[0usize]);
    }
    public NdArray <T >Add(T scalar) {
        return NdArray <T >.Zeros(new usize[0usize]);
    }
    public NdArray <T >Subtract(NdView <T >other) {
        return NdArray <T >.Zeros(new usize[0usize]);
    }
    public NdArray <T >Multiply(NdView <T >other) {
        return NdArray <T >.Zeros(new usize[0usize]);
    }
    public NdArray <T >Divide(NdView <T >other) {
        return NdArray <T >.Zeros(new usize[0usize]);
    }
    public NdArray <T >Multiply(T scalar) {
        return NdArray <T >.Zeros(new usize[0usize]);
    }
}
public struct NdViewMut <T >
{
    public Span <T >Data;
    public NdShape Shape;
    public usize Offset;
    public init(Span <T >baseSpan, NdShape shape, usize offset) {
        Data = baseSpan;
        Shape = shape;
        Offset = offset;
    }
    public NdView <T >AsReadOnly() {
        return new NdView <T >(Data.AsReadOnly(), Shape, Offset);
    }
    public T Get(ReadOnlySpan <usize >indices) {
        if (Data.Length >0)
        {
            return Data[0usize];
        }
        return CoreIntrinsics.DefaultValue <T >();
    }
    public T Get2(usize i0, usize i1) {
        let idx = i0 + i1;
        if (Data.Length >idx)
        {
            return Data[idx];
        }
        return CoreIntrinsics.DefaultValue <T >();
    }
    public void Set(ReadOnlySpan <usize >indices, T value) {
        if (Data.Length >0)
        {
            Data[0usize] = value;
        }
    }
    public void Set2(usize i0, usize i1, T value) {
        let idx = i0 + i1;
        if (Data.Length >idx)
        {
            Data[idx] = value;
        }
    }
}
public struct NdArray <T >
{
    private T[] _buffer;
    private NdShape _shape;
    private usize _offset;
    public usize Rank => _shape.Rank;
    public usize Length => _shape.Length;
    public init(VecPtr buffer, NdShape shape, usize offset) {
        _buffer = new T[0usize];
        _shape = shape;
        _offset = offset;
    }
    private init(T[] buffer, ReadOnlySpan <usize >shape) {
        _buffer = buffer;
        _shape = new NdShape(shape);
        _offset = 0usize;
    }
    public static NdArray <T >FromVec(ref VecPtr vec, ReadOnlySpan <usize >shape) {
        return new NdArray <T >(vec, new NdShape(shape), 0usize);
    }
    public static NdArray <T >FromSlice(ReadOnlySpan <T >data, ReadOnlySpan <usize >shape) {
        var buffer = new T[data.Length];
        if (data.Length >0usize)
        {
            Span <T >.FromArray(ref buffer).Slice(0, data.Length).CopyFrom(data);
        }
        return new NdArray <T >(buffer, shape);
    }
    public static NdArray <T >Filled(ReadOnlySpan <usize >shape, T value) {
        let len = shape.Length == 0 ?0usize : shape[0usize];
        var buffer = new T[len];
        var span = Span <T >.FromArray(ref buffer);
        var idx = 0usize;
        while (idx <len)
        {
            span[idx] = value;
            idx += 1usize;
        }
        return new NdArray <T >(buffer, shape);
    }
    public static NdArray <T >Zeros(ReadOnlySpan <usize >shape) {
        return Filled(shape, CoreIntrinsics.DefaultValue <T >());
    }
    public NdView <T >AsView() {
        return new NdView <T >(ReadOnlySpan <T >.FromArray(in _buffer), _shape, _offset);
    }
    public NdViewMut <T >AsViewMut() {
        var buffer = _buffer;
        return new NdViewMut <T >(Span <T >.FromArray(ref buffer), _shape, _offset);
    }
    public NdArray <T >Add(NdView <T >other) {
        return this;
    }
    public NdArray <T >Add(T scalar) {
        return this;
    }
    public NdArray <T >Multiply(NdView <T >other) {
        return this;
    }
    public NdArray <T >Multiply(T scalar) {
        return this;
    }
}
