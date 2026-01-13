use super::super::functions::{LoweredMethodMetadata, lower_constructor, lower_function};
use super::super::{
    AstStatement, AstStatementKind, BinOp, BindingModifier, Block, ConstraintKind, Expression,
    FunctionKind, Parameter, PositionalElement, Signature, StructLayout, TypeConstraint,
    TypeLayout, UnOp, Visibility, align_to, pointer_align, pointer_size, qualify,
};
use super::driver::{LoweringDiagnostic, ModuleLowering, dispatch_participates};
use crate::frontend::ast::{
    BinaryOperator, ClassDecl, ClassKind, ClassMember, ConstructorKind, ConversionKind, FieldDecl,
    FunctionDecl, GenericParams, OperatorKind as AstOperatorKind, PropertyAccessor,
    PropertyAccessorBody, PropertyAccessorKind, PropertyDecl, TypeExpr, UnaryOperator,
};
use crate::frontend::attributes::{
    collect_layout_hints, extract_global_allocator, has_fallible_attr,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::mir::layout::{
    AutoTraitOverride, AutoTraitSet, ClassLayoutInfo, ClassLayoutKind, TypeRepr,
};
use crate::mir::operators::{
    ConversionKind as RegistryConversionKind, OperatorKind as RegistryOperatorKind,
    OperatorOverload,
};
use crate::mir::{FieldLayout, PointerTy, Ty};
use crate::type_metadata::TypeFlags;

fn method_is_static(method: &FunctionDecl) -> bool {
    method
        .modifiers
        .iter()
        .any(|modifier| modifier.eq_ignore_ascii_case("static"))
}

impl ModuleLowering {
    pub(super) fn register_class_layout(&mut self, class: &ClassDecl, namespace: Option<&str>) {
        let name = qualify(namespace, &class.name);
        self.register_primitive_attribute(&name, &class.attributes);
        if let Some(existing) = self.type_layouts.types.get(&name) {
            // Allow full Std definitions to replace bootstrap stubs that only define the vtable.
            let is_stub = matches!(
                existing,
                TypeLayout::Class(layout)
                    if layout.fields.iter().all(|field| field.name == "$vtable")
            );
            if !is_stub {
                return;
            }
        }

        for member in &class.members {
            match member {
                ClassMember::Field(field) => {
                    self.register_static_field_decl(&name, namespace, class.visibility, field);
                }
                ClassMember::Property(property) => {
                    self.register_static_property_backing(&name, namespace, property);
                }
                _ => {}
            }
        }

        self.record_type_visibility(&name, class.visibility, namespace, None);

        let (allocator_attr, errors) = extract_global_allocator(&class.attributes);
        self.push_attribute_errors(errors);
        if let Some(attr) = allocator_attr {
            self.record_global_allocator(name.clone(), attr);
        }

        let mut resolved_bases: Vec<String> = class
            .bases
            .iter()
            .filter_map(|base| {
                match self.resolve_type_for_expr(base, namespace, Some(name.as_str())) {
                    ImportResolution::Found(resolved) => Some(resolved),
                    ImportResolution::Ambiguous(candidates) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "base type of `{name}` resolves to multiple types: {}",
                                candidates.join(", ")
                            ),
                            span: base.span,
                        });
                        None
                    }
                    ImportResolution::NotFound => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "base type `{}` for `{name}` could not be resolved",
                                base.name
                            ),
                            span: base.span,
                        });
                        None
                    }
                }
            })
            .collect();
        resolved_bases.retain(|base| base != &name);
        if !resolved_bases.is_empty() {
            self.class_bases
                .insert(name.clone(), resolved_bases.clone());
        }

        let field_members = class
            .members
            .iter()
            .filter_map(|member| match member {
                ClassMember::Field(field) if !field.is_static => Some(field.clone()),
                ClassMember::Property(property) if property.is_auto() => Some(FieldDecl {
                    visibility: Visibility::Private,
                    name: property.backing_field_name(),
                    ty: property.ty.clone(),
                    initializer: property.initializer.clone(),
                    mmio: None,
                    doc: None,
                    is_required: property.is_required,
                    display_name: Some(property.name.clone()),
                    attributes: Vec::new(),
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                }),
                ClassMember::Const(_) => None,
                _ => None,
            })
            .collect::<Vec<_>>();
        let (layout_hints, errors) = collect_layout_hints(&class.attributes);
        for error in errors {
            self.diagnostics.push(LoweringDiagnostic {
                message: error.message,
                span: error.span,
            });
        }

        let packing_limit = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1) as usize);
        let layout_packing = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1));

        let mut base_offset = pointer_size();
        let mut base_align_requirement = pointer_align();
        if let Some(base) = resolved_bases.first() {
            if let Some(base_layout) = self.type_layouts.layout_for_name(base) {
                if let TypeLayout::Class(base_layout) = base_layout {
                    if let Some(size) = base_layout.size {
                        base_offset = base_offset.max(size);
                    }
                    if let Some(align) = base_layout.align {
                        base_align_requirement = base_align_requirement.max(align);
                    }
                }
            }
        }

        let (mut fields, mut size, mut align) = self.compute_field_layouts(
            &field_members,
            namespace,
            Some(name.as_str()),
            packing_limit,
            base_offset,
        );
        let positional = fields
            .iter()
            .map(|field| PositionalElement {
                field_index: field.index,
                name: Some(field.name.clone()),
                span: field.span,
            })
            .collect();
        let list = Self::infer_list_layout(&fields);
        fields.push(FieldLayout {
            name: "$vtable".into(),
            ty: Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true))),
            index: u32::MAX,
            offset: Some(0),
            span: None,
            mmio: None,
            display_name: None,
            is_required: false,
            is_nullable: false,
            is_readonly: true,
            view_of: None,
        });
        let overrides = AutoTraitOverride {
            thread_safe: class.thread_safe_override,
            shareable: class.shareable_override,
            copy: class.copy_override,
        };
        if let Some(align_hint) = layout_hints.align {
            let mut requested = align_hint.value as usize;
            if let Some(pack_limit) = packing_limit {
                if requested > pack_limit {
                    let pack_display = layout_packing.unwrap_or(1);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("`@align({requested})` exceeds the `@repr(packed({pack_display}))` limit"),
                        span: align_hint.span.or(layout_hints.packing.and_then(|hint| hint.span)),
                                            });
                    requested = pack_limit;
                }
            }
            align = Some(align.map_or(requested, |current| current.max(requested)));
        }

        if let Some(alignment) = align {
            if let Some(current_size) = size {
                size = Some(align_to(current_size, alignment));
            }
        }
        let header_align = pointer_align();
        align = Some(
            align
                .unwrap_or(header_align)
                .max(header_align)
                .max(base_align_requirement),
        );
        if let Some(current_size) = size {
            size = Some(align_to(current_size, align.unwrap()));
        }

        let mut layout = StructLayout {
            name: name.clone(),
            repr: if layout_hints.repr_c {
                TypeRepr::C
            } else {
                TypeRepr::Default
            },
            packing: layout_packing,
            fields,
            positional,
            list,
            size,
            align,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides,
            mmio: None,
            dispose: None,
            class: Some(ClassLayoutInfo {
                kind: match class.kind {
                    ClassKind::Class => ClassLayoutKind::Class,
                    ClassKind::Error => ClassLayoutKind::Error,
                },
                bases: resolved_bases.clone(),
                vtable_offset: Some(0),
            }),
        };
        layout.dispose = self.dispose_symbol(&name);
        self.type_layouts
            .types
            .insert(name.clone(), TypeLayout::Class(layout));
        self.class_decls.insert(name.clone(), class.clone());
        self.mark_fallible_class(&name, &class.attributes, &resolved_bases);
    }

    fn map_unary_operator(op: UnaryOperator) -> Option<UnOp> {
        match op {
            UnaryOperator::Negate => Some(UnOp::Neg),
            UnaryOperator::UnaryPlus => Some(UnOp::UnaryPlus),
            UnaryOperator::LogicalNot => Some(UnOp::Not),
            UnaryOperator::OnesComplement => Some(UnOp::BitNot),
            UnaryOperator::Increment => Some(UnOp::Increment),
            UnaryOperator::Decrement => Some(UnOp::Decrement),
        }
    }

    fn map_binary_operator(op: BinaryOperator) -> Option<BinOp> {
        match op {
            BinaryOperator::Add => Some(BinOp::Add),
            BinaryOperator::Subtract => Some(BinOp::Sub),
            BinaryOperator::Multiply => Some(BinOp::Mul),
            BinaryOperator::Divide => Some(BinOp::Div),
            BinaryOperator::Remainder => Some(BinOp::Rem),
            BinaryOperator::BitAnd => Some(BinOp::BitAnd),
            BinaryOperator::BitOr => Some(BinOp::BitOr),
            BinaryOperator::BitXor => Some(BinOp::BitXor),
            BinaryOperator::ShiftLeft => Some(BinOp::Shl),
            BinaryOperator::ShiftRight => Some(BinOp::Shr),
            BinaryOperator::Equal => Some(BinOp::Eq),
            BinaryOperator::NotEqual => Some(BinOp::Ne),
            BinaryOperator::LessThan => Some(BinOp::Lt),
            BinaryOperator::LessThanOrEqual => Some(BinOp::Le),
            BinaryOperator::GreaterThan => Some(BinOp::Gt),
            BinaryOperator::GreaterThanOrEqual => Some(BinOp::Ge),
        }
    }

    fn qualified_type_name(
        &mut self,
        ty: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> String {
        match self.resolve_type_for_expr(ty, namespace, context_type) {
            ImportResolution::Found(resolved) => resolved,
            ImportResolution::Ambiguous(_) | ImportResolution::NotFound => ty.name.clone(),
        }
    }

    pub(super) fn register_operator_overload(
        &mut self,
        owner: &str,
        namespace: Option<&str>,
        context_type: Option<&str>,
        method: &FunctionDecl,
        function_name: &str,
    ) {
        let Some(operator) = method.operator.as_ref() else {
            return;
        };

        let mut params = Vec::new();
        for param in &method.signature.parameters {
            params.push(self.qualified_type_name(&param.ty, namespace, context_type));
        }
        let result =
            self.qualified_type_name(&method.signature.return_type, namespace, context_type);

        let kind = match operator.kind {
            AstOperatorKind::Unary(op) => {
                let Some(mapped) = Self::map_unary_operator(op) else {
                    return;
                };
                RegistryOperatorKind::Unary(mapped)
            }
            AstOperatorKind::Binary(op) => {
                let Some(mapped) = Self::map_binary_operator(op) else {
                    return;
                };
                RegistryOperatorKind::Binary(mapped)
            }
            AstOperatorKind::Conversion(conv) => {
                let mapped = match conv {
                    ConversionKind::Implicit => RegistryConversionKind::Implicit,
                    ConversionKind::Explicit => RegistryConversionKind::Explicit,
                };
                RegistryOperatorKind::Conversion(mapped)
            }
        };

        let overload = OperatorOverload {
            kind,
            params,
            result,
            function: function_name.to_string(),
        };
        self.operator_registry.register(owner, overload);
    }

    pub(super) fn lower_class(&mut self, class: &ClassDecl, namespace: Option<&str>) {
        let class_ns = qualify(namespace, &class.name);
        let class_layout = self
            .type_layouts
            .types
            .get(&class_ns)
            .and_then(|layout| match layout {
                TypeLayout::Class(layout) => Some(layout.clone()),
                _ => None,
            });
        let mut has_designated_ctor = false;
        let mut first_convenience_span: Option<Span> = None;
        for member in &class.members {
            if let ClassMember::Constructor(ctor) = member {
                match ctor.kind {
                    ConstructorKind::Designated => has_designated_ctor = true,
                    ConstructorKind::Convenience => {
                        if first_convenience_span.is_none() {
                            first_convenience_span = ctor.span;
                        }
                    }
                }
            }
        }
        if !has_designated_ctor && first_convenience_span.is_some() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "class `{}` requires a designated init for convenience constructors to delegate",
                    class_ns
                ),
                span: first_convenience_span,
                            });
        }
        for base in &class.bases {
            self.ensure_type_expr_accessible(
                base,
                namespace,
                Some(class_ns.as_str()),
                &format!("base type of `{class_ns}`"),
                None,
            );
            let interface = base.name.replace('.', "::");
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::ImplementsInterface {
                    type_name: class_ns.clone(),
                    interface,
                },
                None,
            ));
        }
        for member in &class.members {
            if let ClassMember::Method(method) = member {
                let method_name = format!("{class_ns}::{}", method.name);
                self.collect_exports_for(&method_name, &method.attributes);
                self.collect_link_library(method.link_library.as_deref());
                self.check_signature(
                    &method.signature,
                    Some(class_ns.as_str()),
                    Some(class_ns.as_str()),
                    &method_name,
                );
                let perf_diags =
                    self.record_perf_attributes(&method_name, &method.attributes, None);
                self.diagnostics.extend(perf_diags);
                let mut lowered = lower_function(
                    method,
                    &method_name,
                    FunctionKind::Method,
                    Some(class_ns.as_str()),
                    self.current_package.as_deref(),
                    class.generics.as_ref(),
                    &mut self.type_layouts,
                    &self.type_visibilities,
                    &self.primitive_registry,
                    self.default_arguments.clone(),
                    &self.function_packages,
                    &self.operator_registry,
                    &mut self.string_interner,
                    &self.symbol_index,
                    &self.import_resolver,
                    &self.static_registry,
                    &self.class_bases,
                    &self.class_virtual_slots,
                    &self.trait_decls,
                    self.generic_specializations.clone(),
                );
                if !method_is_static(method) && dispatch_participates(method.dispatch) {
                    lowered.method_metadata = Some(LoweredMethodMetadata {
                        owner: class_ns.clone(),
                        member: method.name.clone(),
                        dispatch: method.dispatch,
                        accessor: None,
                    });
                }
                let _ = self.record_lowered_function(lowered);
            }
        }
        for member in &class.members {
            if let ClassMember::Property(property) = member {
                self.lower_property(class_ns.as_str(), property, class.generics.as_ref());
            }
        }
        let mut ctor_index = 0usize;
        for member in &class.members {
            if let ClassMember::Constructor(ctor) = member {
                if let Some(layout) = class_layout.as_ref() {
                    let ctor_name = format!("{class_ns}::init#{ctor_index}");
                    let perf_diags =
                        self.record_perf_attributes(&ctor_name, &ctor.attributes, None);
                    self.diagnostics.extend(perf_diags);
                    let lowered = lower_constructor(
                        class_ns.as_str(),
                        ctor,
                        &ctor_name,
                        Some(class_ns.as_str()),
                        self.current_package.as_deref(),
                        class.generics.as_ref(),
                        &mut self.type_layouts,
                        &self.type_visibilities,
                        &self.primitive_registry,
                        self.default_arguments.clone(),
                        &self.function_packages,
                        &self.operator_registry,
                        &mut self.string_interner,
                        &self.symbol_index,
                        &self.import_resolver,
                        &self.static_registry,
                        &self.class_bases,
                        &self.class_virtual_slots,
                        &self.trait_decls,
                        self.generic_specializations.clone(),
                        layout,
                    );
                    let _ = self.record_lowered_function(lowered);
                }
                ctor_index += 1;
            }
        }
    }

    pub(super) fn lower_property(
        &mut self,
        type_ns: &str,
        property: &PropertyDecl,
        type_generics: Option<&GenericParams>,
    ) {
        let backing_field = property.is_auto().then(|| property.backing_field_name());

        for kind in [
            PropertyAccessorKind::Get,
            PropertyAccessorKind::Set,
            PropertyAccessorKind::Init,
        ] {
            let Some(accessor) = property.accessor(kind) else {
                continue;
            };

            match self.build_property_accessor_decl(
                type_ns,
                property,
                accessor,
                kind,
                backing_field.as_deref(),
            ) {
                Ok(accessor_decl) => {
                    let qualified_name = format!("{type_ns}::{}", accessor_decl.name);
                    let perf_diags = self.record_perf_attributes(
                        &qualified_name,
                        &accessor_decl.attributes,
                        None,
                    );
                    self.diagnostics.extend(perf_diags);
                    let mut lowered = lower_function(
                        &accessor_decl,
                        &qualified_name,
                        FunctionKind::Method,
                        Some(type_ns),
                        self.current_package.as_deref(),
                        type_generics,
                        &mut self.type_layouts,
                        &self.type_visibilities,
                        &self.primitive_registry,
                        self.default_arguments.clone(),
                        &self.function_packages,
                        &self.operator_registry,
                        &mut self.string_interner,
                        &self.symbol_index,
                        &self.import_resolver,
                        &self.static_registry,
                        &self.class_bases,
                        &self.class_virtual_slots,
                        &self.trait_decls,
                        self.generic_specializations.clone(),
                    );
                    if !property.is_static && dispatch_participates(accessor.dispatch) {
                        lowered.method_metadata = Some(LoweredMethodMetadata {
                            owner: type_ns.to_string(),
                            member: accessor_decl.name.clone(),
                            dispatch: accessor.dispatch,
                            accessor: Some(kind),
                        });
                    }
                    let _ = self.record_lowered_function(lowered);
                }
                Err(diag) => self.diagnostics.push(diag),
            }
        }
    }

    fn build_property_accessor_decl(
        &mut self,
        type_ns: &str,
        property: &PropertyDecl,
        accessor: &PropertyAccessor,
        kind: PropertyAccessorKind,
        backing_field: Option<&str>,
    ) -> Result<FunctionDecl, LoweringDiagnostic> {
        let visibility = accessor.visibility.unwrap_or(property.visibility.clone());
        let signature = self.property_accessor_signature(type_ns, property, kind);
        let body = self.property_accessor_body(type_ns, property, accessor, kind, backing_field)?;

        Ok(FunctionDecl {
            visibility,
            name: property.accessor_method_name(kind),
            name_span: None,
            signature,
            body: Some(body),
            is_async: false,
            is_constexpr: false,
            doc: accessor.doc.clone().or_else(|| property.doc.clone()),
            modifiers: if property.is_static {
                vec!["static".to_string()]
            } else {
                Vec::new()
            },
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: None,
            vectorize_hint: None,
            dispatch: accessor.dispatch,
        })
    }

    fn property_accessor_signature(
        &self,
        _type_ns: &str,
        property: &PropertyDecl,
        kind: PropertyAccessorKind,
    ) -> Signature {
        let mut parameters = Vec::new();
        let mut return_type = TypeExpr::simple("void");

        if !property.is_static {
            let binding = match kind {
                PropertyAccessorKind::Get => BindingModifier::Value,
                PropertyAccessorKind::Set | PropertyAccessorKind::Init => BindingModifier::Ref,
            };
            parameters.push(Parameter {
                binding,
                binding_nullable: false,
                name: "this".to_string(),
                name_span: None,
                ty: TypeExpr::self_type(),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: true,
            });
        }

        parameters.extend(property.parameters.iter().cloned());

        match kind {
            PropertyAccessorKind::Get => {
                return_type = property.ty.clone();
            }
            PropertyAccessorKind::Set | PropertyAccessorKind::Init => {
                parameters.push(Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "value".to_string(),
                    name_span: None,
                    ty: property.ty.clone(),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                });
            }
        }

        Signature {
            parameters,
            return_type,
            lends_to_return: None,
            variadic: false,
            throws: None,
        }
    }

    fn property_accessor_body(
        &self,
        type_ns: &str,
        property: &PropertyDecl,
        accessor: &PropertyAccessor,
        kind: PropertyAccessorKind,
        backing_field: Option<&str>,
    ) -> Result<Block, LoweringDiagnostic> {
        let span = accessor.span.or(property.span);
        match &accessor.body {
            PropertyAccessorBody::Auto => {
                let field_name = backing_field.ok_or_else(|| LoweringDiagnostic {
                    message: format!(
                        "auto property `{}` requires generated backing storage but accessor `{kind:?}` was not recognised as auto",
                        property.name
                    ),
                    span,
                                    })?;

                let target_expr = if property.is_static {
                    format!("{}.{}", type_ns.replace("::", "."), field_name)
                } else {
                    format!("this.{field_name}")
                };
                match kind {
                    PropertyAccessorKind::Get => Ok(Block {
                        statements: vec![AstStatement::new(
                            span,
                            AstStatementKind::Return {
                                expression: Some(Expression::new(target_expr, span)),
                            },
                        )],
                        span,
                    }),
                    PropertyAccessorKind::Set | PropertyAccessorKind::Init => Ok(Block {
                        statements: vec![
                            AstStatement::new(
                                span,
                                AstStatementKind::Expression(Expression::new(
                                    format!("{target_expr} = value"),
                                    span,
                                )),
                            ),
                            AstStatement::new(span, AstStatementKind::Return { expression: None }),
                        ],
                        span,
                    }),
                }
            }
            PropertyAccessorBody::Block(block) => Ok(block.clone()),
            PropertyAccessorBody::Expression(expr) => match kind {
                PropertyAccessorKind::Get => Ok(Block {
                    statements: vec![AstStatement::new(
                        span,
                        AstStatementKind::Return {
                            expression: Some(expr.clone()),
                        },
                    )],
                    span,
                }),
                PropertyAccessorKind::Set | PropertyAccessorKind::Init => Ok(Block {
                    statements: vec![
                        AstStatement::new(span, AstStatementKind::Expression(expr.clone())),
                        AstStatement::new(span, AstStatementKind::Return { expression: None }),
                    ],
                    span,
                }),
            },
        }
    }

    fn mark_fallible_class(
        &mut self,
        name: &str,
        attrs: &[crate::frontend::ast::Attribute],
        bases: &[String],
    ) {
        let mut should_flag = false;
        if has_fallible_attr(attrs) {
            should_flag = true;
        } else {
            let short = name.rsplit("::").next().unwrap_or(name);
            if short.ends_with("Exception") {
                should_flag = true;
            }
        }
        if !should_flag {
            should_flag = bases.iter().any(|base| {
                let canonical = base.replace('.', "::");
                self.type_layouts
                    .type_flags_for_name(canonical)
                    .contains(TypeFlags::FALLIBLE)
            });
        }
        if should_flag {
            self.type_layouts
                .add_type_flags(name.to_string(), TypeFlags::FALLIBLE);
        }
    }
}
