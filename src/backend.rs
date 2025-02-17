//! The backend of our compiler translates our intermediate
//! representation into assembly code, mapping intermediate
//! representation variables into concrete memory locations.

use crate::asm::*;
use crate::identifiers::*;
use crate::middle_end::Lowerer;
use crate::ssa::*;

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
        todo!("finish implementing emit_prog")
    }
}
