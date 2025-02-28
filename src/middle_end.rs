//! The middle "end" of our compiler is the part that transforms our well-formed
//! source-language abstract syntax tree (AST) into the intermediate representation

use crate::ast::{self, *};
use crate::ssa::{self, *};
use crate::{frontend::Resolver, identifiers::*};
use im::HashMap;
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

/// Env is used to temporarily map FunNames to their corresponding "_tail"
/// BlockNames.
type Env = HashMap<FunName, BlockName>;

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
        let main_fun_block_arg = self.vars.fresh("x");
        let main_fun_block = FunBlock {
            name: prog.name.clone(),
            params: vec![main_fun_block_arg.clone()],
            body: Branch {
                target: main_block_label.clone(),
                args: vec![Immediate::Var(main_fun_block_arg)],
            },
        };
        let funs = vec![main_fun_block];

        let mut env = Env::new();
        env.insert(prog.name, main_block_label.clone());

        let main_block_body =
            self.lower_expr_kont(prog.body, Continuation::Return, &mut env);
        let main_basic_block = BasicBlock {
            label: main_block_label,
            params: vec![prog.param.0],
            body: main_block_body,
        };
        let blocks = vec![main_basic_block];

        Program { externs, funs, blocks }
    }

    fn lower_expr_kont(
        &mut self, expr: BoundExpr, k: Continuation, env: &mut Env,
    ) -> BlockBody {
        match expr {
            Expr::Num(n, _) => k.invoke(Immediate::Const(n)),
            Expr::Bool(b, _) => {
                k.invoke(Immediate::Const(if b { 1 } else { 0 }))
            }
            Expr::Var(var, _) => k.invoke(Immediate::Var(var)),
            Expr::Prim { prim, args, loc: _ } => {
                // For each arg, create a tmp variable to store the result in
                // and the corresponding Immediate
                let (args_var, args_imm): (Vec<_>, Vec<_>) = args
                    .iter()
                    .enumerate()
                    .map(|(i, _arg)| {
                        let var =
                            self.vars.fresh(format!("{}_{}", &prim, i));
                        (var.clone(), Immediate::Var(var))
                    })
                    .unzip();

                // Get the result variable and the continuation's BlockBody
                let (dest, next) = match k {
                    Continuation::Block(res, block) => (res, block),
                    Continuation::Return => {
                        let res = self.vars.fresh(format!("{}_res", &prim));
                        (res.clone(), k.invoke(Immediate::Var(res)))
                    }
                };

                // Helper functions for different categories of Prim. Each
                // helper handles that type of function and returns the
                // BlockBody corresponding to that operation.

                // prim1 handles Add1 and Sub1 operations
                let prim1 = |prim: ssa::Prim2, imm: Immediate, next| {
                    let dest = dest.clone();
                    let op =
                        Operation::Prim2(prim, args_imm[0].clone(), imm);
                    BlockBody::Operation { dest, op, next: Box::new(next) }
                };

                // prim2 handles all arithmetic and comparison Prim operations
                let prim2 = |prim: ssa::Prim2, next| {
                    let dest = dest.clone();
                    let op = Operation::Prim2(
                        prim,
                        args_imm[0].clone(),
                        args_imm[1].clone(),
                    );
                    BlockBody::Operation { dest, op, next: Box::new(next) }
                };

                // prim2_logical handles all Prims that require 2 boolean
                // arguments (i.e. Prim::And and Prim::Or)
                let mut prim2_logical = |prim: ssa::Prim2, next| {
                    let dest = dest.clone();

                    // Create the VarNames and corresponding Immediates
                    // for the type-converted versions of the arguments
                    let (type_checked_args, type_checked_imms): (
                        Vec<_>,
                        Vec<_>,
                    ) = args
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            let var = self.vars.fresh("itob_res");
                            (var.clone(), Immediate::Var(var))
                        })
                        .collect();

                    BlockBody::Operation {
                        dest: type_checked_args[0].clone(),
                        op: Operation::Prim1(
                            Prim1::IntToBool,
                            args_imm[0].clone(),
                        ),
                        next: Box::new(BlockBody::Operation {
                            dest: type_checked_args[1].clone(),
                            op: Operation::Prim1(
                                Prim1::IntToBool,
                                args_imm[1].clone(),
                            ),
                            next: Box::new(BlockBody::Operation {
                                dest,
                                op: Operation::Prim2(
                                    prim,
                                    type_checked_imms[0].clone(),
                                    type_checked_imms[1].clone(),
                                ),
                                next: Box::new(next),
                            }),
                        }),
                    }
                };

                // Create the BlockBody for the final operation
                let block = match prim {
                    Prim::Add1 => {
                        prim1(Prim2::Add, Immediate::Const(1), next)
                    }
                    Prim::Sub1 => {
                        prim1(Prim2::Sub, Immediate::Const(1), next)
                    }
                    Prim::Add => prim2(Prim2::Add, next),
                    Prim::Sub => prim2(Prim2::Sub, next),
                    Prim::Mul => prim2(Prim2::Mul, next),
                    Prim::Not => {
                        let tmp = self.vars.fresh("itob_res");
                        BlockBody::Operation {
                            dest: tmp.clone(),
                            op: Operation::Prim1(
                                Prim1::IntToBool,
                                args_imm[0].clone(),
                            ),
                            next: Box::new(BlockBody::Operation {
                                dest,
                                op: Operation::Prim2(
                                    Prim2::BitXor,
                                    Immediate::Var(tmp),
                                    Immediate::Const(1),
                                ),
                                next: Box::new(next),
                            }),
                        }
                    }
                    Prim::And => prim2_logical(Prim2::BitAnd, next),
                    Prim::Or => prim2_logical(Prim2::BitOr, next),
                    Prim::Lt => prim2(Prim2::Lt, next),
                    Prim::Le => prim2(Prim2::Le, next),
                    Prim::Gt => prim2(Prim2::Gt, next),
                    Prim::Ge => prim2(Prim2::Ge, next),
                    Prim::Eq => prim2(Prim2::Eq, next),
                    Prim::Neq => prim2(Prim2::Neq, next),
                };

                // Use fold() to build up the surrounding expression
                // evaluations over the current block.
                args.into_iter().zip(args_var).rev().fold(
                    block,
                    |block, (arg, var)| {
                        self.lower_expr_kont(
                            arg,
                            Continuation::Block(var, block),
                            env,
                        )
                    },
                )
            }
            Expr::Let { bindings, body, loc } => {
                let block = self.lower_expr_kont(*body, k, env);
                bindings.into_iter().rev().fold(block, |block, binding| {
                    self.lower_expr_kont(
                        binding.expr,
                        Continuation::Block(binding.var.0, block),
                        env,
                    )
                })
            }
            Expr::If { cond, thn, els, loc } => {
                let cond_var = self.vars.fresh("cond");
                let thn_label = self.blocks.fresh("thn");
                let els_label = self.blocks.fresh("els");
                let cond_branch = Box::new(self.lower_expr_kont(
                    *cond,
                    Continuation::Block(
                        cond_var.clone(),
                        BlockBody::Terminator(
                            Terminator::ConditionalBranch {
                                cond: Immediate::Var(cond_var),
                                thn: thn_label.clone(),
                                els: els_label.clone(),
                            },
                        ),
                    ),
                    env,
                ));
                // Here is the exponential implementation
                // let mut branch = |label, body: BoundExpr| BasicBlock {
                //     label,
                //     params: Vec::new(),
                //     body: self.lower_expr_kont(body, k.clone()),
                // };
                // BlockBody::SubBlocks {
                //     blocks: vec![branch(thn_label, *thn), branch(els_label, *els)],
                //     next: cond_branch,
                // }

                // Here is the correct implementation, also optimizing to not create a join point if in tail position
                match k {
                    Continuation::Return => {
                        let mut branch =
                            |label, body: BoundExpr| BasicBlock {
                                label,
                                params: Vec::new(),
                                body: self.lower_expr_kont(
                                    body,
                                    Continuation::Return,
                                    env,
                                ),
                            };

                        BlockBody::SubBlocks {
                            blocks: vec![
                                branch(thn_label, *thn),
                                branch(els_label, *els),
                            ],
                            next: cond_branch,
                        }
                    }
                    // if we have a non-trivial continuation, we create a join point
                    Continuation::Block(dest, body) => {
                        // fresh variables for return positions in kontinuations
                        let thn_var = self.vars.fresh("thn_res");
                        let els_var = self.vars.fresh("els_res");
                        let join_label = self.blocks.fresh("jn");

                        let mut branch =
                            |label, expr: BoundExpr, var: VarName| {
                                BasicBlock {
                                    label,
                                    params: Vec::new(),
                                    body: self.lower_expr_kont(
                                        expr,
                                        Continuation::Block(
                                            var.clone(),
                                            BlockBody::Terminator(
                                                Terminator::Branch(Branch {
                                                    target: join_label
                                                        .clone(),
                                                    args: vec![
                                                        Immediate::Var(var),
                                                    ],
                                                }),
                                            ),
                                        ),
                                        env,
                                    ),
                                }
                            };

                        BlockBody::SubBlocks {
                            blocks: vec![
                                branch(thn_label, *thn, thn_var),
                                branch(els_label, *els, els_var),
                                BasicBlock {
                                    label: join_label,
                                    params: vec![dest],
                                    body,
                                },
                            ],
                            next: cond_branch,
                        }
                    }
                }
            }
            Expr::FunDefs { decls, body, loc } => {
                let next = Box::new(self.lower_expr_kont(*body, k, env));
                BlockBody::SubBlocks {
                    blocks: decls
                        .into_iter()
                        .filter_map(
                            |FunDecl { name, params, body, loc: _ }| {
                                // tail recursive functions are built as sub-blocks
                                Some(BasicBlock {
                                    label: match env.get(&name) {
                                        Some(lbl) => lbl.clone(),
                                        None => {
                                            let tail_lbl =
                                                self.blocks.fresh(format!(
                                                    "{}_tail",
                                                    name.hint()
                                                ));
                                            env.insert(
                                                name.clone(),
                                                tail_lbl.clone(),
                                            );
                                            tail_lbl
                                        }
                                    },

                                    params: params
                                        .into_iter()
                                        .map(|(p, _)| p)
                                        .collect(),
                                    body: self.lower_expr_kont(
                                        body,
                                        Continuation::Return,
                                        env,
                                    ),
                                })
                            },
                        )
                        .collect(),
                    next,
                }
            }
            Expr::Call { fun, args, loc } => {
                match k {
                    Continuation::Return => {}
                    Continuation::Block(..) => {
                        panic!(
                            "middle end doesn't support non-tail calls yet"
                        )
                    }
                }

                // prepare the arguments
                let (args_var, args_imm): (Vec<_>, _) = args
                    .iter()
                    .enumerate()
                    .map(|(i, _arg)| {
                        // the arguments are named after the function name and the argument index
                        let var =
                            self.vars.fresh(format!("{}_{}", fun.hint(), i));
                        (var.clone(), Immediate::Var(var))
                    })
                    .unzip();
                // tail calls are compiled to a branch
                let block =
                    BlockBody::Terminator(Terminator::Branch(Branch {
                        target: match env.get(&fun) {
                            Some(lbl) => lbl.clone(),
                            None => {
                                let tail_lbl = self
                                    .blocks
                                    .fresh(format!("{}_tail", fun.hint()));
                                env.insert(fun.clone(), tail_lbl.clone());
                                tail_lbl
                            }
                        },

                        args: args_imm,
                    }));

                // compile in reverse order, as above
                args.into_iter().zip(args_var).rev().fold(
                    block,
                    |block, (arg, var)| {
                        self.lower_expr_kont(
                            arg,
                            Continuation::Block(var, block),
                            env,
                        )
                    },
                )
            }
        }
    }
}
