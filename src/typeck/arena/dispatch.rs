use super::*;
use crate::frontend::ast::PropertyAccessorKind;
use crate::frontend::diagnostics::Span;
use crate::typeck::helpers::{canonical_type_name, type_names_equivalent};
use std::collections::{HashSet, VecDeque};

impl<'a> TypeChecker<'a> {
    pub(crate) fn collect_interface_closure(&self, bases: &[BaseTypeBinding]) -> HashSet<String> {
        let mut closure = HashSet::new();
        let mut queue = VecDeque::new();
        for base in bases {
            if self.is_interface(&base.name) {
                queue.push_back(base.name.clone());
            }
        }
        while let Some(current) = queue.pop_front() {
            if !closure.insert(current.clone()) {
                continue;
            }
            if let Some(info) = self.resolve_type_info(&current) {
                if let TypeKind::Interface { bases, .. } = &info.kind {
                    for base_iface in bases {
                        queue.push_back(base_iface.name.clone());
                    }
                }
            }
        }
        closure
    }

    pub(super) fn check_virtual_dispatch(&mut self) {
        let type_names: Vec<String> = self.types.keys().cloned().collect();
        for name in type_names {
            let Some(infos) = self.types.get(&name).cloned() else {
                continue;
            };
            for info in infos {
                if let TypeKind::Class {
                    methods,
                    bases,
                    properties,
                    ..
                } = &info.kind
                {
                    self.validate_class_virtuals(&name, methods, properties, bases);
                }
            }
        }
    }

    fn validate_class_virtuals(
        &mut self,
        class_name: &str,
        methods: &[SignatureId],
        properties: &[PropertyInfo],
        bases: &[BaseTypeBinding],
    ) {
        let is_abstract_class = self
            .resolve_type_info(class_name)
            .and_then(|info| match &info.kind {
                TypeKind::Class { is_abstract, .. } => Some(*is_abstract),
                _ => None,
            })
            .unwrap_or(false);
        let base_chain = self.class_base_chain(bases);
        let has_base = !base_chain.is_empty();
        let mut inherited = self.collect_inherited_members(&base_chain);

        for method_id in methods {
            self.validate_method_member(class_name, *method_id, &mut inherited);
        }

        for property in properties {
            self.validate_property_member(class_name, property, &mut inherited);
        }

        if !is_abstract_class {
            for method_id in methods {
                if let Some(meta) = self.method_dispatch.get(method_id).cloned()
                    && meta.dispatch.is_abstract
                {
                    let sig = self.signatures.get(*method_id);
                    self.emit_error(
                        codes::ABSTRACT_NOT_IMPLEMENTED,
                        meta.span.or(sig.span),
                        format!(
                            "class `{class_name}` must be declared `abstract` because it declares abstract method `{}`",
                            sig.name
                        ),
                    );
                }
            }
            for property in properties {
                if let Some(accessor) = property
                    .accessor_details
                    .iter()
                    .find(|accessor| accessor.dispatch.is_abstract)
                {
                    let accessor_kind = Self::describe_accessor_kind(accessor.kind);
                    self.emit_error(
                        codes::ABSTRACT_NOT_IMPLEMENTED,
                        accessor.span.or(property.span),
                        format!(
                            "class `{class_name}` must be declared `abstract` because it declares abstract {accessor_kind} for property `{}`",
                            property.name
                        ),
                    );
                }
            }
        }

        if has_base {
            for record in inherited.methods.values() {
                if record.dispatch.is_abstract {
                    let base_sig = self.signatures.get(record.signature_id);
                    self.emit_error(
                        codes::ABSTRACT_NOT_IMPLEMENTED,
                        base_sig.span.or(record.span),
                        format!(
                            "class `{class_name}` must implement abstract method `{}` declared in `{}`",
                            base_sig.name, record.owner
                        ),
                    );
                }
            }
            for (key, record) in inherited.accessors.iter() {
                if record.dispatch.is_abstract {
                    let accessor_kind = Self::describe_accessor_kind(key.kind);
                    self.emit_error(
                        codes::ABSTRACT_NOT_IMPLEMENTED,
                        record.span,
                        format!(
                            "class `{class_name}` must implement abstract {accessor_kind} for property `{}` declared in `{}`",
                            key.property, record.owner
                        ),
                    );
                }
            }
        }
    }

