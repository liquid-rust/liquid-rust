macro_rules! fail_test {
    ($name:ident) => {
        #[test]
        #[should_panic]
        fn $name() {
            let home = option_env!("RUSTUP_HOME").unwrap();
            let toolchain = option_env!("RUSTUP_TOOLCHAIN").unwrap();
            let sysroot = format!("--sysroot={}/toolchains/{}", home, toolchain);

            let path = concat!("tests/fail/", stringify!($name), ".rs");
            liquid_rust_driver::run_compiler(vec![
                "whatever".into(),
                "-Znll-facts".into(),
                path.into(),
                sysroot.into(),
                "--crate-type=lib".into(),
            ]);
        }
    };
}

fail_test!(one);
