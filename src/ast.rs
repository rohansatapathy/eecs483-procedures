use crate::identifiers::*;
pub use crate::span::*;

/* --------------------------------- Aliases -------------------------------- */

pub type SurfProg = Prog<String, String>;
pub type SurfExtDecl = ExtDecl<String, String>;
pub type SurfFunDecl = FunDecl<String, String>;
pub type SurfBinding = Binding<String, String>;
pub type SurfExpr = Expr<String, String>;

pub type BoundProg = Prog<VarName, FunName>;
pub type BoundExtDecl = ExtDecl<VarName, FunName>;
pub type BoundFunDecl = FunDecl<VarName, FunName>;
pub type BoundBinding = Binding<VarName, FunName>;
pub type BoundExpr = Expr<VarName, FunName>;

/* ----------------------------------- AST ---------------------------------- */

#[derive(Clone, Debug)]
pub struct Prog<Var, Fun> {
    pub externs: Vec<ExtDecl<Var, Fun>>,
    /// The name of the main function. Should always be "main".
    pub name: Fun,
    pub param: (Var, SrcLoc),
    pub body: Expr<Var, Fun>,
    pub loc: SrcLoc,
}

#[derive(Clone, Debug)]
pub enum Expr<Var, Fun> {
    Num(i64, SrcLoc),
    Bool(bool, SrcLoc),
    Var(Var, SrcLoc),
    Prim {
        prim: Prim,
        args: Vec<Expr<Var, Fun>>,
        loc: SrcLoc,
    },
    Let {
        bindings: Vec<Binding<Var, Fun>>,
        body: Box<Expr<Var, Fun>>,
        loc: SrcLoc,
    },
    If {
        cond: Box<Expr<Var, Fun>>,
        thn: Box<Expr<Var, Fun>>,
        els: Box<Expr<Var, Fun>>,
        loc: SrcLoc,
    },
    FunDefs {
        decls: Vec<FunDecl<Var, Fun>>,
        body: Box<Expr<Var, Fun>>,
        loc: SrcLoc,
    },
    Call {
        fun: Fun,
        args: Vec<Expr<Var, Fun>>,
        loc: SrcLoc,
    },
}

#[derive(Clone, Debug)]
pub struct ExtDecl<Var, Fun> {
    pub name: Fun,
    /// The parameters of an external declaration are merely
    /// used for pretty-printing purposes.
    pub params: Vec<(Var, SrcLoc)>,
    pub loc: SrcLoc,
}

#[derive(Clone, Debug)]
pub struct Binding<Var, Fun> {
    pub var: (Var, SrcLoc),
    pub expr: Expr<Var, Fun>,
}

#[derive(Clone, Debug)]
pub struct FunDecl<Var, Fun> {
    pub name: Fun,
    pub params: Vec<(Var, SrcLoc)>,
    pub body: Expr<Var, Fun>,
    pub loc: SrcLoc,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Prim {
    // unary arithmetic
    Add1,
    Sub1,
    // binary arithmetic
    Add,
    Sub,
    Mul,
    // unary logical
    Not,
    // binary logical
    And,
    Or,
    // binary comparison
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
}

impl Prim {
    pub fn arity(&self) -> usize {
        match self {
            Prim::Add1 | Prim::Sub1 | Prim::Not => 1,
            Prim::Add
            | Prim::Sub
            | Prim::Mul
            | Prim::And
            | Prim::Or
            | Prim::Lt
            | Prim::Le
            | Prim::Gt
            | Prim::Ge
            | Prim::Eq
            | Prim::Neq => 2,
        }
    }
}
