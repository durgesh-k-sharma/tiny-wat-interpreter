use sexp::{parse, Atom, Sexp};
use std::collections::HashMap;

/// An AST expression representing a folded WebAssembly instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Push a 32-bit signed integer constant: `(i32.const n)`
    Const(i32),
    /// Retrieve value of a local variable: `(local.get $name)`
    LocalGet(String),
    /// Pop two operands and push their sum: `(i32.add lhs rhs)`
    Add(Box<Expr>, Box<Expr>),
    /// Pop two operands and push (lhs - rhs): `(i32.sub lhs rhs)`
    Sub(Box<Expr>, Box<Expr>),
    /// Pop two operands and push their product: `(i32.mul lhs rhs)`
    Mul(Box<Expr>, Box<Expr>),
    /// Pop two operands and push signed remainder: `(i32.rem_s lhs rhs)`
    RemS(Box<Expr>, Box<Expr>),
    /// Pop two operands and push 1 if lhs < rhs else 0: `(i32.lt_s lhs rhs)`
    LtS(Box<Expr>, Box<Expr>),
    /// Conditional execution: `(if (result i32) cond (then ...) (else ...))`
    If {
        condition: Box<Expr>,
        then_branch: Vec<Expr>,
        else_branch: Vec<Expr>,
    },
    /// Function invocation: `(call $func arg1 arg2 ...)`
    Call { func_name: String, args: Vec<Expr> },
}

/// A parsed WebAssembly function declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub export_name: Option<String>,
    pub params: Vec<String>,
    pub result: Option<String>,
    pub body: Vec<Expr>,
}

/// A parsed WebAssembly module consisting of functions and export mappings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Module {
    pub funcs: HashMap<String, Function>,
    pub exports: HashMap<String, String>,
}

impl Module {
    /// Parses a WebAssembly Text (WAT) format module from an S-expression string.
    pub fn parse_wat(wat: &str) -> Result<Self, String> {
        let module_expr = parse(wat).map_err(|error| format!("WAT parse error: {error:?}"))?;
        let module_list = match module_expr {
            Sexp::List(list) if !list.is_empty() && atom_text(&list[0]) == Some("module") => list,
            _ => return Err("Top-level element must be (module ...)".to_string()),
        };

        let mut module = Module::default();
        for item in &module_list[1..] {
            match item {
                Sexp::List(func_list)
                    if !func_list.is_empty() && atom_text(&func_list[0]) == Some("func") =>
                {
                    let (func, export_opt) = parse_func(func_list)?;
                    if let Some(export_name) = export_opt {
                        module.exports.insert(export_name, func.name.clone());
                    }
                    module.funcs.insert(func.name.clone(), func);
                }
                _ => return Err(format!("Unexpected module element: {item:?}")),
            }
        }
        Ok(module)
    }

    /// Invokes an exported function by name with the given `i32` arguments.
    pub fn invoke(&self, export_name: &str, args: &[i32]) -> Result<i32, String> {
        let func_name = self
            .exports
            .get(export_name)
            .or_else(|| self.funcs.get(export_name).map(|f| &f.name))
            .ok_or_else(|| format!("Export or function '{export_name}' not found in module"))?;

        let func = self
            .funcs
            .get(func_name)
            .ok_or_else(|| format!("Function '{func_name}' not found"))?;

        if func.params.len() != args.len() {
            return Err(format!(
                "Argument count mismatch for '{export_name}': expected {}, got {}",
                func.params.len(),
                args.len()
            ));
        }

        let mut env = HashMap::new();
        for (param, &arg) in func.params.iter().zip(args.iter()) {
            env.insert(param.as_str(), arg);
        }

        self.eval_function_body(func, &env)
    }

    fn eval_function_body(&self, func: &Function, env: &HashMap<&str, i32>) -> Result<i32, String> {
        if func.body.is_empty() {
            return Ok(0);
        }
        let mut last_val = 0;
        for expr in &func.body {
            last_val = self.eval_expr(expr, env)?;
        }
        Ok(last_val)
    }

