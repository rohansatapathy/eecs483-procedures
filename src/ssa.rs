use crate::identifiers::*;

// A Program has a single input parameter, and a block of straightline code to execute
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub externs: Vec<Extern>,
    pub funs: Vec<FunBlock>,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extern {
    pub name: FunName,
    pub params: Vec<VarName>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunBlock {
    pub name: FunName,
    pub params: Vec<VarName>,
    pub body: Branch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    pub label: BlockName,
    pub params: Vec<VarName>,
    pub body: BlockBody,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockBody {
    Terminator(Terminator),
    Operation { dest: VarName, op: Operation, next: Box<BlockBody> },
    SubBlocks { blocks: Vec<BasicBlock>, next: Box<BlockBody> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Return(Immediate),
    Branch(Branch),
    ConditionalBranch { cond: Immediate, thn: BlockName, els: BlockName },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    pub target: BlockName,
    pub args: Vec<Immediate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Immediate(Immediate),
    Prim1(Prim1, Immediate),
    Prim2(Prim2, Immediate, Immediate),
    Call { fun: FunName, args: Vec<Immediate> },
}

#[derive(Clone, PartialEq, Eq)]
pub enum Prim1 {
    BitNot,
    IntToBool,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Prim2 {
    // arithmetic
    Add,
    Sub,
    Mul,
    // logical
    BitAnd,
    BitOr,
    BitXor,
    // comparison
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Immediate {
    Const(i64),
    Var(VarName),
}
