macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/testdate/",
            $name
        ))
    };
}

pub(crate) use fixture;