    fn eval_expr(&self, expr: &Expr, env: &HashMap<&str, i32>) -> Result<i32, String> {
        match expr {
            Expr::Const(val) => Ok(*val),
            Expr::LocalGet(name) => env
                .get(name.as_str())
                .copied()
                .ok_or_else(|| format!("Unknown local variable: {name}")),
            Expr::Add(lhs, rhs) => {
                let left = self.eval_expr(lhs, env)?;
                let right = self.eval_expr(rhs, env)?;
                Ok(left.wrapping_add(right))
            }
            Expr::Sub(lhs, rhs) => {
                let left = self.eval_expr(lhs, env)?;
                let right = self.eval_expr(rhs, env)?;
                Ok(left.wrapping_sub(right))
            }
            Expr::Mul(lhs, rhs) => {
                let left = self.eval_expr(lhs, env)?;
                let right = self.eval_expr(rhs, env)?;
                Ok(left.wrapping_mul(right))
            }
            Expr::RemS(lhs, rhs) => {
                let left = self.eval_expr(lhs, env)?;
                let right = self.eval_expr(rhs, env)?;
                if right == 0 {
                    return Err("Division by zero in i32.rem_s".to_string());
                }
                if left == i32::MIN && right == -1 {
                    Ok(0)
                } else {
                    Ok(left.wrapping_rem(right))
                }
            }
            Expr::LtS(lhs, rhs) => {
                let left = self.eval_expr(lhs, env)?;
                let right = self.eval_expr(rhs, env)?;
                Ok(if left < right { 1 } else { 0 })
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.eval_expr(condition, env)?;
                let branch = if cond_val != 0 {
                    then_branch
                } else {
                    else_branch
                };
                let mut res = 0;
                for expr in branch {
                    res = self.eval_expr(expr, env)?;
                }
                Ok(res)
            }
            Expr::Call { func_name, args } => {
                let func = self
                    .funcs
                    .get(func_name)
                    .ok_or_else(|| format!("Undefined function called: {func_name}"))?;

                if func.params.len() != args.len() {
                    return Err(format!(
                        "Argument count mismatch in call to '{func_name}': expected {}, got {}",
                        func.params.len(),
                        args.len()
                    ));
                }

                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg_expr in args {
                    evaluated_args.push(self.eval_expr(arg_expr, env)?);
                }

                let mut callee_env = HashMap::new();
                for (param, val) in func.params.iter().zip(evaluated_args) {
                    callee_env.insert(param.as_str(), val);
                }

                self.eval_function_body(func, &callee_env)
            }
        }
    }
}

fn atom_text(expression: &Sexp) -> Option<&str> {
    match expression {
        Sexp::Atom(Atom::S(text)) => Some(text),
        _ => None,
    }
}

