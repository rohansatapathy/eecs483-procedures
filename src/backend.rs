//! The backend of our compiler translates our intermediate
//! representation into assembly code, mapping intermediate
//! representation variables into concrete memory locations.

use crate::asm::*;
use crate::identifiers::*;
use crate::middle_end::Lowerer;
use crate::ssa::*;

use std::collections::HashMap;
#[derive(Clone)]
struct Env<'a> {
    next: i32,
    arena: HashMap<&'a VarName, i32>,
    blocks: HashMap<&'a BlockName, i32>,
}

impl<'a> Env<'a> {
    fn new() -> Self {
        Env { next: 1, arena: HashMap::new(), blocks: HashMap::new() }
    }
    fn allocate(&mut self, x: &'a VarName) -> i32 {
        let loc = self.next;
        self.arena.insert(x, loc);
        self.next += 1;
        loc
    }
    fn lookup(&self, x: &'a VarName) -> i32 {
        self.arena.get(x).copied().expect("variable not allocated")
    }
}

pub struct Emitter {
    // the output buffer for the sequence of instructions we are generating
    instrs: Vec<Instr>,
}

impl From<Lowerer> for Emitter {
    fn from(Lowerer { .. }: Lowerer) -> Self {
        Emitter { instrs: Vec::new() }
    }
}

impl Emitter {
    pub fn to_asm(self) -> Vec<Instr> {
        self.instrs
    }

    fn emit(&mut self, instr: Instr) {
        self.instrs.push(instr);
    }

    pub fn emit_prog(&mut self, prog: &Program) {
        self.emit(Instr::Section(".data".to_string()));
        self.emit(Instr::Section(".text".to_string()));
        self.emit(Instr::Global("entry".to_string()));

        let mut env = Env::new();

        for ext in &prog.externs {
            self.emit_extern(ext, &mut env);
        }

        // First, register all blocks as having the same base offset of 1.
        // We need to do this all at once so that if any of the code inside
        // these blocks needs to jump to one of these blocks, they know where
        // to store the arguments.
        for block in &prog.blocks {
            env.blocks.insert(&block.label, env.next);
        }

        // Then, emit all the basic block with a cloned environment. (Why
        // cloned?)
        for block in &prog.blocks {
            self.emit_basic_block(block, &mut env.clone());
        }

        for fun in &prog.funs {
            self.emit_fun_block(fun, &mut env);
        }
    }

