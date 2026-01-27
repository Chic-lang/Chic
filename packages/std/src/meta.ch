namespace Std.Meta;
/// Type categories surfaced by `reflect<T>()`.
public enum TypeKind
{
    Struct = 0, Record = 1, Class = 2, Enum = 3, Interface = 4, Union = 5, Extension = 6, Trait = 7, Delegate = 8, Impl = 9, Function = 10, Const = 11, Static = 12,
}
/// Member classifications carried by reflection descriptors.
public enum MemberKind
{
    Field = 0, Property = 1, Method = 2, Constructor = 3, Const = 4, EnumVariant = 5, UnionField = 6, UnionView = 7, AssociatedType = 8, ExtensionMethod = 9, TraitMethod = 10,
}
public enum VisibilityDescriptor
{
    Public = 0, Internal = 1, Protected = 2, Private = 3, ProtectedInternal = 4, PrivateProtected = 5,
}
public enum ParameterMode
{
    In = 0, Ref = 1, Out = 2, Value = 3,
}
public struct TypeHandle
{
    public string Name;
    public Std.Option <ulong >TypeId;
}
public struct AttributeArgument
{
    public Std.Option <string >Name;
    public string Value;
}
public struct AttributeDescriptor
{
    public string Name;
    public DescriptorList <AttributeArgument >PositionalArgs;
    public DescriptorList <AttributeArgument >NamedArgs;
}
public struct LayoutDescriptor
{
    public bool ReprC;
    public Std.Option <uint >Pack;
    public Std.Option <uint >Align;
}
public struct FieldLayoutDescriptor
{
    public string Name;
    public Std.Option <ulong >Offset;
    public Std.Option <TypeHandle >Type;
    public Std.Option <bool >Readonly;
}
public struct TypeLayoutDescriptor
{
    public Std.Option <ulong >Size;
    public Std.Option <uint >Align;
    public DescriptorList <FieldLayoutDescriptor >Fields;
}
/// Singly linked list used by reflection descriptors.
public struct DescriptorList <T >
{
    public bool IsEmpty;
    public T Head;
    public Std.Option <DescriptorList <T >> Tail;
}
public struct ParameterDescriptor
{
    public string Name;
    public TypeHandle ParameterType;
    public ParameterMode Mode;
    public bool HasDefault;
    public Std.Option <string >DefaultValue;
    public DescriptorList <AttributeDescriptor >Attributes;
}
public struct FieldDescriptor
{
    public TypeHandle FieldType;
    public bool IsStatic;
    public bool IsReadonly;
    public Std.Option <ulong >Offset;
}
public struct MethodDescriptor
{
    public TypeHandle ReturnType;
    public DescriptorList <ParameterDescriptor >Parameters;
    public bool IsStatic;
    public bool IsVirtual;
    public bool IsOverride;
    public bool IsAbstract;
    public bool IsAsync;
    public DescriptorList <string >Throws;
    public Std.Option <string >ExternAbi;
}
public struct ConstructorDescriptor
{
    public DescriptorList <ParameterDescriptor >Parameters;
    public bool IsDesignated;
    public bool IsConvenience;
}
public struct MemberDescriptor
{
    public string Name;
    public MemberKind Kind;
    public VisibilityDescriptor Visibility;
    public TypeHandle DeclaringType;
    public DescriptorList <AttributeDescriptor >Attributes;
    public Std.Option <FieldDescriptor >Field;
    public Std.Option <PropertyDescriptor >Property;
    public Std.Option <MethodDescriptor >Method;
    public Std.Option <ConstructorDescriptor >Constructor;
    public DescriptorList <MemberDescriptor >Children;
}
public struct PropertyDescriptor
{
    public TypeHandle PropertyType;
    public bool HasGetter;
    public bool HasSetter;
    public bool HasInit;
    public DescriptorList <ParameterDescriptor >Parameters;
    public Std.Option <MethodDescriptor >Getter;
    public Std.Option <MethodDescriptor >Setter;
    public Std.Option <MethodDescriptor >Init;
}
public struct TypeDescriptor
{
    public Std.Option <string >Namespace;
    public string Name;
    public string FullName;
    public Std.Option <ulong >TypeId;
    public TypeKind Kind;
    public VisibilityDescriptor Visibility;
    public bool IsGeneric;
    public DescriptorList <TypeHandle >GenericArguments;
    public DescriptorList <TypeHandle >Bases;
    public DescriptorList <AttributeDescriptor >Attributes;
    public Std.Option <TypeHandle >UnderlyingType;
    public DescriptorList <MemberDescriptor >Members;
    public Std.Option <TypeLayoutDescriptor >Layout;
    public Std.Option <LayoutDescriptor >LayoutHints;
    public bool Readonly;
}
/// Reflection entry points backed by compile-time metadata.
public static class Reflection
{
    public static extern TypeDescriptor reflect <T >();
}
/// Captured syntax tree nodes produced by `quote(expr)`.
public enum QuoteNodeKind
{
    Literal = 0, Identifier = 1, Unary = 2, Binary = 3, Conditional = 4, Cast = 5, Lambda = 6, Tuple = 7, Assign = 8, Member = 9, Call = 10, Argument = 11, Ref = 12, New = 13, Index = 14, Await = 15, TryPropagate = 16, Throw = 17, SizeOf = 18, AlignOf = 19, NameOf = 20, InterpolatedString = 21, Quote = 22, Pattern = 23, Unknown = 24,
}
public struct QuoteNode
{
    public QuoteNodeKind Kind;
    public Std.Option <string >Value;
    public DescriptorList <QuoteNode >Children;
}
public struct QuoteSpan
{
    public ulong Start;
    public ulong End;
}
public struct QuoteHygiene
{
    public ulong Anchor;
    public ulong Seed;
}
public struct QuoteInterpolation
{
    public string Placeholder;
    public Quote Value;
    public QuoteSpan Span;
}
/// Result of `quote(expr)` including hygiene, span, and interpolation metadata.
public struct Quote
{
    public string Source;
    public string Sanitized;
    public QuoteSpan Span;
    public QuoteHygiene Hygiene;
    public DescriptorList <string >Captures;
    public DescriptorList <QuoteInterpolation >Interpolations;
    public QuoteNode Root;
}
