//! The frontend of our compiler processes source code into an
//! abstract syntax tree (AST). During this process, the frontend
//! ensures that variables are in scope and renames them to use unique
//! identifiers.

use crate::ast::*;
use crate::identifiers::*;
use crate::span::SrcLoc;

pub struct Resolver {
    pub vars: IdGen<VarName>,
    pub funs: IdGen<FunName>,
}

/// CompileErr is an error type that is used to report errors during
/// compilation.
///
/// In the following constructors, the String argument is the original
/// name of the variable or function and the SrcLoc argument is where
/// in the source program the error occurred.
#[derive(Debug, Clone)]
pub enum CompileErr {
    UnboundVariable(String, SrcLoc),
    DuplicateVariable(String, SrcLoc),
    UnboundFunction(String, SrcLoc),
    DuplicateFunction(String, SrcLoc),
    DuplicateParameter(String, SrcLoc),
    ArityMismatch { name: String, expected: usize, found: usize, loc: SrcLoc },
}

impl Resolver {
    pub fn new() -> Self {
        Resolver { vars: IdGen::new(), funs: IdGen::new() }
    }

    pub fn resolve_prog(&mut self, prog: SurfProg) -> Result<BoundProg, CompileErr> {
        todo!("resolve_prog not implemented")
    }
}
