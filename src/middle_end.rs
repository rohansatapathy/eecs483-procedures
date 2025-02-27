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

impl Continuation {
    fn invoke(self, imm: Immediate) -> BlockBody {
        match self {
            Continuation::Return => {
                BlockBody::Terminator(Terminator::Return(imm))
            }
            Continuation::Block(dest, next) => BlockBody::Operation {
                dest,
                op: Operation::Immediate(imm),
                next: Box::new(next),
            },
        }
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
        if !prog.externs.is_empty() {
            panic!("middle end doesn't support externs yet")
        }
        let externs = vec![];

        let main_block_label = self.blocks.fresh("main_tail");
        let main_fun_block = FunBlock {
            name: prog.name,
            params: vec![prog.param.0.clone()],
            body: Branch {
                target: main_block_label.clone(),
                args: vec![Immediate::Var(prog.param.0)],
            },
        };
        let funs = vec![main_fun_block];

        let main_block_body =
            self.lower_expr_kont(prog.body, Continuation::Return, true);
        let main_basic_block = BasicBlock {
            label: main_block_label,
            params: vec![self.vars.fresh("x")],
            body: main_block_body,
        };
        let blocks = vec![main_basic_block];

        Program { externs, funs, blocks }
    }

    fn lower_expr_kont(
        &mut self, expr: BoundExpr, k: Continuation, in_tail_pos: bool,
    ) -> BlockBody {
        todo!("lower_expr_kont not implemented")
    }
}
