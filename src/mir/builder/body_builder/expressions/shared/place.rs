use super::*;
use crate::mir::FloatWidth;
use crate::mir::layout::TypeLayout;
use crate::syntax::numeric::{IntegerWidth, NumericLiteralType};

body_builder_impl! {
    pub(crate) fn target_looks_like_static_member(&self, expr: &ExprNode) -> Option<bool> {
        match expr {
            ExprNode::Member { base, .. } => Some(self.member_chain_unresolved(base.as_ref())),
            ExprNode::Parenthesized(inner) => self.target_looks_like_static_member(inner.as_ref()),
            _ => None,
        }
    }
    pub(crate) fn normalise_place(&self, place: &mut Place) {
        let mut current_type = self
            .locals
            .get(place.local.0)
            .and_then(|decl| self.resolve_ty_name(&decl.ty));

        for elem in &mut place.projection {
            current_type = self.update_projection_element(elem, current_type.as_deref());
        }
    }
    pub(crate) fn resolve_self_field_place(&mut self, name: &str) -> Option<Place> {
        let Some(self_local) = self.lookup_name("self") else {
            return None;
        };
        let Some(self_type) = self.current_self_type_name() else {
            return None;
        };
        let layout = self
            .lookup_struct_layout_by_name(&self_type)
            .or_else(|| {
                self.type_layouts
                    .types
                    .get(&self_type)
                    .and_then(|layout| match layout {
                        TypeLayout::Class(layout) => Some(layout),
                        _ => None,
                    })
            });
        if let Some(layout) = layout.as_ref() {
            if !layout.fields.iter().any(|field| field.matches_name(name)) {
                return None;
            }
        }

        let mut place = Place::new(self_local);
        place
            .projection
            .push(ProjectionElem::FieldNamed(name.to_string()));
        self.normalise_place(&mut place);
        Some(place)
    }
    pub(crate) fn place_ty(&self, place: &Place) -> Option<Ty> {
        let mut current_ty = self.locals.get(place.local.0)?.ty.clone();
        for elem in &place.projection {
            if let Some(projected) = self.project_ty(&current_ty, elem) {
                current_ty = projected;
            } else if let Some(fallback) = self.fallback_project_ty(&current_ty, elem) {
                current_ty = fallback;
            } else {
                return None;
            }
        }
        Some(current_ty)
    }
    pub(crate) fn place_type_name(&self, place: &Place) -> Option<String> {
        let ty = self.place_ty(place)?;
        if matches!(Self::strip_nullable(&ty), Ty::Unknown) {
            return None;
        }
        if let Some(name) = self.resolve_ty_name(&ty) {
            return Some(name);
        }
        Some(ty.canonical_name())
    }

    pub(crate) fn place_owner_type_name(&self, place: &Place) -> Option<String> {
        if place.projection.is_empty() {
            return None;
        }
        let mut parent = place.clone();
        parent.projection.pop();
        self.place_type_name(&parent)
    }
    pub(crate) fn operand_is_nullable(&self, operand: &Operand) -> Option<bool> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.place_is_nullable(place),
            Operand::Borrow(borrow) => self.place_is_nullable(&borrow.place),
            Operand::Mmio(_) => Some(false),
            Operand::Const(_) => Some(false),
            Operand::Pending(_) => None,
        }
    }
    pub(crate) fn place_is_nullable(&self, place: &Place) -> Option<bool> {
        let current_decl = self.locals.get(place.local.0)?;
        let mut nullable = current_decl.is_nullable;
        let mut current_type = self.resolve_ty_name(&current_decl.ty);

        for elem in &place.projection {
            match elem {
                ProjectionElem::Field(index) => {
                    let type_name = current_type.as_ref()?;
                    let layout = self.lookup_struct_layout_by_name(type_name)?;
                    let field = layout.fields.iter().find(|f| f.index == *index)?;
                    nullable = field.is_nullable;
                    current_type = self.resolve_ty_name(&field.ty);
                }
                ProjectionElem::FieldNamed(name) => {
                    let type_name = current_type.as_ref()?;
                    if let Some(struct_layout) = self.lookup_struct_layout_by_name(type_name) {
                        if let Some(field) =
                            struct_layout.fields.iter().find(|f| f.matches_name(name))
                        {
                            nullable = field.is_nullable;
                            current_type = self.resolve_ty_name(&field.ty);
                            continue;
                        }
                    }
                    if let Some(union_layout) = self.lookup_union_layout(type_name) {
                        if let Some(view) = union_layout.views.iter().find(|view| view.name == *name)
                        {
                            nullable = view.is_nullable;
                            current_type = self.resolve_ty_name(&view.ty);
                            continue;
                        }
                    }
                    return None;
                }
                ProjectionElem::UnionField { index, .. } => {
                    let type_name = current_type.as_ref()?;
                    let layout = self.lookup_union_layout(type_name)?;
                    let view = layout.views.iter().find(|v| v.index == *index)?;
                    nullable = view.is_nullable;
                    current_type = self.resolve_ty_name(&view.ty);
                }
                ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Deref
                | ProjectionElem::Downcast { .. }
                | ProjectionElem::Subslice { .. } => return None,
            };
        }

        Some(nullable)
    }
    pub(crate) fn operand_type_name(&self, operand: &Operand) -> Option<String> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.place_type_name(place),
            Operand::Borrow(borrow) => {
                let ty = self.place_ty(&borrow.place)?;
                let mut referent = ty.canonical_name();
                if referent == "string" {
                    referent = "str".to_string();
                }
                let prefix = if matches!(borrow.kind, BorrowKind::Shared) {
                    "ref readonly "
                } else {
                    "ref "
                };
                Some(format!("{prefix}{referent}"))
            }
            Operand::Mmio(spec) => self.resolve_ty_name(&spec.ty),
            Operand::Const(constant) => self.const_operand_type_from_const_operand(constant),
            Operand::Pending(_) => None,
        }
    }
    pub(crate) fn property_symbol_from_operand(
        &self,
        operand: &Operand,
        member: &str,
    ) -> Option<(String, &PropertySymbol)> {
        let raw = self.operand_type_name(operand)?;
        let debug_property = std::env::var_os("CHIC_DEBUG_PROPERTY_LOOKUP").is_some();
        if debug_property && raw.contains("MutexGuard") && member == "Value" {
            eprintln!("[chic-debug] property lookup (operand) type={raw}");
            let known: Vec<_> = self
                .symbol_index
                .types()
                .filter(|t| t.contains("MutexGuard"))
                .cloned()
                .collect();
            eprintln!("[chic-debug] property lookup known types with MutexGuard: {known:?}");
        }
        let base = raw
            .strip_prefix("ref readonly ")
            .or_else(|| raw.strip_prefix("ref "))
            .unwrap_or(raw.as_str())
            .trim_end_matches('?');
        let base = base.replace('.', "::");
        let mut candidates = Vec::new();
        candidates.push(base.clone());
        candidates.push(
            base.split('<')
                .next()
                .unwrap_or(base.as_str())
                .to_string(),
        );
        let short = crate::mir::casts::short_type_name(&base);
        candidates.push(short.split('<').next().unwrap_or(short).to_string());

        let mut visited = std::collections::HashSet::new();
        let mut stack = candidates;
        while let Some(candidate) = stack.pop() {
            if !visited.insert(candidate.clone()) {
                continue;
            }
            if let Some((owner, symbol)) = self.property_symbol_from_candidate(&candidate, member) {
                if debug_property && base.contains("MutexGuard") && member == "Value" {
                    eprintln!(
                        "[chic-debug] property lookup (operand) matched candidate={candidate}"
                    );
                }
                return Some((owner, symbol));
            }
            let base_key = candidate
                .split('<')
                .next()
                .unwrap_or(candidate.as_str())
                .to_string();
            if let Some(bases) = self.class_bases.get(&base_key) {
                for base in bases {
                    stack.push(base.clone());
                }
                continue;
            }
            for (owner, bases) in self.class_bases {
                if owner.ends_with(&format!("::{base_key}")) {
                    for base in bases {
                        stack.push(base.clone());
                    }
                }
            }
        }
        None
    }
    fn property_symbol_from_candidate(
        &self,
        candidate: &str,
        member: &str,
    ) -> Option<(String, &PropertySymbol)> {
        if let Some(props) = self.symbol_index.type_properties.get(candidate) {
            if let Some(symbol) = props.get(member) {
                return Some((candidate.to_string(), symbol));
            }
        }
        let suffix = format!("::{candidate}");
        for (owner, props) in &self.symbol_index.type_properties {
            if owner.ends_with(&suffix) {
                if let Some(symbol) = props.get(member) {
                    return Some((owner.clone(), symbol));
                }
            }
        }
        if let Some(desc) = self.primitive_registry.descriptor_for_name(candidate) {
            if let Some(wrapper) = desc.std_wrapper_type.as_deref() {
                let wrapper_key = wrapper.replace('.', "::");
                if wrapper_key != candidate {
                    if let Some(props) = self.symbol_index.type_properties.get(&wrapper_key) {
                        if let Some(symbol) = props.get(member) {
                            return Some((wrapper_key, symbol));
                        }
                    }
                    let wrapper_suffix = format!("::{wrapper_key}");
                    for (owner, props) in &self.symbol_index.type_properties {
                        if owner.ends_with(&wrapper_suffix) {
                            if let Some(symbol) = props.get(member) {
                                return Some((owner.clone(), symbol));
                            }
                        }
                    }
                }
            }
        }
        None
    }
    pub(crate) fn property_symbol_from_place(
        &self,
        place: &Place,
        member: &str,
    ) -> Option<(String, &PropertySymbol)> {
        let raw = self.place_type_name(place)?;
        let debug_property = std::env::var_os("CHIC_DEBUG_PROPERTY_LOOKUP").is_some();
        if debug_property && raw.contains("MutexGuard") && member == "Value" {
            eprintln!("[chic-debug] property lookup (place) type={raw:?}");
            let known: Vec<_> = self
                .symbol_index
                .types()
                .filter(|t| t.contains("MutexGuard"))
                .cloned()
                .collect();
            eprintln!("[chic-debug] property lookup known types with MutexGuard: {known:?}");
        }
        let base = raw
            .strip_prefix("ref readonly ")
            .or_else(|| raw.strip_prefix("ref "))
            .unwrap_or(raw.as_str())
            .trim_end_matches('?');
        let base = base.replace('.', "::");
        let mut candidates = Vec::new();
        candidates.push(base.clone());
        candidates.push(
            base.split('<')
                .next()
                .unwrap_or(base.as_str())
                .to_string(),
        );
        let short = crate::mir::casts::short_type_name(&base);
        candidates.push(short.split('<').next().unwrap_or(short).to_string());

        let mut visited = std::collections::HashSet::new();
        let mut stack = candidates;
        while let Some(candidate) = stack.pop() {
            if !visited.insert(candidate.clone()) {
                continue;
            }
            if let Some((owner, symbol)) = self.property_symbol_from_candidate(&candidate, member) {
                if debug_property && base.contains("MutexGuard") && member == "Value" {
                    eprintln!(
                        "[chic-debug] property lookup (place) matched candidate={candidate}"
                    );
                }
                return Some((owner, symbol));
            }
            let base_key = candidate
                .split('<')
                .next()
                .unwrap_or(candidate.as_str())
                .to_string();
            if let Some(bases) = self.class_bases.get(&base_key) {
                for base in bases {
                    stack.push(base.clone());
                }
                continue;
            }
            for (owner, bases) in self.class_bases {
                if owner.ends_with(&format!("::{base_key}")) {
                    for base in bases {
                        stack.push(base.clone());
                    }
                }
            }
        }
        None
    }
    pub(crate) fn const_operand_type_from_const_operand(
        &self,
        constant: &ConstOperand,
    ) -> Option<String> {
        if let Some(literal) = constant.literal() {
            if literal.explicit_suffix {
                if let Some(name) = match literal.literal_type {
                    NumericLiteralType::Signed(width) => Some(match width {
                        IntegerWidth::W8 => "i8",
                        IntegerWidth::W16 => "i16",
                        IntegerWidth::W32 => "int",
                        IntegerWidth::W64 => "long",
                        IntegerWidth::W128 => "i128",
                        IntegerWidth::Size => "isize",
                    }),
                    NumericLiteralType::Unsigned(width) => Some(match width {
                        IntegerWidth::W8 => "u8",
                        IntegerWidth::W16 => "u16",
                        IntegerWidth::W32 => "uint",
                        IntegerWidth::W64 => "ulong",
                        IntegerWidth::W128 => "u128",
                        IntegerWidth::Size => "usize",
                    }),
                    NumericLiteralType::Float16 => Some("float16"),
                    NumericLiteralType::Float32 => Some("float"),
                    NumericLiteralType::Float64 => Some("double"),
                    NumericLiteralType::Float128 => Some("float128"),
                    NumericLiteralType::Decimal => Some("decimal"),
                } {
                    return Some(name.to_string());
                }
            }
        }
        self.const_operand_type(&constant.value)
    }

    pub(crate) fn const_operand_type(&self, value: &ConstValue) -> Option<String> {
        match value {
            ConstValue::Int(value) | ConstValue::Int32(value) => {
                if *value >= i32::MIN as i128 && *value <= i32::MAX as i128 {
                    Some("int".to_string())
                } else if *value >= i64::MIN as i128 && *value <= i64::MAX as i128 {
                    Some("long".to_string())
                } else {
                    Some("i128".to_string())
                }
            }
            ConstValue::UInt(value) => {
                if *value <= u32::MAX as u128 {
                    Some("uint".to_string())
                } else if *value <= u64::MAX as u128 {
                    Some("ulong".to_string())
                } else {
                    Some("u128".to_string())
                }
            }
            ConstValue::Float(v) => Some(match v.width {
                FloatWidth::F16 => "float16".to_string(),
                FloatWidth::F32 => "float".to_string(),
                FloatWidth::F64 => "double".to_string(),
                FloatWidth::F128 => "float128".to_string(),
            }),
            ConstValue::Decimal(_) => Some("decimal".to_string()),
            ConstValue::Bool(_) => Some("bool".to_string()),
            ConstValue::Char(_) => Some("char".to_string()),
            ConstValue::Str { .. } | ConstValue::RawStr(_) => Some("str".to_string()),
            ConstValue::Symbol(name) => self
                .symbol_index
                .function_signature(name)
                .map(|sig| sig.canonical_name())
                .or_else(|| {
                    self.closure_fn_signatures
                        .get(name)
                        .map(|sig| sig.canonical_name())
                }),
            ConstValue::Enum { type_name, .. } => Some(type_name.clone()),
            ConstValue::Struct { type_name, .. } => Some(type_name.clone()),
            ConstValue::Null | ConstValue::Unit | ConstValue::Unknown => None,
        }
    }
    pub(crate) fn operand_fn_ty(&self, operand: &Operand) -> Option<FnTy> {
        match operand {
            Operand::Const(constant) => constant
                .symbol_name()
                .and_then(|name| self.symbol_index.function_signature(name).cloned()),
            Operand::Copy(place) | Operand::Move(place) => {
                let decl = self.locals.get(place.local.0)?;
                match &decl.ty {
                    Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                    Ty::Nullable(inner) => {
                        if let Ty::Fn(fn_ty) = inner.as_ref() {
                            Some(fn_ty.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
    pub(crate) fn update_projection_element(
        &self,
        elem: &mut ProjectionElem,
        current_type: Option<&str>,
    ) -> Option<String> {
        let struct_layout = current_type.and_then(|name| self.lookup_struct_layout_by_name(name));
        let union_layout = current_type.and_then(|name| self.lookup_union_layout(name));

        match elem {
            ProjectionElem::FieldNamed(name) => {
                if struct_layout.is_none() {
                    if let Some(ty_name) = current_type {
                        let base = ty_name.split('<').next().unwrap_or(ty_name);
                        if base.ends_with("::ReadOnlySpan") || base == "ReadOnlySpan" {
                            if name == "Raw" || name == "Handle" {
                                return Some("Std::Span::ReadOnlySpanPtr".to_string());
                            }
                        }
                        if base.ends_with("::Span") || base == "Span" {
                            if name == "Raw" || name == "Handle" {
                                return Some("Std::Span::SpanPtr".to_string());
                            }
                        }
                    }
                }
                let owned = name.clone();
                self.update_field_named(elem, &owned, struct_layout, union_layout)
            }
            ProjectionElem::Field(index) => self.update_field_index(*index, struct_layout),
            ProjectionElem::UnionField { index, .. } => {
                self.update_union_field(*index, union_layout)
            }
            ProjectionElem::Downcast { .. }
            | ProjectionElem::Index(_)
            | ProjectionElem::ConstantIndex { .. }
            | ProjectionElem::Deref
            | ProjectionElem::Subslice { .. } => None,
        }
    }
    pub(crate) fn update_field_named(
        &self,
        elem: &mut ProjectionElem,
        name: &str,
        struct_layout: Option<&StructLayout>,
        union_layout: Option<&UnionLayout>,
    ) -> Option<String> {
        if let Some(union_layout) = union_layout
            && let Some(field) = union_layout.views.iter().find(|f| f.name == name)
        {
            *elem = ProjectionElem::UnionField {
                index: field.index,
                name: field.name.clone(),
            };
            return self.resolve_ty_name(&field.ty);
        }

        if let Some(struct_layout) = struct_layout
            && let Some(field) = struct_layout.fields.iter().find(|f| f.matches_name(name))
        {
            return self.resolve_ty_name(&field.ty);
        }

        None
    }
    pub(crate) fn update_field_index(
        &self,
        index: u32,
        struct_layout: Option<&StructLayout>,
    ) -> Option<String> {
        if let Some(struct_layout) = struct_layout
            && let Some(field) = struct_layout.fields.iter().find(|f| f.index == index)
        {
            return self.resolve_ty_name(&field.ty);
        }

        None
    }
    pub(crate) fn update_union_field(&self, index: u32, union_layout: Option<&UnionLayout>) -> Option<String> {
        if let Some(union_layout) = union_layout
            && let Some(field) = union_layout.views.iter().find(|f| f.index == index)
        {
            return self.resolve_ty_name(&field.ty);
        }

        None
    }
    pub(crate) fn project_member_operand(
        &mut self,
        operand: Operand,
        member: &str,
        span: Option<Span>,
            ) -> Operand {
        match operand {
            Operand::Copy(mut place) => {
                place
                    .projection
                    .push(ProjectionElem::FieldNamed(member.to_string()));
                self.normalise_place(&mut place);
                if let Some(target) = self.mmio_operand_for_place(&place) {
                    self.validate_mmio_access(&target, MmioIntent::Read, span);
                    Operand::Mmio(target)
                } else {
                    Operand::Copy(place)
                }
            }
            Operand::Move(mut place) => {
                place
                    .projection
                    .push(ProjectionElem::FieldNamed(member.to_string()));
                self.normalise_place(&mut place);
                if let Some(target) = self.mmio_operand_for_place(&place) {
                    self.validate_mmio_access(&target, MmioIntent::Read, span);
                    Operand::Mmio(target)
                } else {
                    Operand::Move(place)
                }
            }
            Operand::Mmio(spec) => Operand::Mmio(spec),
            Operand::Borrow(mut borrow) => {
                borrow
                    .place
                    .projection
                    .push(ProjectionElem::FieldNamed(member.to_string()));
                self.normalise_place(&mut borrow.place);
                Operand::Borrow(borrow)
            }
            Operand::Const(constant) => {
                if let ConstValue::Symbol(symbol) = &constant.value {
                    if let Some(enum_value) = self.decimal_enum_const(symbol, member) {
                        let normalised = self.normalise_const(enum_value, span);
                        return Operand::Const(ConstOperand::new(normalised));
                    }
                }
                let repr = format!("{:?}.{member}", constant.value);
                Operand::Pending(PendingOperand {
                    category: ValueCategory::Pending,
                    repr,
                    span,
                                        info: None,
                })
            }
            Operand::Pending(mut pending) => {
                if let Some(enum_value) = self.decimal_enum_const(&pending.repr, member) {
                    let normalised = self.normalise_const(enum_value, span);
                    return Operand::Const(ConstOperand::new(normalised));
                }
                pending.repr = format!("{}.{}", pending.repr, member);
                if pending.span.is_none() {
                    pending.span = span;
                }
                Operand::Pending(pending)
            }
        }
    }
    pub(crate) fn sequence_element_ty(&self, ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Array(array) => {
                if array.rank > 1 {
                    Some(Ty::Array(ArrayTy::new(array.element.clone(), array.rank - 1)))
                } else {
                    Some((*array.element).clone())
                }
            }
            Ty::Vec(vec) => Some((*vec.element).clone()),
            Ty::Span(span) => Some((*span.element).clone()),
            Ty::ReadOnlySpan(span) => Some((*span.element).clone()),
            Ty::String | Ty::Str => Some(Ty::named("char")),
            Ty::Ref(reference) => self.sequence_element_ty(&reference.element),
            Ty::Nullable(inner) => self.sequence_element_ty(inner),
            _ => None,
        }
    }
    pub(crate) fn strip_nullable<'ty>(mut ty: &'ty Ty) -> &'ty Ty {
        while let Ty::Nullable(inner) = ty {
            ty = inner.as_ref();
        }
        ty
    }
    fn decimal_enum_const(&self, symbol: &str, member: &str) -> Option<ConstValue> {
        let canonical = format!("{}::{}", symbol.replace('.', "::"), member);
        if canonical.contains("DecimalStatus::") {
            let discriminant = match member {
                "Success" => 0,
                "Overflow" => 1,
                "DivideByZero" => 2,
                "InvalidRounding" => 3,
                "InvalidFlags" => 4,
                "InvalidPointer" => 5,
                "InvalidOperand" => 6,
                _ => return None,
            };
            return Some(ConstValue::Enum {
                type_name: "Std::Numeric::Decimal::DecimalStatus".into(),
                variant: member.to_string(),
                discriminant,
            });
        }
        if canonical.contains("DecimalIntrinsicVariant::") {
            let discriminant = match member {
                "Scalar" => 0,
                _ => return None,
            };
            return Some(ConstValue::Enum {
                type_name: "Std::Numeric::Decimal::DecimalIntrinsicVariant".into(),
                variant: member.to_string(),
                discriminant,
            });
        }
        if canonical.contains("DecimalRoundingMode::") {
            let discriminant = match member {
                "TiesToEven" => 0,
                "TowardZero" => 1,
                "AwayFromZero" => 2,
                "TowardPositive" => 3,
                "TowardNegative" => 4,
                _ => return None,
            };
            return Some(ConstValue::Enum {
                type_name: "Std::Numeric::Decimal::DecimalRoundingMode".into(),
                variant: member.to_string(),
                discriminant,
            });
        }
        if canonical.contains("DecimalVectorizeHint::") {
            let discriminant = match member {
                "None" => 0,
                "Decimal" => 1,
                _ => return None,
            };
            return Some(ConstValue::Enum {
                type_name: "Std::Numeric::Decimal::DecimalVectorizeHint".into(),
                variant: member.to_string(),
                discriminant,
            });
        }
        None
    }
    pub(crate) fn indexable_kind(&self, ty: &Ty) -> Option<IndexableKind> {
        match ty {
            Ty::Array(array) => Some(IndexableKind::Array(array.rank)),
            Ty::Vec(_) => Some(IndexableKind::Vec),
            Ty::Span(_) => Some(IndexableKind::Span),
            Ty::ReadOnlySpan(_) => Some(IndexableKind::ReadOnlySpan),
            Ty::String | Ty::Str => Some(IndexableKind::ReadOnlySpan),
            Ty::Nullable(inner) => self.indexable_kind(inner),
            _ => None,
        }
    }
    pub(crate) fn project_ty(&self, ty: &Ty, elem: &ProjectionElem) -> Option<Ty> {
        let base_ty = if let Ty::Ref(reference) = ty {
            &reference.element
        } else {
            ty
        };
        match elem {
            ProjectionElem::Field(index) => {
                let type_name = self.resolve_ty_name(base_ty)?;
                let layout = self.lookup_struct_layout_by_name(&type_name)?;
                let field = layout.fields.iter().find(|f| f.index == *index)?;
                Some(field.ty.clone())
            }
            ProjectionElem::FieldNamed(name) => {
                let type_name = self.resolve_ty_name(base_ty)?;
                if let Some(layout) = self.lookup_struct_layout_by_name(&type_name) {
                    if let Some(field) = layout.fields.iter().find(|f| f.name == *name) {
                        if std::env::var("CHIC_DEBUG_PLACE_TYPES").is_ok()
                            && (type_name.contains("ReadOnlySpan") || type_name.contains("Span"))
                        {
                            eprintln!(
                                "[chic-debug] struct layout projection {}.{} -> {}",
                                type_name,
                                name,
                                field.ty.canonical_name()
                            );
                        }
                        return Some(field.ty.clone());
                    }
                }
                if let Some(union_layout) = self.lookup_union_layout(&type_name) {
                    if let Some(view) = union_layout.views.iter().find(|view| view.name == *name)
                    {
                        return Some(view.ty.clone());
                    }
                }
                let base = type_name.split('<').next().unwrap_or(&type_name);
                let short_base = crate::mir::casts::short_type_name(base);
                if short_base == "ReadOnlySpan" || base.ends_with("::ReadOnlySpan") {
                    let result = match name.as_str() {
                        "Raw" | "Handle" => Some(Ty::named("Std::Span::ReadOnlySpanPtr")),
                        "Data" | "ptr" | "Pointer" => {
                            Some(Ty::named("Std::Runtime::Collections::ValueConstPtr"))
                        }
                        "len" | "Length" | "elem_size" | "ElementSize" | "ElementAlignment" => {
                            Some(Ty::named("usize"))
                        }
                        _ => None,
                    };
                    if let Some(ret_ty) = &result
                        && std::env::var("CHIC_DEBUG_PLACE_TYPES").is_ok()
                    {
                        eprintln!(
                            "[chic-debug] projection fallback for `{}` field `{}` -> {}",
                            type_name,
                            name,
                            ret_ty.canonical_name()
                        );
                    }
                    return result;
                }
                if short_base == "Span" || base.ends_with("::Span") {
                    let result = match name.as_str() {
                        "Raw" | "Handle" => Some(Ty::named("Std::Span::SpanPtr")),
                        "Data" | "ptr" | "Pointer" => {
                            Some(Ty::named("Std::Runtime::Collections::ValueMutPtr"))
                        }
                        "len" | "Length" | "elem_size" | "ElementSize" | "ElementAlignment" => {
                            Some(Ty::named("usize"))
                        }
                        _ => None,
                    };
                    if let Some(ret_ty) = &result
                        && std::env::var("CHIC_DEBUG_PLACE_TYPES").is_ok()
                    {
                        eprintln!(
                            "[chic-debug] projection fallback for `{}` field `{}` -> {}",
                            type_name,
                            name,
                            ret_ty.canonical_name()
                        );
                    }
                    return result;
                }
                None
            }
            ProjectionElem::UnionField { index, .. } => {
                let type_name = self.resolve_ty_name(base_ty)?;
                let layout = self.lookup_union_layout(&type_name)?;
                let view = layout.views.iter().find(|v| v.index == *index)?;
                Some(view.ty.clone())
            }
            ProjectionElem::Index(_)
            | ProjectionElem::ConstantIndex { .. } => self.sequence_element_ty(base_ty),
            ProjectionElem::Deref
            | ProjectionElem::Downcast { .. }
            | ProjectionElem::Subslice { .. } => None,
        }
    }

    fn fallback_project_ty(&self, ty: &Ty, elem: &ProjectionElem) -> Option<Ty> {
        match elem {
            ProjectionElem::FieldNamed(name) => {
                let type_name = self.resolve_ty_name(ty)?;
                let base = type_name.split('<').next().unwrap_or(&type_name);
                let short_base = crate::mir::casts::short_type_name(base);
                if short_base == "ReadOnlySpan" || base.ends_with("::ReadOnlySpan") {
                    return match name.as_str() {
                        "Raw" | "Handle" => Some(Ty::named("Std::Span::ReadOnlySpanPtr")),
                        "Data" | "ptr" | "Pointer" => {
                            Some(Ty::named("Std::Runtime::Collections::ValueConstPtr"))
                        }
                        "len" | "Length" | "elem_size" | "ElementSize" | "ElementAlignment" => {
                            Some(Ty::named("usize"))
                        }
                        _ => None,
                    };
                }
                if short_base == "Span" || base.ends_with("::Span") {
                    return match name.as_str() {
                        "Raw" | "Handle" => Some(Ty::named("Std::Span::SpanPtr")),
                        "Data" | "ptr" | "Pointer" => {
                            Some(Ty::named("Std::Runtime::Collections::ValueMutPtr"))
                        }
                        "len" | "Length" | "elem_size" | "ElementSize" | "ElementAlignment" => {
                            Some(Ty::named("usize"))
                        }
                        _ => None,
                    };
                }
                None
            }
            ProjectionElem::Index(_) | ProjectionElem::ConstantIndex { .. } => self.sequence_element_ty(ty),
            ProjectionElem::Deref => match Self::strip_nullable(ty) {
                Ty::Ref(reference) => Some(reference.element.clone()),
                Ty::Pointer(pointer) => Some(pointer.element.clone()),
                _ => None,
            },
            ProjectionElem::Field(_)
            | ProjectionElem::UnionField { .. }
            | ProjectionElem::Downcast { .. }
            | ProjectionElem::Subslice { .. } => None,
        }
    }
}
