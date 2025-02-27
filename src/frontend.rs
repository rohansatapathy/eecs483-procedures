//! The frontend of our compiler processes source code into an
//! abstract syntax tree (AST). During this process, the frontend
//! ensures that variables are in scope and renames them to use unique
//! identifiers.

use crate::ast::*;
use crate::identifiers::*;
use crate::span::SrcLoc;
use im::HashMap;
use std::collections::HashSet;

pub struct Resolver {
    pub vars: IdGen<VarName>,
    pub funs: IdGen<FunName>,
}

#[derive(Debug, Clone)]
struct EnvFun {
    name: FunName,
    arity: usize,
}

impl EnvFun {
    fn new(name: FunName, arity: usize) -> Self {
        Self { name, arity }
    }
}

#[derive(Debug, Clone)]
struct Env {
    vars: HashMap<String, VarName>,
    labels: HashMap<String, EnvFun>,
}

impl Env {
    fn new() -> Self {
        Self { vars: HashMap::new(), labels: HashMap::new() }
    }

    fn insert_var(&mut self, var: String, var_name: VarName) {
        self.vars.insert(var, var_name);
    }

    fn get_var_name(&self, var: &String) -> Option<&VarName> {
        self.vars.get(var)
    }

    fn insert_label(
        &mut self, label: String, fun_name: FunName, arity: usize,
    ) {
        self.labels.insert(label, EnvFun::new(fun_name, arity));
    }

    fn get_env_fun(&self, label: &String) -> Option<&EnvFun> {
        self.labels.get(label)
    }
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
    ArityMismatch {
        name: String,
        expected: usize,
        found: usize,
        loc: SrcLoc,
    },
}

impl Resolver {
    pub fn new() -> Self {
        Resolver { vars: IdGen::new(), funs: IdGen::new() }
    }

    pub fn resolve_prog(
        &mut self, prog: SurfProg,
    ) -> Result<BoundProg, CompileErr> {
        let mut env = Env::new();

        // Add main function to environment
        let name = FunName::Unmangled("entry".to_string());
        env.insert_label(prog.name.clone(), name.clone(), 1);

        // Add extern functions to environment
        let externs = prog
            .externs
            .iter()
            .map(|decl| {
                if let Some(_) = env.get_env_fun(&decl.name) {
                    return Err(CompileErr::DuplicateFunction(
                        decl.name.clone(),
                        decl.loc,
                    ));
                }

                let name = FunName::Unmangled(decl.name.clone());
                let params =
                    self.resolve_params(&decl.params, &mut env.clone())?;
                let loc = decl.loc;

                env.insert_label(
                    decl.name.clone(),
                    name.clone(),
                    params.len(),
                );

                Ok(BoundExtDecl { name, params, loc })
            })
            .collect::<Result<Vec<BoundExtDecl>, _>>()?;

        // Add parameter to environment
        let param = self.vars.fresh(&prog.param.0);
        env.insert_var(prog.param.0, param.clone());

        Ok(BoundProg {
            externs,
            name,
            param: (param, prog.param.1),
            body: self.resolve_expr(prog.body, env)?,
            loc: prog.loc,
        })
    }

    fn resolve_params(
        &mut self, params: &Vec<(String, SrcLoc)>, env: &mut Env,
    ) -> Result<Vec<(VarName, SrcLoc)>, CompileErr> {
        // Check for duplicates
        let mut param_set: HashSet<String> = HashSet::new();
        for (param, loc) in params {
            if !param_set.insert(param.clone()) {
                return Err(CompileErr::DuplicateParameter(
                    param.clone(),
                    *loc,
                ));
            }
        }

        Ok(params
            .iter()
            .map(|(param, loc)| {
                let param_var_name = self.vars.fresh(param);
                env.insert_var(param.clone(), param_var_name.clone());
                (param_var_name, *loc)
            })
            .collect())
    }

