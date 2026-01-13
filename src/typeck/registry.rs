mod calls;
mod discovery;
mod effects;
mod extensions;
pub(super) mod hooks;
mod numeric;
mod orchestrator;
mod patterns;
mod signatures;
mod statements;
pub(crate) use calls::*;
pub(crate) use extensions::*;
pub(super) use hooks::{RegistryHooks, RegistryIndex};
pub(super) use signatures::{qualify, signature_from, signature_from_extension};
pub(crate) use statements::*;
#[cfg(test)]
mod tests;

use super::arena::{
    BaseTypeBinding, ConstructorInfo, FunctionSignature, InterfaceDefaultKind,
    InterfaceDefaultProvider, MethodDispatchInfo, PropertyInfo, TypeChecker, TypeInfo, TypeKind,
};
use super::diagnostics::{self, codes};
use super::helpers::{base_type_name, type_names_equivalent};
use crate::accessibility::{AccessContext, check_access};
use crate::frontend::ast::{
    BindingModifier, Block, DelegateDecl, Expression, FunctionDecl, InterfaceDecl, InterfaceMember,
    Parameter, PropertyAccessorKind, Signature, Statement, StatementKind, TypeExpr, Variance,
    Visibility,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::frontend::parser::parse_type_expression_text_with_span;
use crate::mir::{AutoTraitStatus, ConstValue};
use crate::syntax::expr::{ExprNode, LiteralConst, NewExpr, NewInitializer, SizeOfOperand};
use std::collections::{HashSet, VecDeque};

impl<'a> TypeChecker<'a> {
    fn validate_interface_variance(&mut self, iface_name: &str, iface: &InterfaceDecl) {
        let Some(generics) = iface.generics.as_ref() else {
            return;
        };
        let type_params: Vec<_> = generics
            .params
            .iter()
            .filter_map(|param| {
                param
                    .as_type()
                    .map(|data| (param.name.as_str(), data.variance))
            })
            .collect();
        if type_params.is_empty() {
            return;
        }

        for member in &iface.members {
            match member {
                InterfaceMember::Method(method) => {
                    let member_name = format!("{iface_name}::{}", method.name);
                    if !is_void_type(&method.signature.return_type) {
                        self.enforce_variance_positions(
                            "interface",
                            iface_name,
                            &member_name,
                            &method.signature.return_type,
                            &type_params,
                            VarianceUsage::Output,
                            None,
                        );
                    }
                    for param in &method.signature.parameters {
                        let usage = match param.binding {
                            BindingModifier::Out => VarianceUsage::Output,
                            BindingModifier::Ref => VarianceUsage::Both,
                            _ => VarianceUsage::Input,
                        };
                        self.enforce_variance_positions(
                            "interface",
                            iface_name,
                            &member_name,
                            &param.ty,
                            &type_params,
                            usage,
                            None,
                        );
                    }
                }
                InterfaceMember::Property(property) => {
                    let member_name = format!("{iface_name}::{}", property.name);
                    if property.accessor(PropertyAccessorKind::Get).is_some() {
                        self.enforce_variance_positions(
                            "interface",
                            iface_name,
                            &member_name,
                            &property.ty,
                            &type_params,
                            VarianceUsage::Output,
                            property.span,
                        );
                    }
                    if property.accessor(PropertyAccessorKind::Set).is_some()
                        || property.accessor(PropertyAccessorKind::Init).is_some()
                    {
                        self.enforce_variance_positions(
                            "interface",
                            iface_name,
                            &member_name,
                            &property.ty,
                            &type_params,
                            VarianceUsage::Input,
                            property.span,
                        );
                    }
                }
                InterfaceMember::Const(_) | InterfaceMember::AssociatedType(_) => {}
            }
        }
    }

    fn validate_delegate_variance(&mut self, delegate_name: &str, delegate: &DelegateDecl) {
        let Some(generics) = delegate.generics.as_ref() else {
            return;
        };
        let type_params: Vec<_> = generics
            .params
            .iter()
            .filter_map(|param| {
                param
                    .as_type()
                    .map(|data| (param.name.as_str(), data.variance))
            })
            .collect();
        if type_params.is_empty() {
            return;
        }

        if !is_void_type(&delegate.signature.return_type) {
            self.enforce_variance_positions(
                "delegate",
                delegate_name,
                &format!("{delegate_name}::Invoke"),
                &delegate.signature.return_type,
                &type_params,
                VarianceUsage::Output,
                delegate.span,
            );
        }

        for param in &delegate.signature.parameters {
            let usage = match param.binding {
                BindingModifier::Out => VarianceUsage::Output,
                BindingModifier::Ref => VarianceUsage::Both,
                _ => VarianceUsage::Input,
            };
            self.enforce_variance_positions(
                "delegate",
                delegate_name,
                &format!("{delegate_name}::Invoke"),
                &param.ty,
                &type_params,
                usage,
                delegate.span,
            );
        }
    }

    fn enforce_variance_positions(
        &mut self,
        container_kind: &str,
        container_name: &str,
        member_name: &str,
        ty: &TypeExpr,
        params: &[(&str, Variance)],
        usage: VarianceUsage,
        span: Option<Span>,
    ) {
        for (param_name, variance) in params {
            if !type_expr_mentions_parameter(ty, param_name) {
                continue;
            }
            if variance_allows_usage(*variance, usage) {
                continue;
            }
            let variance_label = match variance {
                Variance::Covariant => "covariant (`out`)",
                Variance::Contravariant => "contravariant (`in`)",
                Variance::Invariant => "invariant",
            };
            let usage_label = match usage {
                VarianceUsage::Input => "input",
                VarianceUsage::Output => "output",
                VarianceUsage::Both => "input/output",
            };
            self.emit_error(
                codes::GENERIC_CONSTRAINT_VIOLATION,
                span,
                format!(
                    "{container_kind} `{container_name}` declares `{param_name}` as {variance_label}, but member `{member_name}` uses it in an {usage_label} position"
                ),
            );
        }
    }
}

impl<'a> TypeChecker<'a> {
    fn validate_async_return_type(
        &mut self,
        function_name: &str,
        signature: &Signature,
        namespace: Option<&str>,
        context_type: Option<&str>,
        span: Option<Span>,
    ) -> Option<Option<TypeExpr>> {
        let resolved =
            match self.resolve_type_for_expr(&signature.return_type, namespace, context_type) {
                ImportResolution::Found(name) => name,
                ImportResolution::Ambiguous(_) | ImportResolution::NotFound => return None,
            };

        if resolved.as_str() != "Std::Async::Task" {
            self.emit_error(
                codes::ASYNC_RETURN_TYPE_INVALID,
                span,
                                format!(
                    "async function `{function_name}` must return `Std.Async.Task` or `Std.Async.Task<T>`"
                ),
            );
            return None;
        }

        if let Some(args) = signature.return_type.generic_arguments() {
            if args.len() != 1 {
                self.emit_error(
                    codes::ASYNC_RETURN_TYPE_INVALID,
                    span,
                                        format!(
                        "async function `{function_name}` must specify exactly one type argument for `Task`"
                    ),
                );
                return None;
            }
            if let Some(ty_arg) = args[0].ty().cloned() {
                return Some(Some(ty_arg));
            }
            self.emit_error(
                codes::ASYNC_RETURN_TYPE_INVALID,
                span,
                format!(
                    "async function `{function_name}` must supply a type argument for `Task<T>`"
                ),
            );
            return None;
        }

        Some(None)
    }

    fn validate_function_body(
        &mut self,
        function_name: &str,
        body: &'a Block,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        self.validate_block(function_name, body, namespace, context_type);
    }

    fn queue_body_validation(
        &mut self,
        function_name: &str,
        body: &'a Block,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        self.pending_bodies.push(crate::typeck::arena::PendingBody {
            name: function_name.to_string(),
            body,
            namespace: namespace.map(str::to_string),
            context_type: context_type.map(str::to_string),
        });
    }

    pub(super) fn run_pending_body_validations(&mut self) {
        let pending = std::mem::take(&mut self.pending_bodies);
        for pending in pending {
            self.validate_function_body(
                &pending.name,
                pending.body,
                pending.namespace.as_deref(),
                pending.context_type.as_deref(),
            );
        }
    }

    fn validate_const_function(
        &mut self,
        function_name: &str,
        function: &FunctionDecl,
        _namespace: Option<&str>,
        _owner: Option<&str>,
    ) {
        if !function.is_constexpr {
            return;
        }

        let mut signature_errors: Vec<String> = Vec::new();
        if function.is_async {
            signature_errors.push("`async` functions cannot be evaluated at compile time".into());
        }
        if function.is_extern {
            signature_errors.push("`extern` const functions are not supported".into());
        }
        if function
            .generics
            .as_ref()
            .is_some_and(|generics| !generics.params.is_empty())
        {
            signature_errors.push("const functions may not declare generic parameters".into());
        }
        if function.is_unsafe {
            signature_errors.push("`unsafe` const functions are not supported".into());
        }
        if function.signature.throws.is_some() {
            signature_errors.push("const functions may not declare `throws` effects".into());
        }
        for param in &function.signature.parameters {
            if matches!(param.binding, BindingModifier::Ref | BindingModifier::Out) {
                let binding = match param.binding {
                    BindingModifier::Ref => "ref",
                    BindingModifier::Out => "out",
                    BindingModifier::In => "in",
                    BindingModifier::Value => "value",
                };
                signature_errors.push(format!(
                    "parameter `{}` uses `{binding}` binding, which is not supported in const functions",
                    param.name
                ));
            }
        }

        let body_span = function.body.as_ref().and_then(|body| body.span);
        if !signature_errors.is_empty() {
            self.emit_error(
                codes::CONST_FN_SIGNATURE,
                body_span,
                format!(
                    "const fn `{function_name}` cannot be compiled: {}",
                    signature_errors.join("; ")
                ),
            );
        }
        if let Some(body) = &function.body {
            self.validate_const_fn_block(function_name, body);
        } else {
            self.emit_error(
                codes::CONST_FN_SIGNATURE,
                body_span,
                format!("const fn `{function_name}` requires a body"),
            );
        }
    }

    fn validate_const_fn_block(&mut self, function_name: &str, block: &Block) {
        for statement in &block.statements {
            self.validate_const_fn_statement(function_name, statement);
        }
    }

    fn validate_const_fn_statement(&mut self, function_name: &str, statement: &Statement) {
        match &statement.kind {
            StatementKind::Block(inner) => {
                self.validate_const_fn_block(function_name, inner);
            }
            StatementKind::Empty => {}
            StatementKind::ConstDeclaration(const_stmt) => {
                for declarator in &const_stmt.declaration.declarators {
                    self.validate_const_fn_expression(function_name, &declarator.initializer);
                }
            }
            StatementKind::VariableDeclaration(decl) => {
                for declarator in &decl.declarators {
                    let Some(initializer) = &declarator.initializer else {
                        self.emit_error(
                            codes::CONST_FN_BODY,
                            statement.span,
                            format!(
                                "variable `{}` in const fn `{function_name}` requires an initializer",
                                declarator.name
                            ),
                        );
                        continue;
                    };
                    self.validate_const_fn_expression(function_name, initializer);
                }
            }
            StatementKind::Expression(expr) => {
                self.validate_const_fn_expression(function_name, expr);
            }
            StatementKind::Return { expression } => {
                if let Some(expr) = expression {
                    self.validate_const_fn_expression(function_name, expr);
                }
            }
            StatementKind::If(if_stmt) => {
                self.validate_const_fn_expression(function_name, &if_stmt.condition);
                self.validate_const_fn_statement(function_name, if_stmt.then_branch.as_ref());
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.validate_const_fn_statement(function_name, else_branch.as_ref());
                }
            }
            other => {
                self.emit_error(
                    codes::CONST_FN_BODY,
                    statement.span,
                    format!(
                        "const fn `{function_name}` does not support `{}` statements",
                        const_fn_statement_kind_name(other)
                    ),
                );
            }
        }
    }

    fn validate_const_fn_expression(&mut self, function_name: &str, expr: &Expression) {
        let Some(node) = expr.node.as_ref() else {
            self.emit_error(
                codes::CONST_FN_BODY,
                expr.span,
                format!("const fn `{function_name}` contains an unparsable expression"),
            );
            return;
        };
        self.validate_const_fn_node(function_name, node, expr.span);
    }

    fn validate_const_fn_node(&mut self, function_name: &str, node: &ExprNode, span: Option<Span>) {
        match node {
            ExprNode::Literal(_) | ExprNode::Identifier(_) => {}
            ExprNode::Unary { expr: inner, .. } => {
                self.validate_const_fn_node(function_name, inner, span);
            }
            ExprNode::Binary { left, right, .. } => {
                self.validate_const_fn_node(function_name, left, span);
                self.validate_const_fn_node(function_name, right, span);
            }
            ExprNode::Parenthesized(inner) => {
                self.validate_const_fn_node(function_name, inner, span);
            }
            ExprNode::Cast { expr: inner, .. } => {
                self.validate_const_fn_node(function_name, inner, span);
            }
            ExprNode::Call { callee, args, .. } => {
                if expr_path_segments(callee).is_none() {
                    self.emit_error(
                        codes::CONST_FN_BODY,
                        span,
                        format!("const fn `{function_name}` call target must be a simple path"),
                    );
                }
                for arg in args {
                    let arg_span = arg.value_span.or(arg.span).or(span);
                    self.validate_const_fn_node(function_name, &arg.value, arg_span);
                }
            }
            ExprNode::Assign { target, value, .. } => {
                if !matches!(target.as_ref(), ExprNode::Identifier(_)) {
                    self.emit_error(
                        codes::CONST_FN_BODY,
                        span,
                            format!(
                                "assignments in const fn `{function_name}` must target local identifiers"
                            ),
                        );
                }
                self.validate_const_fn_node(function_name, value, span);
            }
            ExprNode::Member { base, .. } => {
                self.validate_const_fn_node(function_name, base, span);
            }
            ExprNode::SizeOf(operand) => match operand {
                SizeOfOperand::Value(inner) => {
                    self.validate_const_fn_node(function_name, inner, span);
                }
                SizeOfOperand::Type(_) => {}
            },
            ExprNode::AlignOf(operand) => match operand {
                SizeOfOperand::Value(inner) => {
                    self.validate_const_fn_node(function_name, inner, span);
                }
                SizeOfOperand::Type(_) => {}
            },
            ExprNode::NameOf(_operand) => {}
            ExprNode::Quote(_) => {}
            other => {
                let label = match other {
                    ExprNode::Await { .. } => "await",
                    ExprNode::Lambda(_) => "lambda",
                    ExprNode::New(_) => "object construction",
                    ExprNode::Index { .. } => "index",
                    ExprNode::Tuple(_) => "tuple literal",
                    ExprNode::InterpolatedString(_) => "interpolated string",
                    ExprNode::TryPropagate { .. } => "try/await",
                    ExprNode::Conditional { .. } => "conditional",
                    ExprNode::IsPattern { .. } => "pattern match",
                    ExprNode::Ref { .. } => "borrow",
                    ExprNode::Throw { .. } => "throw",
                    _ => "expression",
                };
                self.emit_error(
                    codes::CONST_FN_BODY,
                    span,
                    format!("const fn `{function_name}` cannot use {label} expressions"),
                );
            }
        }
    }

    fn type_hierarchy(&self, type_name: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back(type_name.to_string());
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            result.push(current.clone());
            if let Some(info) = self.resolve_type_info(&current) {
                if let TypeKind::Class { bases, .. } = &info.kind {
                    for base in bases {
                        queue.push_back(base.name.clone());
                    }
                }
            }
        }
        result
    }

    pub(super) fn enforce_base_accessibility(&mut self) {
        let type_names: Vec<String> = self.types.keys().cloned().collect();
        for name in type_names {
            let Some(infos) = self.types.get(&name).cloned() else {
                continue;
            };
            let namespace = self.namespace_of_type(&name);
            for info in infos {
                let bases = match &info.kind {
                    TypeKind::Class { bases, .. }
                    | TypeKind::Struct { bases, .. }
                    | TypeKind::Interface { bases, .. } => Some(bases.clone()),
                    _ => None,
                };
                let Some(bases) = bases else { continue };
                for base in bases {
                    if let Some(base_info) = self.resolve_type_info(&base.name).cloned() {
                        if !self.type_accessible_from_current(
                            base_info.visibility,
                            &base.name,
                            None,
                            namespace.as_deref(),
                            Some(name.as_str()),
                        ) {
                            self.emit_error(
                                codes::INACCESSIBLE_BASE,
                                base.expr.span,
                                format!(
                                    "type `{}` cannot inherit from `{}` because it is not accessible from this package",
                                    name, base.name
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    fn access_context_type<'s>(&self, context_type: Option<&'s str>) -> Option<&'s str> {
        let Some(context_type) = context_type else {
            return None;
        };
        if self.has_type(context_type) {
            return Some(context_type);
        }
        let Some((owner, _)) = context_type.rsplit_once("::") else {
            return None;
        };
        if self.has_type(owner) {
            Some(owner)
        } else {
            None
        }
    }

    fn type_accessible_from_current(
        &self,
        visibility: Visibility,
        owner: &str,
        owner_namespace: Option<&str>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> bool {
        let access_context = self.access_context_type(context_type);
        let ctx = AccessContext {
            current_package: self.context_package(access_context.or(context_type)),
            current_type: access_context,
            current_namespace: namespace,
            receiver_type: None,
            is_instance: false,
        };
        check_access(
            visibility,
            owner,
            self.package_of_owner(owner),
            owner_namespace,
            &ctx,
            |a, b| type_names_equivalent(a, b),
            |ty, base| self.type_is_subclass_of_name(ty, base),
        )
        .allowed
    }

    fn is_member_accessible(
        &self,
        visibility: Visibility,
        owner: &str,
        member_namespace: Option<&str>,
        namespace: Option<&str>,
        context_type: Option<&str>,
        receiver_type: Option<&str>,
        is_instance: bool,
    ) -> bool {
        let owned_namespace = self.namespace_of_type(owner);
        let owner_namespace = member_namespace.or_else(|| owned_namespace.as_deref());
        let access_context = self.access_context_type(context_type);
        let ctx = AccessContext {
            current_package: self.context_package(access_context.or(context_type)),
            current_type: access_context,
            current_namespace: namespace,
            receiver_type,
            is_instance,
        };
        check_access(
            visibility,
            owner,
            self.package_of_owner(owner),
            owner_namespace,
            &ctx,
            |a, b| type_names_equivalent(a, b),
            |ty, base| self.type_is_subclass_of_name(ty, base),
        )
        .allowed
    }

    fn validate_public_type_expr(
        &mut self,
        member_name: &str,
        member_visibility: Visibility,
        ty: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        if !matches!(member_visibility, Visibility::Public) {
            return;
        }
        let resolved = match self.resolve_type_for_expr(ty, namespace, context_type) {
            ImportResolution::Found(name) => name,
            ImportResolution::Ambiguous(_) | ImportResolution::NotFound => return,
        };
        if let Some(info) = self.resolve_type_info(&resolved) {
            if !matches!(info.visibility, Visibility::Public) {
                self.emit_error(
                    codes::PUBLIC_MEMBER_INACCESSIBLE_TYPE,
                    ty.span,
                    format!(
                        "public member `{member_name}` exposes `{}` which is less accessible",
                        resolved
                    ),
                );
            }
        }
    }

    fn validate_public_signature(
        &mut self,
        member_name: &str,
        visibility: Visibility,
        return_ty: Option<&TypeExpr>,
        params: &[Parameter],
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        if let Some(ret) = return_ty {
            self.validate_public_type_expr(member_name, visibility, ret, namespace, context_type);
        }
        for param in params {
            self.validate_public_type_expr(
                member_name,
                visibility,
                &param.ty,
                namespace,
                context_type,
            );
        }
    }

    fn required_members_for_type(&self, type_name: &str) -> HashSet<String> {
        let mut members = HashSet::new();
        for candidate in self.type_hierarchy(type_name) {
            for field in self.symbol_index.required_field_names(&candidate) {
                members.insert(field);
            }
            for property in self.symbol_index.required_property_names(&candidate) {
                members.insert(property);
            }
        }
        members
    }

    fn constructor_visibility(
        metadata: &[ConstructorInfo],
        owner: &str,
        qualified: &str,
    ) -> Visibility {
        if let Some(index) = Self::parse_constructor_index(owner, qualified)
            && let Some(info) = metadata.get(index)
        {
            info.visibility
        } else {
            Visibility::Public
        }
    }

    fn parse_constructor_index(owner: &str, qualified: &str) -> Option<usize> {
        let prefix = format!("{owner}::init#");
        qualified
            .strip_prefix(&prefix)
            .and_then(|suffix| suffix.parse::<usize>().ok())
    }

    fn check_new_expression(
        &mut self,
        function_name: &str,
        new_expr: &NewExpr,
        span: Option<Span>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> Option<ConstructedTypeInfo> {
        if new_expr.type_name.trim_start().starts_with('[') || new_expr.type_name.trim().is_empty()
        {
            self.emit_error(
                codes::ARRAY_IMPLICIT_TYPE_UNSUPPORTED,
                new_expr.type_span.or(span).or(new_expr.span),
                "array creation requires an explicit element type (`new T[] { ... }`)",
            );
            return None;
        }
        let type_span = new_expr.type_span.or(new_expr.span).or(span);
        let type_offset = type_span.map_or(0, |sp| sp.start);
        let type_file = type_span.map(|sp| sp.file_id);
        let Some(type_expr) =
            parse_type_expression_text_with_span(&new_expr.type_name, type_file, type_offset)
        else {
            self.emit_error(
                codes::UNKNOWN_TYPE,
                type_span,
                format!(
                    "`{}` is not a valid type name in `new` expressions",
                    new_expr.type_name
                ),
            );
            return None;
        };
        if type_expr.array_ranks().next().is_some() {
            let resolved = self.resolve_type_for_expr(&type_expr, namespace, context_type);
            let resolved_name = match resolved {
                ImportResolution::Found(name) => name,
                ImportResolution::Ambiguous(candidates) => {
                    self.emit_error(
                        codes::AMBIGUOUS_TYPE,
                        type_span,
                        format!(
                            "type `{}` resolves to multiple candidates: {}",
                            new_expr.type_name,
                            candidates.join(", ")
                        ),
                    );
                    return None;
                }
                ImportResolution::NotFound => {
                    let candidate = self.canonical_type_name(&type_expr);
                    if self.context_declares_generic(context_type, &candidate)
                        || self.context_declares_generic(Some(function_name), &candidate)
                    {
                        candidate
                    } else {
                        self.emit_error(
                            codes::UNKNOWN_TYPE,
                            type_span,
                            format!(
                                "type `{}` is not known at this call site",
                                new_expr.type_name
                            ),
                        );
                        return None;
                    }
                }
            };
            return self.check_array_new(new_expr, &type_expr, span, &resolved_name);
        }
        let mut treat_as_value_type = self.type_is_value_type(&type_expr, context_type);
        let resolved = match self.resolve_type_for_expr(&type_expr, namespace, context_type) {
            ImportResolution::Found(name) => name,
            ImportResolution::Ambiguous(candidates) => {
                self.emit_error(
                    codes::AMBIGUOUS_TYPE,
                    type_span,
                    format!(
                        "type `{}` resolves to multiple candidates: {}",
                        new_expr.type_name,
                        candidates.join(", ")
                    ),
                );
                return None;
            }
            ImportResolution::NotFound => {
                if treat_as_value_type {
                    self.canonical_type_name(&type_expr)
                } else {
                    self.emit_error(
                        codes::UNKNOWN_TYPE,
                        type_span,
                        format!(
                            "type `{}` is not known at this call site",
                            new_expr.type_name
                        ),
                    );
                    return None;
                }
            }
        };

        let expected_arity = type_expr.generic_arguments().map(|args| args.len());
        let type_info = self
            .resolve_type_info_with_arity(&resolved, expected_arity)
            .or_else(|| self.resolve_type_info(&resolved))
            .cloned();
        let diagnostics_before = self.diagnostics.len();
        if let Some(info) = type_info.as_ref() {
            if let Some(args) = type_expr.generic_arguments() {
                self.validate_generic_arguments(&resolved, info, args, context_type, type_span);
            } else if info
                .generics
                .as_ref()
                .is_some_and(|params| !params.params.is_empty())
            {
                let expected = info
                    .generics
                    .as_ref()
                    .map_or(0, |params| params.params.len());
                self.emit_error(
                    codes::GENERIC_ARGUMENT_MISMATCH,
                    type_span,
                    format!(
                        "type `{}` requires {} type argument{}",
                        resolved,
                        expected,
                        if expected == 1 { "" } else { "s" }
                    ),
                );
            }
        }
        if self.diagnostics.len() > diagnostics_before {
            return None;
        }
        if let Some(info) = type_info.as_ref() {
            if let TypeKind::Class { is_abstract, .. } = &info.kind
                && *is_abstract
            {
                self.emit_error(
                    codes::ABSTRACT_INSTANTIATION,
                    type_span,
                    format!("cannot construct abstract class `{resolved}`"),
                );
                return None;
            }
        }
        let mut constructor_metadata = Vec::new();
        let mut is_class_type = false;
        let mut is_record_type = false;
        if let Some(info) = type_info.as_ref() {
            match &info.kind {
                TypeKind::Class { constructors, .. } => {
                    is_class_type = true;
                    constructor_metadata = constructors.clone();
                }
                TypeKind::Struct {
                    constructors,
                    is_record,
                    ..
                } => {
                    treat_as_value_type = true;
                    constructor_metadata = constructors.clone();
                    is_record_type = *is_record;
                }
                _ => {}
            }
        }
        if matches!(
            resolved.as_str(),
            "ChicStr" | "Std::Runtime::Native::ChicStr" | "Std.Runtime.Native.ChicStr"
        ) {
            treat_as_value_type = true;
        }

        if !treat_as_value_type
            && !is_class_type
            && std::env::var_os("CHIC_DEBUG_NS_TYPES").is_some()
        {
            eprintln!(
                "[chic-debug] new target `{resolved}` missing class/struct info; candidates: {:?}",
                self.types.keys().collect::<Vec<_>>()
            );
        }
        if !treat_as_value_type && !is_class_type {
            self.emit_error(
                codes::CONSTRUCTOR_TARGET_INVALID,
                type_span,
                format!(
                    "type `{resolved}` cannot be constructed; only structs, classes, and value types support `new`"
                ),
            );
            return None;
        }

        let raw_candidates = self.symbol_index.constructor_overloads(&resolved);
        let mut candidates = Vec::new();
        for symbol in raw_candidates {
            let owned = symbol.clone();
            let visibility =
                Self::constructor_visibility(&constructor_metadata, &resolved, &owned.qualified);
            candidates.push(ConstructorCandidate {
                visibility,
                symbol: owned,
            });
        }
        let implicit_default_allowed = treat_as_value_type || constructor_metadata.is_empty();
        let metadata_missing = type_info.is_none();
        if !self.check_constructor_applicability(
            &resolved,
            new_expr,
            &candidates,
            implicit_default_allowed,
            namespace,
            context_type,
            span,
            metadata_missing,
        ) {
            return None;
        }

        Some(ConstructedTypeInfo {
            name: resolved,
            is_value_type: treat_as_value_type,
            is_record: is_record_type,
        })
    }

    fn check_array_new(
        &mut self,
        new_expr: &NewExpr,
        type_expr: &TypeExpr,
        span: Option<Span>,
        resolved_name: &str,
    ) -> Option<ConstructedTypeInfo> {
        let type_span = new_expr.type_span.or(new_expr.span).or(span);
        let ranks: Vec<_> = type_expr.array_ranks().collect();
        if ranks.len() > 1 || ranks.first().is_some_and(|rank| rank.dimensions > 1) {
            self.emit_error(
                codes::ARRAY_RANK_UNSUPPORTED,
                type_span,
                "multi-dimensional array ranks are not supported; use jagged arrays (`T[][]`) instead",
            );
            return None;
        }
        if let Some(lengths) = new_expr.array_lengths.as_ref() {
            if lengths.len() > 1 {
                self.emit_error(
                    codes::ARRAY_RANK_UNSUPPORTED,
                    type_span,
                    "multi-dimensional array lengths are not supported; use jagged arrays (`T[][]`) instead",
                );
                return None;
            }
        }
        if !new_expr.args.is_empty() {
            self.emit_error(
                codes::ARRAY_INITIALIZER_UNSUPPORTED,
                new_expr.arguments_span.or(span).or(new_expr.span),
                "array creation does not accept constructor arguments; provide a length (`new T[n]`) or an initializer list (`new T[] { ... }`)",
            );
            return None;
        }

        let mut initializer_len = None;
        if let Some(initializer) = &new_expr.initializer {
            match initializer {
                NewInitializer::Object {
                    span: init_span, ..
                } => {
                    self.emit_error(
                        codes::ARRAY_INITIALIZER_UNSUPPORTED,
                        init_span.or(type_span),
                        "object initializers are not supported for arrays",
                    );
                    return None;
                }
                NewInitializer::Collection { elements, .. } => {
                    initializer_len = Some(elements.len());
                }
            }
        }

        let explicit_length_expr = new_expr
            .array_lengths
            .as_ref()
            .and_then(|lengths| lengths.get(0));

        if initializer_len.is_none() && explicit_length_expr.is_none() {
            self.emit_error(
                codes::ARRAY_LENGTH_REQUIRED,
                type_span,
                "array creation requires either a length (`new T[n]`) or an initializer list (`new T[] { ... }`)",
            );
            return None;
        }

        if let Some(init_count) = initializer_len {
            if let Some(length_expr) = explicit_length_expr {
                match self.eval_array_length_literal(length_expr) {
                    Some(value) if value == init_count => {}
                    Some(_) => {
                        self.emit_error(
                            codes::ARRAY_LENGTH_MISMATCH,
                            type_span,
                            format!(
                                "array length does not match initializer element count ({init_count})"
                            ),
                        );
                        return None;
                    }
                    None => {
                        self.emit_error(
                            codes::ARRAY_LENGTH_NONCONST,
                            type_span,
                            "array length must be a compile-time constant when an initializer list is provided",
                        );
                        return None;
                    }
                }
            }
        }

        Some(ConstructedTypeInfo {
            name: resolved_name.to_string(),
            is_value_type: true,
            is_record: false,
        })
    }

    fn eval_array_length_literal(&self, expr: &ExprNode) -> Option<usize> {
        let ExprNode::Literal(LiteralConst { value, .. }) = expr else {
            return None;
        };
        match value {
            ConstValue::UInt(value) if *value <= usize::MAX as u128 => Some(*value as usize),
            ConstValue::Int(value) | ConstValue::Int32(value)
                if *value >= 0 && (*value as u128) <= usize::MAX as u128 =>
            {
                Some(*value as usize)
            }
            _ => None,
        }
    }

    pub(super) fn validate_atomic_type_argument(
        &mut self,
        usage: &str,
        arg: &TypeExpr,
        span: Option<Span>,
    ) {
        let type_name = self.canonical_type_name(arg);
        let traits = self.type_layouts.resolve_auto_traits(&type_name);

        let mut missing = Vec::new();
        let mut unknown = Vec::new();

        match traits.thread_safe {
            AutoTraitStatus::Yes => {}
            AutoTraitStatus::No => missing.push("ThreadSafe"),
            AutoTraitStatus::Unknown => unknown.push("ThreadSafe"),
        }

        match traits.shareable {
            AutoTraitStatus::Yes => {}
            AutoTraitStatus::No => missing.push("Shareable"),
            AutoTraitStatus::Unknown => unknown.push("Shareable"),
        }

        if missing.is_empty() && unknown.is_empty() {
            return;
        }

        let message = if !missing.is_empty() {
            format!(
                "type `{}` stored in `{}` must implement {}",
                type_name,
                usage,
                join_trait_names(&missing)
            )
        } else {
            format!(
                "cannot prove type `{}` stored in `{}` implements {}",
                type_name,
                usage,
                join_trait_names(&unknown)
            )
        };

        self.emit_error(codes::ATOMIC_INNER_THREADSAFE, span, message);
    }

    fn canonical_type_name(&self, ty: &TypeExpr) -> String {
        if !ty.base.is_empty() {
            ty.base.join("::")
        } else {
            ty.name.replace('.', "::")
        }
    }

    pub(super) fn is_atomic_type(name: &str) -> bool {
        matches!(
            name.replace('.', "::").as_str(),
            "Std::Sync::Atomic" | "std::sync::Atomic"
        )
    }
}

fn join_trait_names(names: &[&str]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].to_string(),
        2 => format!("{} and {}", names[0], names[1]),
        _ => names.join(", "),
    }
}

fn const_fn_statement_kind_name(kind: &StatementKind) -> &'static str {
    match kind {
        StatementKind::While { .. } => "while",
        StatementKind::DoWhile { .. } => "do-while",
        StatementKind::For(_) => "for",
        StatementKind::Foreach(_) => "foreach",
        StatementKind::Switch(_) => "switch",
        StatementKind::Try(_) => "try",
        StatementKind::Region { .. } => "region",
        StatementKind::Using(_) => "using",
        StatementKind::Lock { .. } => "lock",
        StatementKind::Checked { .. } => "checked",
        StatementKind::Atomic { .. } => "atomic",
        StatementKind::Unchecked { .. } => "unchecked",
        StatementKind::YieldReturn { .. } => "yield return",
        StatementKind::YieldBreak => "yield break",
        StatementKind::Fixed(_) => "fixed",
        StatementKind::Unsafe { .. } => "unsafe",
        StatementKind::Break => "break",
        StatementKind::Continue => "continue",
        StatementKind::Goto(_) => "goto",
        StatementKind::Throw { .. } => "throw",
        StatementKind::Labeled { .. } => "labeled",
        StatementKind::ConstDeclaration(_) => "const",
        StatementKind::VariableDeclaration(_) => "variable",
        StatementKind::Expression(_) => "expression",
        StatementKind::Return { .. } => "return",
        StatementKind::Block(_) => "block",
        StatementKind::Empty => "empty",
        StatementKind::If(_) => "if",
        StatementKind::LocalFunction(_) => "local function",
    }
}

pub(super) fn returns_self_value(ty: &TypeExpr) -> bool {
    if ty.is_trait_object() || ty.is_tuple() || ty.is_fn() {
        return false;
    }
    if ty.pointer_depth() > 0 || ty.array_ranks().next().is_some() {
        return false;
    }
    matches!(ty.base.last(), Some(segment) if segment == "Self")
}

#[derive(Clone, Copy)]
enum VarianceUsage {
    Input,
    Output,
    Both,
}

fn variance_allows_usage(variance: Variance, usage: VarianceUsage) -> bool {
    match (variance, usage) {
        (Variance::Invariant, _) => true,
        (Variance::Covariant, VarianceUsage::Output) => true,
        (Variance::Contravariant, VarianceUsage::Input) => true,
        _ => false,
    }
}

fn is_void_type(ty: &TypeExpr) -> bool {
    base_type_name(&ty.name).eq_ignore_ascii_case("void")
}

fn type_expr_mentions_parameter(ty: &TypeExpr, param: &str) -> bool {
    if ty.generic_arguments().is_none()
        && ty.tuple_elements().is_none()
        && ty.fn_signature().is_none()
        && ty.trait_object().is_none()
        && ty.name == param
    {
        return true;
    }

    if let Some(args) = ty.generic_arguments() {
        for arg in args {
            if let Some(arg_ty) = arg.ty() {
                if type_expr_mentions_parameter(arg_ty, param) {
                    return true;
                }
            }
        }
    }

    if let Some(elements) = ty.tuple_elements() {
        if elements
            .iter()
            .any(|element| type_expr_mentions_parameter(element, param))
        {
            return true;
        }
    }

    if let Some(sig) = ty.fn_signature() {
        if type_expr_mentions_parameter(&sig.return_type, param)
            || sig
                .params
                .iter()
                .any(|param_ty| type_expr_mentions_parameter(param_ty, param))
        {
            return true;
        }
    }

    if let Some(obj) = ty.trait_object() {
        if obj
            .bounds
            .iter()
            .any(|bound| type_expr_mentions_parameter(bound, param))
        {
            return true;
        }
    }

    false
}
