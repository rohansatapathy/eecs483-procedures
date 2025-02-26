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

    fn insert_var(&mut self, var: &String, var_name: &VarName) {
        self.vars.insert(var.clone(), var_name.clone());
    }

    fn get_var_name(&self, var: &String) -> Option<&VarName> {
        self.vars.get(var)
    }

    fn insert_label(
        &mut self, label: &String, fun_name: &FunName, arity: usize,
    ) {
        self.labels
            .insert(label.clone(), EnvFun::new(fun_name.clone(), arity));
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
        env.insert_label(&prog.name, &name, 1);

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

                env.insert_label(&decl.name, &name, params.len());

                Ok(BoundExtDecl { name, params, loc })
            })
            .collect::<Result<Vec<BoundExtDecl>, _>>()?;

        // Add parameter to environment
        let param = self.vars.fresh(&prog.param.0);
        env.insert_var(&prog.param.0, &param);

        Ok(BoundProg {
            externs,
            name,
            param: (param, prog.param.1),
            body: self.resolve_expr(prog.body, &mut env)?,
            loc: prog.loc,
        })
    }

    fn resolve_params(
        &mut self, params: &Vec<(String, SrcLoc)>, env: &mut Env,
    ) -> Result<Vec<(VarName, SrcLoc)>, CompileErr> {
        let mut param_set: HashSet<String> = HashSet::new();

        Ok(params
            .iter()
            .map(|(param, loc)| {
                if !param_set.insert(param.clone()) {
                    return Err(CompileErr::DuplicateParameter(
                        param.clone(),
                        *loc,
                    ));
                }

                let param_var_name = self.vars.fresh(param);
                env.insert_var(param, &param_var_name);
                Ok((param_var_name, *loc))
            })
            .collect::<Result<_, _>>()?)
    }

    fn resolve_expr(
        &mut self, expr: SurfExpr, env: &mut Env,
    ) -> Result<BoundExpr, CompileErr> {
        todo!()
    }
}
