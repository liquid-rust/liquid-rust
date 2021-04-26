macro_rules! pass_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let home = option_env!("RUSTUP_HOME").unwrap();
            let toolchain = option_env!("RUSTUP_TOOLCHAIN").unwrap();
            let sysroot = format!("--sysroot={}/toolchains/{}", home, toolchain);

            let path = concat!("tests/pass/", stringify!($name), ".rs");

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

pass_test!(one);
