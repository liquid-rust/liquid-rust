#![feature(rustc_private)]
#![feature(const_panic)]

extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_mir;
extern crate rustc_serialize;
extern crate rustc_span;

pub mod borrowck;
mod callbacks;
mod collector;
mod lower;
mod resolution;

use callbacks::LiquidCallbacks;

use rustc_driver::{catch_with_exit_code, RunCompiler};

/// Run Liquid Rust and return the exit status code.
pub fn run_compiler(args: Vec<String>) -> i32 {
    let mut callbacks = LiquidCallbacks::default();

    catch_with_exit_code(move || RunCompiler::new(&args, &mut callbacks).run())
}
