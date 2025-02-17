//! Interpreter for the snake language and its SSA form.

use crate::identifiers::*;
use std::{
    fmt::{self, Display},
    hash::Hash,
    rc::Rc,
};

#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Clone, Debug)]
pub enum InterpErr<Var, Fun> {
    Unimplemented,
    InvalidArg(String),
    UnboundVar(Var),
    UnboundFun(Fun),
    UnExpectedFun(Fun),
    CallToConst(i64),
    CallWrongArity { name: Fun, expected: usize, got: usize },
    UnboundBlock(BlockName),
    BrWrongArity { name: BlockName, expected: usize, got: usize },
}

impl<Var: Display, Fun: Display> Display for InterpErr<Var, Fun> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterpErr::Unimplemented => write!(f, "unimplemented"),
            InterpErr::InvalidArg(arg) => write!(f, "invalid argument: {}", arg),
            InterpErr::UnboundVar(var) => write!(f, "unbound variable: {}", var),
            InterpErr::UnboundFun(fun) => write!(f, "unbound function: {}", fun),
            InterpErr::UnExpectedFun(fun) => write!(f, "unexpected function: {}", fun),
            InterpErr::CallToConst(n) => write!(f, "call to constant: {}", n),
            InterpErr::CallWrongArity { name, expected, got } => {
                write!(
                    f,
                    "calling function {} with wrong arity: expected {}, got {}",
                    name, expected, got
                )
            }
            InterpErr::UnboundBlock(block) => write!(f, "unbound block: {}", block),
            InterpErr::BrWrongArity { name, expected, got } => {
                write!(
                    f,
                    "branching to block {} with wrong arity: expected {}, got {}",
                    name, expected, got
                )
            }
        }
    }
}

/* ---------------------------------- Snake --------------------------------- */

pub mod ast {
    use super::*;
    use crate::ast::*;
    use im::HashMap;

    #[derive(Clone)]
    pub struct Machine<Var, Fun> {
        redex: Redex<Var, Fun>,
        stack: Stack<Var, Fun>,
    }

    #[derive(Clone)]
    enum Redex<Var, Fun> {
        Decending { expr: Rc<Expr<Var, Fun>>, env: Env<Var, Fun> },
        Ascending(DynValue<Var, Fun>),
    }

    #[derive(Clone)]
    struct RcFunDef<Var, Fun> {
        params: Vec<Var>,
        body: Rc<Expr<Var, Fun>>,
    }

    #[derive(Clone)]
    struct Closure<Var, Fun> {
        env: Env<Var, Fun>,
        decls: HashMap<Fun, RcFunDef<Var, Fun>>,
        name: Fun,
    }

    #[derive(Clone)]
    enum DynValue<Var, Fun> {
        Int(i64),
        Closure(Closure<Var, Fun>),
    }

    #[derive(Clone, Hash, PartialEq, Eq)]
    enum VarOrFun<Var, Fun> {
        Var(Var),
        Fun(Fun),
    }

    type Env<Var, Fun> = HashMap<VarOrFun<Var, Fun>, DynValue<Var, Fun>>;

    #[derive(Clone)]
    enum Operator<Fun> {
        Prim(Prim),
        Call(Fun),
    }

    #[derive(Clone)]
    enum Stack<Var, Fun> {
        Return,
        Operation {
            operator: Operator<Fun>,
            env: Env<Var, Fun>,
            /// evaluated arguments
            evaluated: Vec<DynValue<Var, Fun>>,
            /// reversed remaining arguments
            remaining: Vec<Rc<Expr<Var, Fun>>>,
            stack: Box<Stack<Var, Fun>>,
        },
        Let {
            env: Env<Var, Fun>,
            var: Var,
            remaining: Vec<(Var, Rc<Expr<Var, Fun>>)>,
            body: Rc<Expr<Var, Fun>>,
            stack: Box<Stack<Var, Fun>>,
        },
        If {
            env: Env<Var, Fun>,
            thn: Rc<Expr<Var, Fun>>,
            els: Rc<Expr<Var, Fun>>,
            stack: Box<Stack<Var, Fun>>,
        },
    }

