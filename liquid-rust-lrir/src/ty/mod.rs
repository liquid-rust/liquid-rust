pub mod context;

pub use context::TyCtxt;

use crate::mir::{Constant, Place};

use liquid_rust_common::new_index;

use hashconsing::HConsed;
use std::fmt;

/// A function type declaration
pub struct FnDecl {
    /// A mapping between ghost variables and their required types. From caller's perspective, ghost
    /// variables in this mapping are universally quantified and need to be instantiated at the
    /// call site with concrete ghost variables satisfying the type requirements.
    pub requires: Vec<(GhostVar, Ty)>,
    /// The ghost variables (bound in [requires](Self::requires)) corresponding to each
    /// input argument.
    pub inputs: Vec<GhostVar>,
    /// A mapping between ghost variables and their ensured types. From caller's perspective, ghost
    /// variables in this mapping are existentially quantified and can be _assumed_ at the call
    /// site after the function returns.
    pub ensures: Vec<(GhostVar, Ty)>,
    /// Updated ghost variables (bound in [ensures](Self::ensures)) for argument [references](TyKind::Ref)
    /// Arguments are referenced by specifying their index in [inputs](Self::inputs).
    pub outputs: Vec<(usize, GhostVar)>,
    /// Ghost variable (bound in [ensures](Self::ensures)) corresponding to the output of this
    /// function.
    pub output: GhostVar,
}

pub type Ty = HConsed<TyS>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TyS {
    kind: TyKind,
}

impl TyS {
    pub fn kind(&self) -> &TyKind {
        &self.kind
    }
}

impl fmt::Display for TyS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            TyKind::Ref(BorrowKind::Shared, r, gv) => write!(f, "&{} {}", r, gv),
            TyKind::Ref(BorrowKind::Mut, r, gv) => write!(f, "&{} mut {}", r, gv),
            TyKind::Tuple(tup) => {
                let tup = tup
                    .iter()
                    .map(|(f, ty)| format!("@{}: {}", f, ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({})", tup)
            }
            TyKind::Uninit(size) => write!(f, "uninit({})", size),
            TyKind::Refined(bty, Refine::Infer(k)) => write!(f, "{{ {} | {} }}", bty, k),
            TyKind::Refined(bty, Refine::Pred(pred)) => match pred.kind() {
                PredKind::Const(c) if c.is_true() => {
                    write!(f, "{}", bty)
                }
                _ => {
                    write!(f, "{{ {} | {} }}", bty, pred)
                }
            },
        }
    }
}

/// A refined type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    /// A refined base type: `{B | p}`
    Refined(BaseTy, Refine),
    /// A dependent tuple: `(x: int, y: {int | x > v})`.
    Tuple(Tuple),
    /// A borrowed reference.
    Ref(BorrowKind, Region, GhostVar),
    /// Unitialized memory of given size.
    Uninit(usize),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Tuple(Vec<(Field, Ty)>);

impl Tuple {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn map(&self, mut f: impl FnMut(usize, &Field, &Ty) -> (Field, Ty)) -> Tuple {
        Self(
            self.0
                .iter()
                .enumerate()
                .map(|(i, (fld, ty))| f(i, fld, ty))
                .collect(),
        )
    }

    pub fn map_ty_at(&self, n: usize, f: impl FnOnce(&Ty) -> Ty) -> Tuple {
        let mut v = self.0.clone();
        v[n].1 = f(&v[n].1);
        Tuple(v)
    }

    pub fn ty_at(&self, n: usize) -> &Ty {
        &self.0[n].1
    }

    pub fn types(&self) -> impl DoubleEndedIterator<Item = &Ty> + ExactSizeIterator {
        self.0.iter().map(|x| &x.1)
    }

    pub fn fields(&self) -> impl DoubleEndedIterator<Item = &Field> + ExactSizeIterator {
        self.0.iter().map(|x| &x.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Field, Ty)> {
        self.0.iter()
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Refine {
    /// A refinement that needs to be inferred.
    Infer(Kvar),
    /// A refinement predicate.
    Pred(Pred),
}

impl From<Pred> for Refine {
    fn from(v: Pred) -> Self {
        Self::Pred(v)
    }
}

pub type Pred = HConsed<PredS>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PredS {
    kind: PredKind,
}

impl fmt::Display for PredS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            PredKind::Path(path) => write!(f, "{}", path),
            PredKind::BinaryOp(bin_op, op1, op2) => write!(f, "({} {} {})", op1, bin_op, op2),
            PredKind::UnaryOp(un_op, op) => write!(f, "{}{}", un_op, op),
            PredKind::Const(c) => write!(f, "{}", c),
        }
    }
}

impl PredS {
    pub fn kind(&self) -> &PredKind {
        &self.kind
    }
}

/// A refinement predicate.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PredKind {
    /// A path to a value.
    Path(Path),
    /// A binary operation between predicates.
    BinaryOp(BinOp, Pred, Pred),
    /// A unary operation applied to a predicate.
    UnaryOp(UnOp, Pred),
    /// A constant value.
    Const(Constant),
}

/// A path to a value. Note that unlike [Place], a [Path] only contains field projections, i.e.,
/// it doesn't have dereferences.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub base: Var,
    pub projs: Vec<usize>,
}

