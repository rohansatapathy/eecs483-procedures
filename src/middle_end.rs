//! The middle "end" of our compiler is the part that transforms our well-formed
//! source-language abstract syntax tree (AST) into the intermediate representation

use crate::ast::{self, *};
use crate::ssa::{self, *};
use crate::{frontend::Resolver, identifiers::*};
use std::collections::HashSet;

pub struct Lowerer {
    pub vars: IdGen<VarName>,
    pub funs: IdGen<FunName>,
    pub blocks: IdGen<BlockName>,
}

/// Indicates whether the expression being compiled is in a tail position.
#[derive(Clone, Debug)]
enum Continuation {
    Return,
    Block(VarName, BlockBody),
}

impl From<Resolver> for Lowerer {
    fn from(resolver: Resolver) -> Self {
        let Resolver { vars, funs, .. } = resolver;
        Lowerer { vars, funs, blocks: IdGen::new() }
    }
}

/// OPTIONAL:
/// Determine which functions should be lambda lifted.
/// If you choose not to implement this, then lift *all* functions
fn should_lift(prog: &BoundProg) -> HashSet<FunName> {
    todo!("should_lift not implemented")
}

impl Lowerer {
    pub fn lower_prog(&mut self, prog: BoundProg) -> Program {
        todo!("lower_prog not implemented")
    }
}
