use std::{env::args, process::exit};

/// Get the path to the sysroot of the current rustup toolchain. Return `None` if the rustup
/// environment variables are not set.
fn sysroot() -> Option<String> {
    let home = option_env!("RUSTUP_HOME")?;
    let toolchain = option_env!("RUSTUP_TOOLCHAIN")?;
    Some(format!("{}/toolchains/{}", home, toolchain))
}

fn main() {
    // Get the arguments from the command line.
    let mut args: Vec<String> = args().collect();
    // Add the sysroot path to the arguments.
    args.push("--sysroot".into());
    args.push(sysroot().expect("Liquid Rust requires rustup to be built."));
    // Record nll-facts for this compilation.
    args.push("-Znll-facts".into());
    // Add release mode to the arguments.
    args.push("-O".into());
    // Run the rust compiler with the arguments.
    let exit_code = liquid_rust_driver::run_compiler(args);
    // Exit with the exit code returned by the compiler.
    exit(exit_code)
}
