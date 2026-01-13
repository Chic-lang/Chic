//! Frontend metadata utilities including compile-time reflection descriptors.

pub mod reflection;

pub use reflection::{
    AttributeArgument, AttributeDescriptor, ConstructorDescriptor, FieldDescriptor,
    FieldLayoutDescriptor, LayoutDescriptor, MemberDescriptor, MemberKind, MethodDescriptor,
    ParameterDescriptor, ParameterMode, PropertyDescriptor, ReflectionTables, TypeAliasDescriptor,
    TypeDescriptor, TypeHandle, TypeKind, TypeLayoutDescriptor, VisibilityDescriptor,
    collect_and_serialize_reflection, collect_reflection_tables, deserialize_reflection_tables,
    serialize_reflection_tables,
};
