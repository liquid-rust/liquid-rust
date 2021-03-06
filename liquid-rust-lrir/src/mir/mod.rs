mod basic_block;
mod body;
mod constant;
mod local;
mod operand;
mod place;
mod rvalue;
mod statement;
mod terminator;

pub use basic_block::{BasicBlock, BasicBlockData};
pub use body::Body;
pub use constant::Constant;
pub use local::{Local, LocalDecl};
pub use operand::Operand;
pub use place::{Place, PlaceElem, PlaceRef};
pub use rustc_middle::mir::{BinOp, UnOp};
pub use rvalue::Rvalue;
pub use statement::{Statement, StatementKind};
pub use terminator::{SwitchTargets, Terminator, TerminatorKind};

pub use rustc_span::Span;