    fn class_base_chain(&self, bases: &[BaseTypeBinding]) -> Vec<String> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut current = self.first_class_base(bases);
        while let Some(base_name) = current {
            if !visited.insert(base_name.clone()) {
                break;
            }
            chain.push(base_name.clone());
            current = self
                .resolve_type_info(&base_name)
                .and_then(|info| match &info.kind {
                    TypeKind::Class { bases, .. } => self.first_class_base(bases),
                    _ => None,
                });
        }
        chain
    }

    fn first_class_base(&self, bases: &[BaseTypeBinding]) -> Option<String> {
        for base in bases {
            if let Some(info) = self.resolve_type_info(&base.name) {
                if matches!(info.kind, TypeKind::Class { .. }) {
                    return Some(base.name.clone());
                }
            }
        }
        None
    }

    fn collect_inherited_members(&self, chain: &[String]) -> InheritedMembers {
        let mut members = InheritedMembers::default();
        for base in chain {
            let Some(info) = self.resolve_type_info(base) else {
                continue;
            };
            if let TypeKind::Class {
                methods,
                properties,
                ..
            } = &info.kind
            {
                self.collect_base_methods(base, methods, &mut members);
                self.collect_base_properties(base, properties, &mut members);
            }
        }
        members
    }

    fn collect_base_methods(
        &self,
        owner: &str,
        method_ids: &[SignatureId],
        members: &mut InheritedMembers,
    ) {
        for id in method_ids {
            let Some(meta) = self.method_dispatch.get(id).cloned() else {
                continue;
            };
            if !Self::is_virtual_candidate(meta.dispatch) {
                continue;
            }
            let signature = self.signatures.get(*id);
            let key = MethodKey::new(signature, self.signature_generic_arity(signature, *id));
            members.methods.entry(key).or_insert(MethodRecord {
                owner: owner.to_string(),
                signature_id: *id,
                dispatch: meta.dispatch,
                visibility: meta.visibility,
                is_static: meta.is_static,
                span: meta.span.or(signature.span),
            });
        }
    }

    fn collect_base_properties(
        &self,
        owner: &str,
        properties: &[PropertyInfo],
        members: &mut InheritedMembers,
    ) {
        for property in properties {
            for accessor in &property.accessor_details {
                if !Self::is_virtual_candidate(accessor.dispatch) {
                    continue;
                }
                let key = PropertyAccessorKey::new(&property.name, accessor.kind);
                members
                    .accessors
                    .entry(key)
                    .or_insert(PropertyAccessorRecord {
                        owner: owner.to_string(),
                        property_type: property.ty.clone(),
                        dispatch: accessor.dispatch,
                        is_static: property.is_static,
                        span: accessor.span.or(property.span),
                        visibility: accessor.visibility,
                    });
            }
        }
    }

    fn validate_method_member(
        &mut self,
        _class_name: &str,
        method_id: SignatureId,
        inherited: &mut InheritedMembers,
    ) {
        let Some(meta) = self.method_dispatch.get(&method_id).cloned() else {
            return;
        };
        let signature = self.signatures.get(method_id).clone();
        let method_name = signature.name.clone();
        let span = meta.span.or(signature.span);
        let key = MethodKey::new(
            &signature,
            self.signature_generic_arity(&signature, method_id),
        );

        if meta.dispatch.is_abstract && meta.has_body {
            self.emit_error(
                codes::ABSTRACT_BODY_FORBIDDEN,
                span,
                format!("abstract method `{method_name}` cannot declare a body"),
            );
        }

        if (meta.dispatch.is_virtual || meta.dispatch.is_override) && !meta.has_body {
            let modifier = if meta.dispatch.is_override {
                "override"
            } else {
                "virtual"
            };
            self.emit_error(
                codes::VIRTUAL_BODY_REQUIRED,
                span,
                format!("{modifier} method `{method_name}` must declare a body"),
            );
        }

        let base_entry = inherited.methods.get(&key).cloned();

        if meta.dispatch.is_override {
            match base_entry {
                Some(record) => {
                    if self.method_override_conflicts(&signature, method_id, span, &meta, &record) {
                        return;
                    }
                    inherited.methods.remove(&key);
                }
                None => {
                    self.emit_error(
                        codes::OVERRIDE_TARGET_NOT_FOUND,
                        span,
                        format!(
                            "method `{method_name}` is marked `override` but no matching virtual member exists in its base types"
                        ),
                    );
                }
            }
        } else if let Some(record) = base_entry {
            if record.dispatch.is_virtual
                || record.dispatch.is_override
                || record.dispatch.is_abstract
            {
                let base_sig = self.signatures.get(record.signature_id);
                self.emit_error(
                    codes::OVERRIDE_MISSING,
                    span,
                    format!(
                        "method `{method_name}` matches virtual member `{}` but is missing the `override` modifier",
                        base_sig.name
                    ),
                );
                if record.dispatch.is_abstract {
                    inherited.methods.remove(&key);
                }
            }
        }
    }

    pub(super) fn validate_property_member(
        &mut self,
        class_name: &str,
        property: &PropertyInfo,
        inherited: &mut InheritedMembers,
    ) {
        for accessor in &property.accessor_details {
            if accessor.dispatch.is_sealed && !accessor.dispatch.is_override {
                self.emit_error(
                    codes::SEALED_REQUIRES_OVERRIDE,
                    accessor.span.or(property.span),
                    format!(
                        "`sealed` {} for property `{}` must also be marked `override`",
                        Self::describe_accessor_kind(accessor.kind),
                        property.name
                    ),
                );
            }
            if accessor.dispatch.is_abstract && accessor.has_body {
                self.emit_error(
                    codes::ABSTRACT_BODY_FORBIDDEN,
                    accessor.span.or(property.span),
                    format!(
                        "abstract {} for property `{}` cannot declare a body",
                        Self::describe_accessor_kind(accessor.kind),
                        property.name
                    ),
                );
            }

            let key = PropertyAccessorKey::new(&property.name, accessor.kind);
            let base_entry = inherited.accessors.get(&key).cloned();

            if accessor.dispatch.is_override {
                match base_entry {
                    Some(record) => {
                        if self.property_override_conflicts(class_name, property, accessor, &record)
                        {
                            continue;
                        }
                        inherited.accessors.remove(&key);
                    }
                    None => {
                        self.emit_error(
                            codes::OVERRIDE_TARGET_NOT_FOUND,
                            accessor.span.or(property.span),
                            format!(
                                "{} for property `{}` is marked `override` but no matching accessor exists in base types",
                                Self::describe_accessor_kind(accessor.kind),
                                property.name
                            ),
                        );
                    }
                }
            } else if let Some(record) = base_entry {
                if record.dispatch.is_virtual
                    || record.dispatch.is_override
                    || record.dispatch.is_abstract
                {
                    self.emit_error(
                        codes::OVERRIDE_MISSING,
                        accessor.span.or(property.span),
                        format!(
                            "{} for property `{}` matches virtual member in `{}` but is missing the `override` modifier",
                            Self::describe_accessor_kind(accessor.kind),
                            property.name,
                            record.owner
                        ),
                    );
                    if record.dispatch.is_abstract {
                        inherited.accessors.remove(&key);
                    }
                }
            }
        }
    }

    pub(super) fn method_override_conflicts(
        &mut self,
        override_sig: &FunctionSignature,
        override_id: SignatureId,
        span: Option<Span>,
        meta: &MethodDispatchInfo,
        base: &MethodRecord,
    ) -> bool {
        let base_sig = self.signatures.get(base.signature_id);
        let method_name = override_sig.name.clone();
        if base.dispatch.is_sealed {
            self.emit_error(
                codes::OVERRIDE_SEALED_MEMBER,
                span,
                format!(
                    "method `{method_name}` cannot override sealed member `{}`",
                    base_sig.name
                ),
            );
            return true;
        }
        if meta.is_static || base.is_static {
            self.emit_error(
                codes::OVERRIDE_STATIC_CONFLICT,
                span,
                format!(
                    "method `{method_name}` cannot override {} member `{}`",
                    if base.is_static { "static" } else { "instance" },
                    base_sig.name
                ),
            );
            return true;
        }
        if Self::visibility_rank(meta.visibility) < Self::visibility_rank(base.visibility) {
            self.emit_error(
                codes::OVERRIDE_VISIBILITY_REDUCTION,
                span,
                format!(
                    "method `{method_name}` cannot reduce visibility when overriding `{}`",
                    base_sig.name
                ),
            );
            return true;
        }
        if !type_names_equivalent(&override_sig.return_type, &base_sig.return_type) {
            self.emit_error(
                codes::OVERRIDE_TYPE_MISMATCH,
                span,
                format!(
                    "method `{}` must return `{}` to match `{}` but returns `{}`",
                    method_name, base_sig.return_type, base_sig.name, override_sig.return_type
                ),
            );
            return true;
        }
        if let Some(message) =
            self.generic_override_mismatch(override_sig, override_id, base_sig, base.signature_id)
        {
            self.emit_error(codes::OVERRIDE_GENERIC_MISMATCH, span, message);
            return true;
        }
        false
    }

    pub(super) fn property_override_conflicts(
        &mut self,
        class_name: &str,
        property: &PropertyInfo,
        accessor: &PropertyAccessorInfo,
        base: &PropertyAccessorRecord,
    ) -> bool {
        let span = accessor.span.or(property.span);
        if base.dispatch.is_sealed {
            self.emit_error(
                codes::OVERRIDE_SEALED_MEMBER,
                span,
                format!(
                    "{} for property `{}` cannot override sealed member declared in `{}`",
                    Self::describe_accessor_kind(accessor.kind),
                    property.name,
                    base.owner
                ),
            );
            return true;
        }
        if property.is_static != base.is_static {
            self.emit_error(
                codes::OVERRIDE_STATIC_CONFLICT,
                span,
                format!(
                    "{} for property `{}` cannot override {} accessor declared in `{}`",
                    Self::describe_accessor_kind(accessor.kind),
                    property.name,
                    if base.is_static { "static" } else { "instance" },
                    base.owner
                ),
            );
            return true;
        }
        if Self::visibility_rank(accessor.visibility) < Self::visibility_rank(base.visibility) {
            self.emit_error(
                codes::OVERRIDE_VISIBILITY_REDUCTION,
                span,
                format!(
                    "{} for property `{}` cannot reduce visibility when overriding member in `{}`",
                    Self::describe_accessor_kind(accessor.kind),
                    property.name,
                    base.owner
                ),
            );
            return true;
        }
        if !type_names_equivalent(&property.ty, &base.property_type) {
            self.emit_error(
                codes::OVERRIDE_TYPE_MISMATCH,
                span,
                format!(
                    "property `{class_name}::{}` must have type `{}` to override `{}` but `{}` was declared",
                    property.name,
                    base.property_type,
                    base.owner,
                    property.ty
                ),
            );
            return true;
        }
        false
    }

    fn generic_override_mismatch(
        &self,
        override_sig: &FunctionSignature,
        override_id: SignatureId,
        base_sig: &FunctionSignature,
        base_id: SignatureId,
    ) -> Option<String> {
        let override_generics = self
            .signature_generics
            .get(&override_id)
            .cloned()
            .or_else(|| self.function_generics.get(&override_sig.name).cloned())
            .unwrap_or_default();
        let base_generics = self
            .signature_generics
            .get(&base_id)
            .cloned()
            .or_else(|| self.function_generics.get(&base_sig.name).cloned())
            .unwrap_or_default();
        if override_generics.len() != base_generics.len() {
            return Some(format!(
                "method `{}` must declare {} generic parameter{} to override `{}` but declares {}",
                override_sig.name,
                base_generics.len(),
                if base_generics.len() == 1 { "" } else { "s" },
                base_sig.name,
                override_generics.len()
            ));
        }
        for (base_param, derived_param) in base_generics.iter().zip(override_generics.iter()) {
            match (&base_param.kind, &derived_param.kind) {
                (GenericParamKind::Type(base_data), GenericParamKind::Type(derived_data)) => {
                    if base_data.variance != derived_data.variance {
                        return Some(format!(
                            "generic parameter `{}` on `{}` must use the same variance as `{}`",
                            derived_param.name, override_sig.name, base_sig.name
                        ));
                    }
                    if self.normalized_constraints(base_data)
                        != self.normalized_constraints(derived_data)
                    {
                        return Some(format!(
                            "generic constraints on `{}` must match the overridden member `{}`",
                            override_sig.name, base_sig.name
                        ));
                    }
                }
                (GenericParamKind::Const(base_data), GenericParamKind::Const(derived_data)) => {
                    if !type_names_equivalent(&base_data.ty.name, &derived_data.ty.name) {
                        return Some(format!(
                            "const generic `{}` on `{}` must have type `{}` to match `{}`",
                            derived_param.name, override_sig.name, base_data.ty.name, base_sig.name
                        ));
                    }
                }
                _ => {
                    return Some(format!(
                        "generic parameters on `{}` must match those on `{}`",
                        override_sig.name, base_sig.name
                    ));
                }
            }
        }
        None
    }

    fn normalized_constraints(&self, data: &TypeParamData) -> Vec<String> {
        let mut normalized = Vec::new();
        for constraint in &data.constraints {
            match &constraint.kind {
                GenericConstraintKind::Struct => normalized.push("struct".to_string()),
                GenericConstraintKind::Class => normalized.push("class".to_string()),
                GenericConstraintKind::NotNull => normalized.push("notnull".to_string()),
                GenericConstraintKind::DefaultConstructor => normalized.push("new()".to_string()),
                GenericConstraintKind::AutoTrait(kind) => {
                    normalized.push(kind.attribute_name().to_string())
                }
                GenericConstraintKind::Type(ty) => {
                    normalized.push(canonical_type_name(ty));
                }
            }
        }
        normalized.sort();
        normalized
    }

    fn is_virtual_candidate(dispatch: MemberDispatch) -> bool {
        dispatch.is_virtual || dispatch.is_override || dispatch.is_abstract || dispatch.is_sealed
    }

    pub(super) fn visibility_rank(vis: Visibility) -> u8 {
        match vis {
            Visibility::Public => 5,
            Visibility::ProtectedInternal => 4,
            Visibility::Protected => 3,
            Visibility::Internal => 2,
            Visibility::PrivateProtected => 1,
            Visibility::Private => 0,
        }
    }

    pub(super) fn describe_accessor_kind(kind: PropertyAccessorKind) -> &'static str {
        match kind {
            PropertyAccessorKind::Get => "getter",
            PropertyAccessorKind::Set => "setter",
            PropertyAccessorKind::Init => "init accessor",
        }
    }
}