impl Path {
    pub fn new(base: Var, projs: Vec<usize>) -> Self {
        Self { base, projs }
    }

    pub fn extend(&self, n: usize) -> Self {
        let mut projs = self.projs.clone();
        projs.push(n);
        Path::new(self.base, projs)
    }
}

impl<T: Into<Var>> From<T> for Path {
    fn from(base: T) -> Self {
        Path::new(base.into(), vec![])
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.base)?;
        for proj in &self.projs {
            write!(f, ".{}", proj)?;
        }
        Ok(())
    }
}

// TODO: investigate if is convenient to use DeBruijn instead of Nu and Field.
/// A variable that can be used inside a refinement predicate.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Var {
    /// The variable bound by a refined base type, i.e., the `v` in `{v: int | v > 0}`. The name nu
    /// is used because the greek letter nu is tipically used to range over this variables when
    /// formalized.
    Nu,
    /// A ghost variable.
    Ghost(GhostVar),
    /// A field name in a dependent tuple.
    Field(Field),
}

impl From<Field> for Var {
    fn from(v: Field) -> Self {
        Self::Field(v)
    }
}

impl From<GhostVar> for Var {
    fn from(v: GhostVar) -> Self {
        Self::Ghost(v)
    }
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Var::Nu => write!(f, "v"),
            Var::Ghost(l) => write!(f, "{}", l),
            Var::Field(fld) => write!(f, "{}", fld),
        }
    }
}

/// A primitive binary operator.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    /// The integer addition operator.
    Add,
    /// The integer substraction operator.
    Sub,
    /// The integer multiplication operator.
    Mul,
    /// The integer division operator.
    Div,
    /// The integer remainder operator.
    Rem,
    /// The equality operator for a particular base type.
    Eq(BaseTy),
    /// The "not equal to" operator for a particular base type.
    Neq(BaseTy),
    /// The "less than" integer operator.
    Lt,
    /// The "greater than" integer operator.
    Gt,
    /// The "less than or equal" integer operator.
    Lte,
    /// The "greater than or equal" integer operator.
    Gte,
    /// The boolean "and" operator.
    And,
    /// The boolean "or" operator.
    Or,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::And => "&&",
            Self::Or => "||",
            Self::Eq { .. } => "==",
            Self::Neq { .. } => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Lte => "<=",
            Self::Gte => ">=",
        };
        s.fmt(f)
    }
}

/// A primitive unary operator.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// The boolean negation operator.
    Not,
    /// The integer negation operator.
    Neg,
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Not => "!",
            Self::Neg => "-",
        };
        s.fmt(f)
    }
}

/// A base type than can refined.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseTy {
    Int,
    Bool,
}

impl fmt::Display for BaseTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseTy::Int => write!(f, "int"),
            BaseTy::Bool => write!(f, "bool"),
        }
    }
}

/// A region corresponds to an approximate set of possible provenances for a reference.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Region {
    /// A concrete (approximate) provenance set.
    Concrete(Vec<Place>),
    /// An abstract universally quantified region.
    Abstract(UniversalRegion),
    /// A region that needs to be inferred.
    Infer(RegionVid),
}

impl From<Place> for Region {
    fn from(place: Place) -> Self {
        Region::Concrete(vec![place])
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Region::Concrete(places) => {
                let places = places
                    .iter()
                    .map(|place| format!("{}", place))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{{ {} }}", places)
            }
            Region::Abstract(uregion) => write!(f, "{}", uregion),
            Region::Infer(rvid) => write!(f, "{}", rvid),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mut,
}

/// A k-variable correspond to a refinement predicate that needs to be inferred. They are called
/// k-variables because the greek letter kappa is typically used to range over these variables when
/// formalized.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Kvar {
    /// Variable id
    pub id: KVid,
    /// Variables that the predicate can depend on.
    pub vars: Vec<Var>,
}

impl fmt::Display for Kvar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vars = self
            .vars
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{}[{}]", self.id, vars)
    }
}

new_index! {
    /// A **K** **v**ariable **ID** that needs to be inferred.
    KVid
}

impl fmt::Display for KVid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$k{}", self.as_usize())
    }
}

new_index! {
    /// A **Region** **v**ariable **ID** that needs to be inferred.
    RegionVid
}

impl fmt::Display for RegionVid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$r{}", self.as_usize())
    }
}

new_index! {
    /// A field name in a dependent tuple.
    Field
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.as_usize())
    }
}

new_index! {
    /// A universally quantified region, i.e., the `'a` in  `fn foo<'a>(n: &'a int)`
    UniversalRegion
}

impl fmt::Display for UniversalRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}", self.as_usize())
    }
}

new_index! {
    /// A (logical) ghost variable. During typechecking we don't associate types to places directly,
    /// but create ghost variables corresponding to them after each update.
    /// Refinements do not depend directly on [Local]s but on ghost variables. Because we create new
    /// ghost variables without removing old ones, refinements can also depend on old values.
    /// Very much like SSA.
    GhostVar
}

impl fmt::Display for GhostVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "g{}", self.as_usize())
    }
}