use crate::frontend::ast::Visibility;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessFailure {
    Private,
    InternalPackage,
    ProtectedInheritance,
    ProtectedReceiver,
    ProtectedInternalUnavailable,
    PrivateProtectedUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessResult {
    pub allowed: bool,
    pub failure: Option<AccessFailure>,
}

impl AccessResult {
    #[must_use]
    pub fn allowed() -> Self {
        Self {
            allowed: true,
            failure: None,
        }
    }

    #[must_use]
    pub fn denied(kind: AccessFailure) -> Self {
        Self {
            allowed: false,
            failure: Some(kind),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AccessContext<'a> {
    pub current_package: Option<&'a str>,
    pub current_type: Option<&'a str>,
    pub current_namespace: Option<&'a str>,
    pub receiver_type: Option<&'a str>,
    pub is_instance: bool,
}

impl<'a> AccessContext<'a> {
    #[must_use]
    pub fn for_type(
        current_package: Option<&'a str>,
        current_type: Option<&'a str>,
        current_namespace: Option<&'a str>,
    ) -> Self {
        Self {
            current_package,
            current_type,
            current_namespace,
            receiver_type: None,
            is_instance: false,
        }
    }

    #[must_use]
    pub fn with_receiver(mut self, receiver_type: Option<&'a str>) -> Self {
        self.receiver_type = receiver_type;
        self.is_instance = true;
        self
    }
}

fn namespaces_match(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(left), Some(right)) => {
            let left = left.replace("::", ".");
            let right = right.replace("::", ".");
            left == right
        }
        _ => false,
    }
}

/// Evaluate whether a symbol with the given visibility/owner is accessible from the provided
/// context. `is_derived_from` should return true when `candidate` is the same as or derives
/// from `base`.
#[must_use]
pub fn check_access<'a>(
    visibility: Visibility,
    owner: &'a str,
    owner_package: Option<&'a str>,
    owner_namespace: Option<&'a str>,
    ctx: &AccessContext<'a>,
    same_type: impl Fn(&str, &str) -> bool,
    is_derived_from: impl Fn(&str, &str) -> bool,
) -> AccessResult {
    let same_package = match (owner_package, ctx.current_package) {
        (Some(owner), Some(current)) => owner == current,
        (Some(_), None) => false,
        (None, Some(_)) | (None, None) => namespaces_match(owner_namespace, ctx.current_namespace),
    };
    let protected_allowed = ctx
        .current_type
        .map(|ty| is_derived_from(ty, owner))
        .unwrap_or(false);
    let receiver_allowed = if ctx.is_instance {
        match (ctx.receiver_type, ctx.current_type) {
            (Some(receiver), Some(current)) => is_derived_from(receiver, current),
            (None, _) => true,
            _ => false,
        }
    } else {
        true
    };

    match visibility {
        Visibility::Public => AccessResult::allowed(),
        Visibility::Private => ctx
            .current_type
            .filter(|ty| same_type(ty, owner))
            .map(|_| AccessResult::allowed())
            .unwrap_or_else(|| AccessResult::denied(AccessFailure::Private)),
        Visibility::Internal => {
            if ctx.current_type.is_some_and(|ty| same_type(ty, owner)) || same_package {
                AccessResult::allowed()
            } else {
                AccessResult::denied(AccessFailure::InternalPackage)
            }
        }
        Visibility::Protected => {
            if !protected_allowed {
                return AccessResult::denied(AccessFailure::ProtectedInheritance);
            }
            if !receiver_allowed {
                return AccessResult::denied(AccessFailure::ProtectedReceiver);
            }
            AccessResult::allowed()
        }
        Visibility::ProtectedInternal => {
            if same_package {
                return AccessResult::allowed();
            }
            if !protected_allowed {
                return AccessResult::denied(AccessFailure::ProtectedInternalUnavailable);
            }
            if !receiver_allowed {
                return AccessResult::denied(AccessFailure::ProtectedReceiver);
            }
            AccessResult::allowed()
        }
        Visibility::PrivateProtected => {
            if !same_package {
                return AccessResult::denied(AccessFailure::PrivateProtectedUnavailable);
            }
            if !protected_allowed {
                return AccessResult::denied(AccessFailure::ProtectedInheritance);
            }
            if !receiver_allowed {
                return AccessResult::denied(AccessFailure::ProtectedReceiver);
            }
            AccessResult::allowed()
        }
    }
}