    fn resolve_expr(
        &mut self, expr: SurfExpr, mut env: Env,
    ) -> Result<BoundExpr, CompileErr> {
        let bound_expr = match expr {
            Expr::Num(n, loc) => Expr::Num(n, loc),
            Expr::Bool(b, loc) => Expr::Bool(b, loc),
            Expr::Var(var, loc) => Expr::Var(
                env.get_var_name(&var)
                    .ok_or(CompileErr::UnboundVariable(var.clone(), loc))?
                    .clone(),
                loc,
            ),
            Expr::Prim { prim, args, loc } => Expr::Prim {
                prim,
                args: args
                    .into_iter()
                    .map(|arg| self.resolve_expr(arg, env.clone()))
                    .collect::<Result<_, _>>()?,
                loc,
            },
            Expr::Let { bindings, body, loc } => {
                let mut dup: HashSet<String> = HashSet::new();
                for binding in &bindings {
                    if !dup.insert(binding.var.0.clone()) {
                        return Err(CompileErr::DuplicateVariable(
                            binding.var.0.clone(),
                            binding.var.1,
                        ));
                    }
                }

                let bindings = bindings
                    .into_iter()
                    .map(|binding| {
                        let var_name = self.vars.fresh(&binding.var.0);
                        let expr =
                            self.resolve_expr(binding.expr, env.clone())?;

                        env.insert_var(binding.var.0, var_name.clone());
                        Ok(Binding { var: (var_name, binding.var.1), expr })
                    })
                    .collect::<Result<_, _>>()?;

                Expr::Let {
                    bindings,
                    body: Box::new(self.resolve_expr(*body, env)?),
                    loc,
                }
            }
            Expr::If { cond, thn, els, loc } => Expr::If {
                cond: Box::new(self.resolve_expr(*cond, env.clone())?),
                thn: Box::new(self.resolve_expr(*thn, env.clone())?),
                els: Box::new(self.resolve_expr(*els, env)?),
                loc,
            },
            Expr::FunDefs { decls, body, loc } => {
                // Check for duplication. If there are no duplicates, add
                // function names to env before resolving them.
                let mut dup: HashSet<String> = HashSet::new();
                for decl in &decls {
                    if !dup.insert(decl.name.clone()) {
                        return Err(CompileErr::DuplicateFunction(
                            decl.name.clone(),
                            decl.loc,
                        ));
                    }
                    env.insert_label(
                        decl.name.clone(),
                        self.funs.fresh(&decl.name),
                        decl.params.len(),
                    );
                }

                let decls = decls
                    .into_iter()
                    .map(|decl| self.resolve_fun_decl(decl, env.clone()))
                    .collect::<Result<_, _>>()?;

                let body = self.resolve_expr(*body, env)?;

                Expr::FunDefs { decls, body: Box::new(body), loc }
            }
            Expr::Call { fun, args, loc } => {
                let env_fun = env.get_env_fun(&fun).ok_or_else(|| {
                    CompileErr::UnboundFunction(fun.clone(), loc)
                })?;

                if env_fun.arity != args.len() {
                    return Err(CompileErr::ArityMismatch {
                        name: fun.clone(),
                        expected: env_fun.arity,
                        found: args.len(),
                        loc,
                    });
                }

                let fun = env_fun.name.clone();
                let args = args
                    .into_iter()
                    .map(|arg| self.resolve_expr(arg, env.clone()))
                    .collect::<Result<Vec<_>, _>>()?;

                Expr::Call { fun, args, loc }
            }
        };

        Ok(bound_expr)
    }

    /// Resolve a single function declaration.
    ///
    /// Assume that the declaration name has already been checked for
    /// duplication and that the function name is already in env.
    fn resolve_fun_decl(
        &mut self, decl: SurfFunDecl, mut env: Env,
    ) -> Result<BoundFunDecl, CompileErr> {
        let name = env
            .get_env_fun(&decl.name)
            .expect("FunDecl should already be in env")
            .name
            .clone();
        let params = self.resolve_params(&decl.params, &mut env)?;
        let body = self.resolve_expr(decl.body, env.clone())?;

        Ok(BoundFunDecl { name, params, body, loc: decl.loc })
    }
}
