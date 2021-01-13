use crate::local::Local;

use liquid_rust_ty::{Literal, Predicate, Variable};

use std::fmt;

#[derive(Clone)]
pub enum Operand {
    Local(Local),
    Literal(Literal),
}

impl From<Operand> for Predicate {
    fn from(operand: Operand) -> Self {
        match operand {
            Operand::Local(local) => Predicate::Var(Variable::Local(local.into())),
            Operand::Literal(literal) => Predicate::Lit(literal),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(local) => local.fmt(f),
            Self::Literal(literal) => literal.fmt(f),
        }
    }
}
