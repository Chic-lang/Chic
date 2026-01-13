#![allow(dead_code, unused_imports)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "Tokio-based test executor has been removed; build with chic_native_runtime so the native executor runs tests."
);

mod native {
    use crate::driver::{TestCaseResult, TestOptions, TestStatus};
    use crate::mir::MirModule;

    #[derive(Debug)]
    pub struct TestExecutionError {
        pub message: String,
    }

    impl TestExecutionError {
        fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
            }
        }
    }

    pub struct TestExecutor<'a> {
        module: &'a MirModule,
    }

    impl<'a> TestExecutor<'a> {
        #[must_use]
        pub fn new(module: &'a MirModule) -> Self {
            Self { module }
        }

        #[must_use]
        pub fn with_defaults(module: &'a MirModule) -> Self {
            Self::new(module)
        }

        pub fn run_all(&mut self) -> Vec<TestCaseResult> {
            let status = unsafe { chic_rt_test_executor_run_all() };
            if status == 0 {
                // Native executor handled execution internally and reported success.
                return Vec::new();
            }
            vec![TestCaseResult {
                id: "native".to_string(),
                name: "native_test_executor".to_string(),
                qualified_name: "native_test_executor".to_string(),
                namespace: None,
                categories: Vec::new(),
                is_async: false,
                status: TestStatus::Failed,
                message: Some(format!(
                    "native test executor returned error status {status}"
                )),
                wasm_trace: None,
                duration: None,
            }]
        }
    }

    pub fn execute_main(_module: &MirModule, _entry: &str) -> Result<i32, TestExecutionError> {
        Err(TestExecutionError::new(
            "execute_main is not available under chic_native_runtime; use native test executor",
        ))
    }

    pub fn execute_tests(
        module: &MirModule,
        options: &TestOptions,
    ) -> (Vec<TestCaseResult>, usize) {
        let mut exec = TestExecutor::new(module);
        let results = exec.run_all();
        let _ = options;
        (results, 0)
    }

    unsafe extern "C" {
        fn chic_rt_test_executor_run_all() -> i32;
    }
}

pub use native::{TestExecutionError, TestExecutor, execute_main, execute_tests};