    impl<Var, Fun> Machine<Var, Fun>
    where
        Var: Hash + Eq + Clone,
        Fun: Hash + Eq + Clone,
    {
        pub fn run_prog(
            Prog { externs, name, param: (param, _), body, loc: _ }: &Prog<Var, Fun>, arg: String,
        ) -> Result<Value, InterpErr<Var, Fun>> {
            // Note: extern functions are not supported
            assert!(externs.is_empty(), "extern functions are not supported");

            let arg = DynValue::Int(arg.parse().map_err(|_| InterpErr::InvalidArg(arg))?);
            let mut env = HashMap::new();
            let decls = HashMap::from_iter([(
                name.clone(),
                RcFunDef { params: vec![param.clone()], body: Rc::new(body.clone()) },
            )]);
            env.insert(
                VarOrFun::Fun(name.clone()),
                DynValue::Closure(Closure { env: HashMap::new(), decls, name: name.clone() }),
            );
            env.insert(VarOrFun::Var(param.clone()), arg);
            let redex = Redex::Decending { expr: Rc::new(body.clone()), env };
            let machine = Machine { redex, stack: Stack::Return };
            match machine.run_expr()? {
                DynValue::Int(n) => Ok(Value::Int(n)),
                DynValue::Closure(Closure { name, .. }) => Err(InterpErr::UnExpectedFun(name)),
            }
        }
        fn run_expr(mut self) -> Result<DynValue<Var, Fun>, InterpErr<Var, Fun>> {
            loop {
                self = match self {
                    Machine { redex: Redex::Decending { expr, env }, stack } => {
                        Self::dive_expr(expr, env, stack)?
                    }
                    Machine { redex: Redex::Ascending(dv), stack: Stack::Return } => {
                        // the termination of the interpreter
                        break Ok(dv);
                    }
                    Machine { redex: Redex::Ascending(dv), stack } => Self::run_kont(dv, stack)?,
                };
            }
        }
        fn dive_expr(
            expr: Rc<Expr<Var, Fun>>, env: Env<Var, Fun>, stack: Stack<Var, Fun>,
        ) -> Result<Self, InterpErr<Var, Fun>> {
            let ret_machine =
                |dv: DynValue<Var, Fun>, stack| Machine { redex: Redex::Ascending(dv), stack };
            let dive_machine =
                |expr, env, stack| Machine { redex: Redex::Decending { expr, env }, stack };
            match expr.as_ref() {
                Expr::Num(n, _) => Ok(ret_machine(DynValue::Int(*n), stack)),
                Expr::Bool(b, _) => Ok(ret_machine(DynValue::Int(if *b { 1 } else { 0 }), stack)),
                Expr::Var(v, _) => {
                    let val = env
                        .get(&VarOrFun::Var(v.clone()))
                        .ok_or_else(|| InterpErr::UnboundVar(v.clone()))?;
                    Ok(ret_machine(val.clone(), stack))
                }
                Expr::Prim { prim, args, loc: _ } => {
                    Self::dive_operator(Operator::Prim(prim.clone()), args, env.clone(), stack)
                }
                Expr::Let { bindings, body, loc: _ } => {
                    let mut remaining: Vec<_> = bindings
                        .iter()
                        .cloned()
                        .rev()
                        .map(|Binding { var: (var, _), expr }| (var, Rc::new(expr.clone())))
                        .collect();
                    let body = Rc::new(body.as_ref().clone());
                    if let Some((var, expr)) = remaining.pop() {
                        let stack = Box::new(stack);
                        Ok(dive_machine(
                            expr,
                            env.clone(),
                            Stack::Let { env, var, remaining, body, stack },
                        ))
                    } else {
                        Ok(dive_machine(body, env.clone(), stack))
                    }
                }
                Expr::If { cond, thn, els, loc: _ } => {
                    let thn = Rc::new(thn.as_ref().clone());
                    let els = Rc::new(els.as_ref().clone());
                    let stack = Box::new(stack);
                    Ok(dive_machine(
                        Rc::new(cond.as_ref().clone()),
                        env.clone(),
                        Stack::If { env, thn, els, stack },
                    ))
                }
                Expr::FunDefs { decls, body, loc: _ } => {
                    let curr = env;
                    let mut next = curr.clone();
                    let decls = HashMap::from_iter(decls.iter().cloned().map(
                        |FunDecl { name, params, body, loc: _ }| {
                            (
                                name.clone(),
                                RcFunDef {
                                    params: params.into_iter().map(|(var, _)| var).collect(),
                                    body: Rc::new(body),
                                },
                            )
                        },
                    ));
                    for name in decls.keys() {
                        next.insert(
                            VarOrFun::Fun(name.clone()),
                            DynValue::Closure(Closure {
                                env: curr.clone(),
                                decls: decls.clone(),
                                name: name.clone(),
                            }),
                        );
                    }
                    Ok(dive_machine(Rc::new(body.as_ref().clone()), next, stack))
                }
                Expr::Call { fun, args, loc: _ } => {
                    Self::dive_operator(Operator::Call(fun.clone()), args, env.clone(), stack)
                }
            }
        }
        fn dive_operator(
            operator: Operator<Fun>, args: &[Expr<Var, Fun>], env: Env<Var, Fun>,
            stack: Stack<Var, Fun>,
        ) -> Result<Self, InterpErr<Var, Fun>> {
            let dive_machine =
                |expr, env, stack| Machine { redex: Redex::Decending { expr, env }, stack };
            let mut remaining: Vec<_> =
                args.iter().cloned().rev().map(|expr| Rc::new(expr.clone())).collect();
            if let Some(expr) = remaining.pop() {
                let stack = Box::new(stack);
                Ok(dive_machine(
                    expr,
                    env.clone(),
                    Stack::Operation { operator, env, evaluated: Vec::new(), remaining, stack },
                ))
            } else {
                let Operator::Call(fun) = operator else {
                    unreachable!(
                        "no arguments to evaluate in primitive operator, error in our interpreter?!"
                    )
                };
                Self::run_call(fun, Vec::new(), env, stack)
            }
        }
        fn run_kont(
            dv: DynValue<Var, Fun>, stack: Stack<Var, Fun>,
        ) -> Result<Self, InterpErr<Var, Fun>> {
            match stack {
                Stack::Return => {
                    unreachable!("return kont should not be run, error in our interpreter?!")
                }
                Stack::Operation { operator, env, mut evaluated, mut remaining, stack } => {
                    evaluated.push(dv);
                    if let Some(expr) = remaining.pop() {
                        Ok(Machine {
                            redex: Redex::Decending { expr, env: env.clone() },
                            stack: Stack::Operation { operator, env, evaluated, remaining, stack },
                        })
                    } else {
                        use std::ops::*;
                        match operator {
                            Operator::Prim(prim) => match prim {
                                Prim::Add1 => Self::run_prim1(|n| n + 1, evaluated, *stack),
                                Prim::Sub1 => Self::run_prim1(|n| n - 1, evaluated, *stack),
                                Prim::Not => Self::run_prim1(
                                    |n| if n == 0 { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Add => Self::run_prim2(Add::add, evaluated, *stack),
                                Prim::Sub => Self::run_prim2(Sub::sub, evaluated, *stack),
                                Prim::Mul => Self::run_prim2(Mul::mul, evaluated, *stack),
                                Prim::And => Self::run_prim2(
                                    |n, m| if n != 0 && m != 0 { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Or => Self::run_prim2(
                                    |n, m| if n != 0 || m != 0 { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Lt => Self::run_prim2(
                                    |n, m| if n < m { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Le => Self::run_prim2(
                                    |n, m| if n <= m { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Gt => Self::run_prim2(
                                    |n, m| if n > m { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Ge => Self::run_prim2(
                                    |n, m| if n >= m { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Eq => Self::run_prim2(
                                    |n, m| if n == m { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                                Prim::Neq => Self::run_prim2(
                                    |n, m| if n != m { 1 } else { 0 },
                                    evaluated,
                                    *stack,
                                ),
                            },
                            Operator::Call(fun) => Self::run_call(fun, evaluated, env, *stack),
                        }
                    }
                }
                Stack::Let { mut env, var, mut remaining, body, stack } => {
                    env.insert(VarOrFun::Var(var.clone()), dv);
                    if let Some((var, expr)) = remaining.pop() {
                        Ok(Machine {
                            redex: Redex::Decending { expr: expr.clone(), env: env.clone() },
                            stack: Stack::Let { env, var, remaining, body, stack },
                        })
                    } else {
                        let stack = *stack;
                        Ok(Machine { redex: Redex::Decending { expr: body.clone(), env }, stack })
                    }
                }
                Stack::If { env, thn, els, stack } => {
                    let n = match dv {
                        DynValue::Int(n) => n,
                        DynValue::Closure(Closure { name, .. }) => {
                            Err(InterpErr::UnExpectedFun(name))?
                        }
                    };
                    let stack = *stack;
                    if n != 0 {
                        let expr = thn.clone();
                        Ok(Machine { redex: Redex::Decending { expr, env }, stack })
                    } else {
                        let expr = els.clone();
                        Ok(Machine { redex: Redex::Decending { expr, env }, stack })
                    }
                }
            }
        }
        fn run_prim1(
            prim_f: impl Fn(i64) -> i64, args: Vec<DynValue<Var, Fun>>, stack: Stack<Var, Fun>,
        ) -> Result<Self, InterpErr<Var, Fun>> {
            if args.len() != 1 {
                unreachable!("wrong arity to unary primitive operator, error in our interpreter?!");
            }
            let n = match args.into_iter().next().unwrap() {
                DynValue::Int(n) => n,
                DynValue::Closure(Closure { name, .. }) => Err(InterpErr::UnExpectedFun(name))?,
            };
            let o = prim_f(n);
            Ok(Machine { redex: Redex::Ascending(DynValue::Int(o)), stack })
        }
        fn run_prim2(
            prim_f: impl Fn(i64, i64) -> i64, args: Vec<DynValue<Var, Fun>>, stack: Stack<Var, Fun>,
        ) -> Result<Self, InterpErr<Var, Fun>> {
            if args.len() != 2 {
                unreachable!(
                    "wrong arity to binary primitive operator, error in our interpreter?!"
                );
            }
            let args = args
                .into_iter()
                .map(|dv| match dv {
                    DynValue::Int(n) => Ok(n),
                    DynValue::Closure(Closure { name, .. }) => Err(InterpErr::UnExpectedFun(name)),
                })
                .collect::<Result<Vec<_>, InterpErr<Var, Fun>>>()?;
            let n1 = args[0];
            let n2 = args[1];
            let o = prim_f(n1, n2);
            Ok(Machine { redex: Redex::Ascending(DynValue::Int(o)), stack })
        }
        fn run_call(
            fun: Fun, args: Vec<DynValue<Var, Fun>>, env: Env<Var, Fun>, stack: Stack<Var, Fun>,
        ) -> Result<Self, InterpErr<Var, Fun>> {
            {
                let dv = env
                    .get(&VarOrFun::Fun(fun.clone()))
                    .ok_or_else(|| InterpErr::UnboundFun(fun.clone()))?;
                let Closure { env: clo_env, decls, name } = match dv {
                    DynValue::Closure(closure) => closure,
                    DynValue::Int(n) => Err(InterpErr::CallToConst(*n))?,
                };
                let mut env = clo_env.clone();
                for (name, _) in decls {
                    env.insert(
                        VarOrFun::Fun(name.clone()),
                        DynValue::Closure(Closure {
                            env: clo_env.clone(),
                            decls: decls.clone(),
                            name: name.clone(),
                        }),
                    );
                }
                let Some(RcFunDef { params, body }) = decls.get(&name) else {
                    unreachable!("no corresponding function in closure, error in our interpreter?!")
                };
                if args.len() != params.len() {
                    Err(InterpErr::CallWrongArity {
                        name: name.clone(),
                        expected: params.len(),
                        got: args.len(),
                    })?
                }
                for (param, arg) in params.iter().zip(args) {
                    env.insert(VarOrFun::Var(param.clone()), arg.clone());
                }
                Ok(Machine {
                    redex: Redex::Decending { expr: body.clone(), env: env.clone() },
                    stack,
                })
            }
        }
    }
}

/* ----------------------------------- SSA ---------------------------------- */

pub mod ssa {
    use super::*;
    use crate::ssa::*;
    use std::collections::HashMap;

    struct StackEnv(Frame, Vec<Frame>);
    impl StackEnv {
        fn new() -> Self {
            Self(Frame::new([]), Vec::new())
        }
        fn enter(&mut self) {
            let frame = std::mem::replace(&mut self.0, Frame::new([]));
            self.1.push(frame);
        }
        fn exit(&mut self) {
            self.0 = self.1.pop().unwrap();
        }
        fn current(&mut self) -> &mut Frame {
            &mut self.0
        }
    }
    struct Frame(HashMap<VarName, (usize, Value)>);
    impl Frame {
        fn new(param_assign: impl IntoIterator<Item = (VarName, Value)>) -> Self {
            Self(HashMap::from_iter(
                param_assign.into_iter().enumerate().map(|(pos, (var, val))| (var, (pos, val))),
            ))
        }
        fn len(&self) -> usize {
            self.0.len()
        }
        fn insert(&mut self, var: VarName, val: Value) {
            let pos = self.0.len();
            self.0.insert(var, (pos, val));
        }
        fn get(&self, var: &VarName) -> Option<(usize, &Value)> {
            self.0.get(var).map(|(pos, val)| (*pos, val))
        }
        fn chop(&mut self, anchor: usize) {
            self.0.retain(|_, (p, _)| *p < anchor);
        }
    }

    #[derive(Clone)]
    struct AnchorBlock {
        /// the position on the stack indicating the start of the block
        anchor: usize,
        params: Vec<VarName>,
        body: BlockBody,
    }

    pub struct Interp {
        stack: StackEnv,
        kont: Vec<(VarName, BlockBody)>,
        funs: im::HashMap<FunName, FunBlock>,
        blocks: im::HashMap<BlockName, AnchorBlock>,
    }

    /// Trampoline for the interpreter.
    enum State {
        Return(Value),
        Operation(Operation, VarName, BlockBody),
        OpReturn(Value),
        Call(FunName, Vec<Value>),
        Branch(Branch),
        BlockBody(BlockBody),
        Terminator(Terminator),
    }

    impl Interp {
        pub fn new() -> Self {
            Self {
                stack: StackEnv::new(),
                kont: Vec::new(),
                funs: im::HashMap::new(),
                blocks: im::HashMap::new(),
            }
        }
        fn alloc(&mut self, var: VarName, val: Value) {
            let frame = self.stack.current();
            frame.insert(var, val);
        }

        pub fn run(
            &mut self, Program { externs, funs, blocks }: &Program, arg: String,
        ) -> Result<Value, InterpErr<VarName, FunName>> {
            let val = Value::Int(arg.parse().map_err(|_| InterpErr::InvalidArg(arg))?);
            // Note: extern functions are not supported
            assert!(externs.is_empty(), "extern functions are not supported");

            self.funs.extend(funs.iter().cloned().map(|f| (f.name.clone(), f.clone())));
            self.blocks.extend(blocks.iter().cloned().map(|BasicBlock { label, params, body }| {
                (label.clone(), AnchorBlock { anchor: 0, params, body })
            }));

            let mut state = self.run_call(&FunName::unmangled("entry"), vec![val])?;
            loop {
                match state {
                    State::Return(val) => match self.kont.pop() {
                        Some((dest, next)) => {
                            self.stack.exit();
                            self.alloc(dest.clone(), val);
                            state = State::BlockBody(next.clone())
                        }
                        None => return Ok(val),
                    },
                    State::OpReturn(val) => match self.kont.pop() {
                        Some((dest, next)) => {
                            self.alloc(dest.clone(), val);
                            state = State::BlockBody(next.clone())
                        }
                        None => {
                            unreachable!("no return kont for operation, error in our interpreter?!")
                        }
                    },
                    State::Operation(op, dest, next) => {
                        self.kont.push((dest.clone(), next.clone()));
                        state = self.run_operation(&op)?
                    }
                    State::Call(fun, args) => {
                        self.stack.enter();
                        state = self.run_call(&fun, args)?
                    }
                    State::Branch(branch) => state = self.run_branch(&branch)?,
                    State::BlockBody(body) => state = self.run_block_body(&body)?,
                    State::Terminator(terminator) => state = self.run_terminator(&terminator)?,
                }
            }
        }

        /// Run a function call. A frame is already entered before calling this.
        fn run_call(
            &mut self, fun: &FunName, args: Vec<Value>,
        ) -> Result<State, InterpErr<VarName, FunName>> {
            let FunBlock { name: _, params, body: branch } = self.funs[fun].clone();
            for (param, arg) in params.iter().zip(args) {
                self.alloc(param.clone(), arg.clone());
            }
            Ok(State::Branch(branch.clone()))
        }

        fn run_branch(
            &mut self, Branch { target, args }: &Branch,
        ) -> Result<State, InterpErr<VarName, FunName>> {
            let args =
                args.iter().map(|imm| self.run_immediate(imm)).collect::<Result<Vec<_>, _>>()?;
            let AnchorBlock { anchor, params, body } = self.blocks[target].clone();
            self.stack.current().chop(anchor);
            for (param, arg) in params.iter().zip(args) {
                self.alloc(param.clone(), arg.clone());
            }
            Ok(State::BlockBody(body.clone()))
        }
        fn run_block_body(
            &mut self, block: &BlockBody,
        ) -> Result<State, InterpErr<VarName, FunName>> {
            match block {
                BlockBody::Terminator(terminator) => Ok(State::Terminator(terminator.clone())),
                BlockBody::Operation { dest, op, next } => {
                    Ok(State::Operation(op.clone(), dest.clone(), next.as_ref().clone()))
                }
                BlockBody::SubBlocks { blocks, next } => {
                    let anchor = self.stack.current().len();
                    self.blocks.extend(blocks.iter().cloned().map(
                        |BasicBlock { label, params, body }| {
                            (label.clone(), AnchorBlock { anchor, params, body })
                        },
                    ));
                    Ok(State::BlockBody(next.as_ref().clone()))
                }
            }
        }

        fn run_terminator(
            &mut self, terminator: &Terminator,
        ) -> Result<State, InterpErr<VarName, FunName>> {
            match terminator {
                Terminator::Return(imm) => Ok(State::Return(self.run_immediate(imm)?)),
                Terminator::Branch(br) => Ok(State::Branch(br.clone())),
                Terminator::ConditionalBranch { cond, thn, els } => {
                    let Value::Int(n) = self.run_immediate(cond)?;
                    if n != 0 {
                        Ok(State::Branch(Branch { target: thn.clone(), args: Vec::new() }))
                    } else {
                        Ok(State::Branch(Branch { target: els.clone(), args: Vec::new() }))
                    }
                }
            }
        }

        fn run_operation(&mut self, op: &Operation) -> Result<State, InterpErr<VarName, FunName>> {
            match op {
                Operation::Immediate(imm) => Ok(State::OpReturn(self.run_immediate(imm)?)),
                Operation::Prim1(prim, imm) => {
                    let Value::Int(n) = self.run_immediate(imm)?;
                    let o = match prim {
                        Prim1::BitNot => !n,
                        Prim1::IntToBool => {
                            if n != 0 {
                                1
                            } else {
                                0
                            }
                        }
                    };
                    Ok(State::OpReturn(Value::Int(o)))
                }
                Operation::Prim2(prim, imm1, imm2) => {
                    let Value::Int(n) = self.run_immediate(imm1)?;
                    let Value::Int(m) = self.run_immediate(imm2)?;
                    let o = match prim {
                        Prim2::Add => n + m,
                        Prim2::Sub => n - m,
                        Prim2::Mul => n * m,
                        Prim2::BitAnd => n & m,
                        Prim2::BitOr => n | m,
                        Prim2::BitXor => n ^ m,
                        Prim2::Lt => (if n < m { 1 } else { 0 }).clone(),
                        Prim2::Le => (if n <= m { 1 } else { 0 }).clone(),
                        Prim2::Gt => (if n > m { 1 } else { 0 }).clone(),
                        Prim2::Ge => (if n >= m { 1 } else { 0 }).clone(),
                        Prim2::Eq => (if n == m { 1 } else { 0 }).clone(),
                        Prim2::Neq => (if n != m { 1 } else { 0 }).clone(),
                    };
                    Ok(State::OpReturn(Value::Int(o)))
                }
                Operation::Call { fun, args } => {
                    let args = args
                        .iter()
                        .map(|imm| self.run_immediate(imm))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(State::Call(fun.clone(), args))
                }
            }
        }

        fn run_immediate(&mut self, imm: &Immediate) -> Result<Value, InterpErr<VarName, FunName>> {
            match imm {
                Immediate::Var(v) => {
                    let (_, val) =
                        self.stack.current().get(v).ok_or(InterpErr::UnboundVar(v.clone()))?;
                    Ok(val.clone())
                }
                Immediate::Const(n) => Ok(Value::Int(*n)),
            }
        }
    }
}