fn unquote(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_i32(sexp: &Sexp) -> Result<i32, String> {
    match sexp {
        Sexp::Atom(Atom::I(val)) => Ok(*val as i32),
        Sexp::Atom(Atom::S(text)) => {
            let text = text.trim();
            let text_clean = text.trim_start_matches('+');
            if let Some(hex) = text_clean
                .strip_prefix("0x")
                .or_else(|| text_clean.strip_prefix("0X"))
            {
                i32::from_str_radix(hex, 16)
                    .map_err(|e| format!("Invalid hex integer '{text}': {e}"))
            } else if let Some(hex_neg) = text
                .strip_prefix("-0x")
                .or_else(|| text.strip_prefix("-0X"))
            {
                i32::from_str_radix(hex_neg, 16)
                    .map(|v| -v)
                    .map_err(|e| format!("Invalid hex integer '{text}': {e}"))
            } else {
                text_clean
                    .parse::<i32>()
                    .map_err(|e| format!("Invalid integer '{text}': {e}"))
            }
        }
        _ => Err(format!("Expected integer constant, got {sexp:?}")),
    }
}

fn parse_func(list: &[Sexp]) -> Result<(Function, Option<String>), String> {
    let mut func_name = String::new();
    let mut export_name = None;
    let mut params = Vec::new();
    let mut result = None;
    let mut body = Vec::new();

    let mut idx = 1;
    if idx < list.len() {
        if let Some(name) = atom_text(&list[idx]) {
            if name.starts_with('$') {
                func_name = name.to_string();
                idx += 1;
            }
        }
    }

    while idx < list.len() {
        match &list[idx] {
            Sexp::List(inner) if !inner.is_empty() => match atom_text(&inner[0]) {
                Some("export") => {
                    if inner.len() != 2 {
                        return Err(format!("Malformed export declaration: {:?}", list[idx]));
                    }
                    let exp = atom_text(&inner[1]).ok_or_else(|| {
                        format!("Expected export name string, got {:?}", inner[1])
                    })?;
                    export_name = Some(unquote(exp));
                }
                Some("param") => {
                    if inner.len() != 3 {
                        return Err(format!("Malformed param declaration: {:?}", list[idx]));
                    }
                    let param_name = atom_text(&inner[1])
                        .ok_or_else(|| format!("Expected param name, got {:?}", inner[1]))?;
                    let param_type = atom_text(&inner[2])
                        .ok_or_else(|| format!("Expected param type, got {:?}", inner[2]))?;
                    if param_type != "i32" {
                        return Err(format!("Unsupported param type: {param_type}"));
                    }
                    params.push(param_name.to_string());
                }
                Some("result") => {
                    if inner.len() != 2 {
                        return Err(format!("Malformed result declaration: {:?}", list[idx]));
                    }
                    let res_type = atom_text(&inner[1])
                        .ok_or_else(|| format!("Expected result type, got {:?}", inner[1]))?;
                    if res_type != "i32" {
                        return Err(format!("Unsupported result type: {res_type}"));
                    }
                    result = Some(res_type.to_string());
                }
                _ => {
                    let expr = parse_expr(&list[idx])?;
                    body.push(expr);
                }
            },
            _ => {
                return Err(format!(
                    "Unexpected token in function definition: {:?}",
                    list[idx]
                ));
            }
        }
        idx += 1;
    }

    if func_name.is_empty() {
        if let Some(ref exp) = export_name {
            func_name = format!("${exp}");
        } else {
            return Err("Function must have a name or an export declaration".to_string());
        }
    }

    let func = Function {
        name: func_name,
        export_name: export_name.clone(),
        params,
        result,
        body,
    };
    Ok((func, export_name))
}

fn parse_expr(sexp: &Sexp) -> Result<Expr, String> {
    match sexp {
        Sexp::List(list) if !list.is_empty() => {
            let op = atom_text(&list[0])
                .ok_or_else(|| format!("Expected instruction operator, got {:?}", list[0]))?;
            match op {
                "i32.const" => {
                    if list.len() != 2 {
                        return Err(format!("i32.const requires 1 argument, got {:?}", list));
                    }
                    let val = parse_i32(&list[1])?;
                    Ok(Expr::Const(val))
                }
                "local.get" => {
                    if list.len() != 2 {
                        return Err(format!("local.get requires 1 argument, got {:?}", list));
                    }
                    let name = atom_text(&list[1]).ok_or_else(|| {
                        format!("Expected variable name in local.get, got {:?}", list[1])
                    })?;
                    Ok(Expr::LocalGet(name.to_string()))
                }
                "i32.add" => {
                    if list.len() != 3 {
                        return Err(format!("i32.add requires 2 operands, got {:?}", list));
                    }
                    let lhs = parse_expr(&list[1])?;
                    let rhs = parse_expr(&list[2])?;
                    Ok(Expr::Add(Box::new(lhs), Box::new(rhs)))
                }
                "i32.sub" => {
                    if list.len() != 3 {
                        return Err(format!("i32.sub requires 2 operands, got {:?}", list));
                    }
                    let lhs = parse_expr(&list[1])?;
                    let rhs = parse_expr(&list[2])?;
                    Ok(Expr::Sub(Box::new(lhs), Box::new(rhs)))
                }
                "i32.mul" => {
                    if list.len() != 3 {
                        return Err(format!("i32.mul requires 2 operands, got {:?}", list));
                    }
                    let lhs = parse_expr(&list[1])?;
                    let rhs = parse_expr(&list[2])?;
                    Ok(Expr::Mul(Box::new(lhs), Box::new(rhs)))
                }
                "i32.rem_s" => {
                    if list.len() != 3 {
                        return Err(format!("i32.rem_s requires 2 operands, got {:?}", list));
                    }
                    let lhs = parse_expr(&list[1])?;
                    let rhs = parse_expr(&list[2])?;
                    Ok(Expr::RemS(Box::new(lhs), Box::new(rhs)))
                }
                "i32.lt_s" => {
                    if list.len() != 3 {
                        return Err(format!("i32.lt_s requires 2 operands, got {:?}", list));
                    }
                    let lhs = parse_expr(&list[1])?;
                    let rhs = parse_expr(&list[2])?;
                    Ok(Expr::LtS(Box::new(lhs), Box::new(rhs)))
                }
                "call" => {
                    if list.len() < 2 {
                        return Err(format!("call requires function name, got {:?}", list));
                    }
                    let target_name = atom_text(&list[1]).ok_or_else(|| {
                        format!("Expected function name in call, got {:?}", list[1])
                    })?;
                    let mut args = Vec::new();
                    for arg_sexp in &list[2..] {
                        args.push(parse_expr(arg_sexp)?);
                    }
                    Ok(Expr::Call {
                        func_name: target_name.to_string(),
                        args,
                    })
                }
                "if" => {
                    let mut condition = None;
                    let mut then_branch = Vec::new();
                    let mut else_branch = Vec::new();

                    for item in &list[1..] {
                        match item {
                            Sexp::List(sub) if !sub.is_empty() => match atom_text(&sub[0]) {
                                Some("result") => {
                                    // optional result type specification
                                }
                                Some("then") => {
                                    for then_item in &sub[1..] {
                                        then_branch.push(parse_expr(then_item)?);
                                    }
                                }
                                Some("else") => {
                                    for else_item in &sub[1..] {
                                        else_branch.push(parse_expr(else_item)?);
                                    }
                                }
                                _ => {
                                    if condition.is_none() {
                                        condition = Some(parse_expr(item)?);
                                    } else {
                                        return Err(format!(
                                            "Unexpected expression in if: {:?}",
                                            item
                                        ));
                                    }
                                }
                            },
                            _ => {
                                return Err(format!("Unexpected token in if: {:?}", item));
                            }
                        }
                    }

                    let condition = condition
                        .ok_or_else(|| format!("Missing condition in if expression: {:?}", list))?;

                    Ok(Expr::If {
                        condition: Box::new(condition),
                        then_branch,
                        else_branch,
                    })
                }
                other => Err(format!("Unsupported instruction or keyword: {other}")),
            }
        }
        _ => Err(format!("Expected expression list, got {:?}", sexp)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_invalid_module() {
        assert!(Module::parse_wat("not a module").is_err());
        assert!(Module::parse_wat("(func $foo)").is_err());
        assert!(Module::parse_wat("(module (unsupported_form))").is_err());
    }

    #[test]
    fn test_arithmetic_and_overflow() {
        let wat = r#"
        (module
            (func $math (export "math") (param $a i32) (param $b i32) (result i32)
                (i32.add (local.get $a) (local.get $b))
            )
            (func $mul (export "mul") (param $a i32) (param $b i32) (result i32)
                (i32.mul (local.get $a) (local.get $b))
            )
            (func $rem (export "rem") (param $a i32) (param $b i32) (result i32)
                (i32.rem_s (local.get $a) (local.get $b))
            )
            (func $lt (export "lt") (param $a i32) (param $b i32) (result i32)
                (i32.lt_s (local.get $a) (local.get $b))
            )
        )
        "#;
        let module = Module::parse_wat(wat).unwrap();

        // 32-bit wrapping addition
        assert_eq!(module.invoke("math", &[i32::MAX, 1]).unwrap(), i32::MIN);
        // Multiplication wrapping
        assert_eq!(
            module.invoke("mul", &[100_000, 100_000]).unwrap(),
            (100_000i64 * 100_000i64) as i32
        );
        // Signed comparison
        assert_eq!(module.invoke("lt", &[-5, 5]).unwrap(), 1);
        assert_eq!(module.invoke("lt", &[5, -5]).unwrap(), 0);
        // Remainder and overflow edge cases
        assert_eq!(module.invoke("rem", &[10, 3]).unwrap(), 1);
        assert_eq!(module.invoke("rem", &[-10, 3]).unwrap(), -1);
        assert_eq!(module.invoke("rem", &[i32::MIN, -1]).unwrap(), 0);
    }

    #[test]
    fn test_division_by_zero_error() {
        let wat = r#"
        (module
            (func $rem (export "rem") (param $a i32) (param $b i32) (result i32)
                (i32.rem_s (local.get $a) (local.get $b))
            )
        )
        "#;
        let module = Module::parse_wat(wat).unwrap();
        let res = module.invoke("rem", &[10, 0]);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_unknown_export_and_arity_mismatch() {
        let wat = r#"
        (module
            (func $foo (export "foo") (param $a i32) (result i32)
                (local.get $a)
            )
        )
        "#;
        let module = Module::parse_wat(wat).unwrap();
        assert!(module.invoke("nonexistent", &[1]).is_err());
        assert!(module.invoke("foo", &[]).is_err());
        assert!(module.invoke("foo", &[1, 2]).is_err());
    }

    #[test]
    fn test_undefined_local_and_function_call() {
        let wat_local = r#"
        (module
            (func $foo (export "foo") (result i32)
                (local.get $missing)
            )
        )
        "#;
        let module_local = Module::parse_wat(wat_local).unwrap();
        assert!(module_local.invoke("foo", &[]).is_err());

        let wat_call = r#"
        (module
            (func $foo (export "foo") (result i32)
                (call $missing (i32.const 1))
            )
        )
        "#;
        let module_call = Module::parse_wat(wat_call).unwrap();
        assert!(module_call.invoke("foo", &[]).is_err());
    }
}
