use crate::{
    bblock_env::{BBlockEnv, BBlockTy},
    check::Check,
    glob_env::GlobEnv,
    result::TyResult,
    subtype::Subtype,
    synth::Synth,
};

use liquid_rust_fixpoint::Emitter;
use liquid_rust_mir::{ty::Ty, BBlock, BBlockId};

impl<'ty, 'env> Check<'ty, 'env> for BBlock {
    type Ty = &'ty BBlockTy;
    type Env = (&'env GlobEnv, &'env BBlockEnv, &'env Ty);

    fn check(&self, ty: Self::Ty, emitter: &mut Emitter, env: Self::Env) -> TyResult {
        let mut ty = ty.clone();

        for statement in self.statements() {
            ty.input = statement.synth(&ty.input)?;
        }

        self.terminator().check(&ty, emitter, env)
    }
}

impl<'ty, 'env> Check<'ty, 'env> for BBlockId {
    type Ty = &'ty BBlockTy;
    type Env = (&'env GlobEnv, &'env BBlockEnv, &'env Ty);

    fn check(&self, ty: Self::Ty, emitter: &mut Emitter, (_, bb_env, _): Self::Env) -> TyResult {
        let bb_ty = bb_env.get_ty(*self);

        bb_ty.subtype(ty, emitter, ())
    }
}