    fn emit_extern<'a>(&mut self, ext: &'a Extern, env: &mut Env<'a>) {
        unimplemented!("backend doesn't support externs yet")
    }

    fn emit_fun_block<'a>(
        &mut self, fun_block: &'a FunBlock, env: &mut Env<'a>,
    ) {
        // First, emit the label for the block.
        self.emit(Instr::Label(fun_block.name.to_string()));

        // Assume that the arguments are passed according to the SYSVAMD64
        // calling convention. For now, there should only be one argument
        // since the only FunBlock is main.
        let offset = 0;
        let base =
            env.blocks.get(&fun_block.body.target).unwrap_or_else(|| {
                panic!(
                    "no offset found for block '{}'",
                    &fun_block.body.target
                )
            });
        self.emit(store_mem(base + offset, Reg::Rdi));

        // Emit the jmp to the branch
        self.emit(Instr::Jmp(fun_block.body.target.to_string()));
    }

    fn emit_basic_block<'a>(
        &mut self, block: &'a BasicBlock, env: &mut Env<'a>,
    ) {
        self.emit(Instr::Label(block.label.to_string()));
        for param in &block.params {
            env.allocate(param);
        }
        self.emit_block_body(&block.body, env);
    }

    fn emit_block_body<'a>(&mut self, b: &'a BlockBody, env: &mut Env<'a>) {
        match b {
            BlockBody::Terminator(terminator) => {
                self.emit_terminator(terminator, env);
            }
            BlockBody::Operation { dest, op, next } => {
                self.emit_operation(dest, op, env);
                self.emit_block_body(next, env);
            }
            BlockBody::SubBlocks { blocks, next } => {
                // register all the block alignments first
                for BasicBlock { label, .. } in blocks {
                    env.blocks.insert(label, env.next);
                }
                // then emit the body with a cloned environment
                self.emit_block_body(next, &mut env.clone());
                // and finally, emit the sub-blocks, each with a cloned environment
                for BasicBlock { label, params, body } in blocks {
                    let mut env = env.clone();
                    self.emit(Instr::Label(label.to_string()));
                    for param in params {
                        env.allocate(param);
                    }
                    self.emit_block_body(body, &mut env);
                }
            }
        }
    }

    fn emit_terminator<'a>(&mut self, t: &'a Terminator, env: &Env<'a>) {
        match t {
            Terminator::Return(imm) => {
                self.emit_imm_reg(imm, Reg::Rax, env);
                self.emit(Instr::Ret);
            }
            Terminator::Branch(branch) => {
                self.emit_branch(branch, env);
            }
            Terminator::ConditionalBranch { cond, thn, els } => {
                self.emit_imm_reg(cond, Reg::Rax, env);
                self.emit(Instr::Cmp(BinArgs::ToReg(
                    Reg::Rax,
                    Arg32::Signed(0),
                )));
                self.emit(Instr::JCC(ConditionCode::NE, thn.to_string()));
                self.emit(Instr::Jmp(els.to_string()));
            }
        }
    }

    fn emit_branch<'a>(
        &mut self, Branch { target, args }: &'a Branch, env: &Env<'a>,
    ) {
        // lookup the base offset for the target's arguments
        let base = env.blocks.get(target).unwrap_or_else(|| {
            panic!("no offset found for block '{}'", target)
        });

        // store arguments in consecutive offsets from the target's base
        for (i, arg) in args.iter().enumerate() {
            // using Rax as a temp register
            self.emit_imm_reg(arg, Reg::Rax, env);
            self.emit(store_mem(base + i as i32, Reg::Rax));
        }
        // finally, jump to the target
        self.emit(Instr::Jmp(target.to_string()));
    }

    fn emit_operation<'a>(
        &mut self, dest: &'a VarName, op: &Operation, env: &mut Env<'a>,
    ) {
        // First generate code that places the result in rax, using
        // r10 as a scratch register
        match op {
            Operation::Immediate(imm) => {
                self.emit_imm_reg(imm, Reg::Rax, env);
            }
            Operation::Prim1(op, imm) => {
                self.emit_imm_reg(imm, Reg::Rax, env);
                match op {
                    Prim1::BitNot => {
                        self.emit(Instr::Mov(MovArgs::ToReg(
                            Reg::R10,
                            Arg64::Signed(-1),
                        )));
                        self.emit(Instr::Xor(BinArgs::ToReg(
                            Reg::Rax,
                            Arg32::Reg(Reg::R10),
                        )));
                    }
                    Prim1::IntToBool => {
                        // if reg is not zero, make it 1, otherwise make it 0
                        self.emit(Instr::Cmp(BinArgs::ToReg(
                            Reg::Rax,
                            Arg32::Signed(0),
                        )));
                        self.emit(Instr::Mov(MovArgs::ToReg(
                            Reg::Rax,
                            Arg64::Signed(0),
                        )));
                        self.emit(Instr::SetCC(ConditionCode::NE, Reg8::Al));
                    }
                }
            }
            Operation::Prim2(op, imm1, imm2) => {
                self.emit_imm_reg(imm1, Reg::Rax, env);
                self.emit_imm_reg(imm2, Reg::R10, env);
                let ba = BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R10));
                match op {
                    Prim2::Add => self.emit(Instr::Add(ba)),
                    Prim2::Sub => self.emit(Instr::Sub(ba)),
                    Prim2::Mul => self.emit(Instr::IMul(ba)),
                    Prim2::BitAnd => self.emit(Instr::And(ba)),
                    Prim2::BitOr => self.emit(Instr::Or(ba)),
                    Prim2::BitXor => self.emit(Instr::Xor(ba)),
                    Prim2::Lt => self.emit_cc(ConditionCode::L, ba),
                    Prim2::Gt => self.emit_cc(ConditionCode::G, ba),
                    Prim2::Le => self.emit_cc(ConditionCode::LE, ba),
                    Prim2::Ge => self.emit_cc(ConditionCode::GE, ba),
                    Prim2::Eq => self.emit_cc(ConditionCode::E, ba),
                    Prim2::Neq => self.emit_cc(ConditionCode::NE, ba),
                }
            }
            Operation::Call { fun, args } => {
                unimplemented!("backend doesn't support calls yet")
            }
        }
        // allocate the destination to be the next available offset from rsp
        let dst = env.allocate(dest);
        // write the return value back to the destination
        self.emit(store_mem(dst, Reg::Rax))
    }

    fn emit_cc(&mut self, cc: ConditionCode, ba: BinArgs) {
        // Here it is important to set rax to be 0, because setcc only sets al, the bottom byte of rax
        self.emit(Instr::Cmp(ba));
        self.emit(Instr::Mov(MovArgs::ToReg(Reg::Rax, Arg64::Signed(0))));
        self.emit(Instr::SetCC(cc, Reg8::Al))
    }

    fn emit_imm_reg<'a>(
        &mut self, imm: &'a Immediate, reg: Reg, env: &Env<'a>,
    ) {
        match imm {
            Immediate::Var(v) => {
                let src = env.lookup(v);
                self.emit(load_mem(reg, src))
            }
            Immediate::Const(i) => {
                self.emit(load_signed(reg, *i));
            }
        }
    }
}

/// Put the value of a signed constant into a register.
fn load_signed(reg: Reg, val: i64) -> Instr {
    Instr::Mov(MovArgs::ToReg(reg, Arg64::Signed(val)))
}

/// Put the value of a memory reference into a register.
fn load_mem(reg: Reg, src: i32) -> Instr {
    Instr::Mov(MovArgs::ToReg(
        reg,
        Arg64::Mem(MemRef { reg: Reg::Rsp, offset: -8 * src }),
    ))
}

/// Flush the value of a register into a memory reference.
fn store_mem(dst: i32, reg: Reg) -> Instr {
    Instr::Mov(MovArgs::ToMem(
        MemRef { reg: Reg::Rsp, offset: -8 * dst },
        Reg32::Reg(reg),
    ))
}
