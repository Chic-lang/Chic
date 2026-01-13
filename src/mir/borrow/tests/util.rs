use crate::mir::borrow::{BorrowCheckResult, borrow_check_function_with_layouts};
use crate::mir::data::{
    Abi, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, ParamMode, Ty,
};
use crate::mir::layout::TypeLayoutTable;
use crate::mir::state::AsyncStateMachine;

/// Harness that captures shared MIR borrow checker fixtures and layouts for reuse across tests.
pub struct BorrowTestHarness {
    name: String,
    signature: FnSig,
    kind: FunctionKind,
    is_async: bool,
    base_body: MirBody,
    layouts: TypeLayoutTable,
}

/// Borrow checker test case produced from a [`BorrowTestHarness`].
pub struct BorrowTestCase<'h> {
    harness: &'h BorrowTestHarness,
    body: MirBody,
    is_async: bool,
}

impl BorrowTestHarness {
    /// Create a harness with a default Chic function signature returning `Unit`.
    #[must_use]
    pub fn new(name: &str) -> Self {
        let signature = FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        };
        let mut base_body = MirBody::new(0, None);
        base_body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            signature.ret.clone(),
            false,
            None,
            LocalKind::Return,
        ));

        Self {
            name: name.into(),
            signature,
            kind: FunctionKind::Function,
            is_async: false,
            base_body,
            layouts: TypeLayoutTable::default(),
        }
    }

    /// Override the function's return type and align the base `_ret` slot to match.
    #[must_use]
    pub fn with_return_type(mut self, ret: Ty) -> Self {
        self.signature.ret = ret.clone();
        if let Some(ret_local) = self.base_body.locals.first_mut() {
            ret_local.ty = ret;
        }
        self
    }

    /// Spawn a new borrow-checking test case that clones the current base body/layout state.
    #[must_use]
    pub fn case(&self) -> BorrowTestCase<'_> {
        BorrowTestCase {
            harness: self,
            body: self.base_body.clone(),
            is_async: self.is_async,
        }
    }

    /// Configure the harness for async functions by default.
    #[must_use]
    pub fn mark_async(mut self) -> Self {
        self.is_async = true;
        self
    }

    /// Configure the harness to mirror constructor semantics.
    #[must_use]
    pub fn mark_constructor(mut self) -> Self {
        self.kind = FunctionKind::Constructor;
        self
    }

    /// Expose a mutable reference to the layout table so callers can register custom layouts.
    pub fn layouts_mut(&mut self) -> &mut TypeLayoutTable {
        &mut self.layouts
    }

    fn build_function(&self, body: MirBody, is_async: bool) -> MirFunction {
        MirFunction {
            name: self.name.clone(),
            kind: self.kind,
            signature: self.signature.clone(),
            body,
            is_async,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        }
    }
}

impl BorrowTestCase<'_> {
    /// Mutably access the case MIR body for fixture construction.
    pub fn body_mut(&mut self) -> &mut MirBody {
        &mut self.body
    }

    /// Add a local with the supplied metadata, returning its [`LocalId`].
    #[must_use]
    pub fn push_local(
        &mut self,
        name: Option<&str>,
        ty: Ty,
        mutable: bool,
        kind: LocalKind,
    ) -> LocalId {
        self.push_local_with_mode(name, ty, mutable, kind, None)
    }

    /// Add a local with an explicit `ParamMode`, returning its [`LocalId`].
    pub fn push_local_with_mode(
        &mut self,
        name: Option<&str>,
        ty: Ty,
        mutable: bool,
        kind: LocalKind,
        mode: Option<ParamMode>,
    ) -> LocalId {
        let mut decl = LocalDecl::new(name.map(str::to_owned), ty, mutable, None, kind);
        if let Some(mode) = mode {
            decl = decl.with_param_mode(mode);
        }
        self.body.locals.push(decl);
        LocalId(self.body.locals.len() - 1)
    }

    /// Obtain the return slot for convenience in assertions.
    #[must_use]
    pub fn return_slot(&self) -> LocalId {
        LocalId(0)
    }

    /// Attach an async state machine to the MIR body and mark the case async.
    #[must_use]
    pub fn with_async_machine(mut self, machine: AsyncStateMachine) -> Self {
        self.body.async_machine = Some(machine);
        self.is_async = true;
        self
    }

    /// Run the borrow checker for this case.
    pub fn run(self) -> BorrowCheckResult {
        let function = self.harness.build_function(self.body, self.is_async);
        borrow_check_function_with_layouts(&function, &self.harness.layouts)
    }
}

/// Extension helpers for asserting diagnostics in tests.
pub trait BorrowCheckResultExt {
    fn expect_message(&self, needle: &str);
}

impl BorrowCheckResultExt for BorrowCheckResult {
    fn expect_message(&self, needle: &str) {
        assert!(
            self.diagnostics
                .iter()
                .any(|diag| diag.message.contains(needle)),
            "expected diagnostic containing `{needle}`, got {:?}",
            self.diagnostics
        );
    }
}
