// qed's tree-walking interpreter
use std::collections::HashMap;
use std::fmt;
use std::io::Write;

use crate::parser::{BigOp, BinOp, Bound, Branch, Cmp, Def, Expr, MetaHead, MetaStmt, Program};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nat(u64),
    Bool(bool),
    Str(String),
    Nats(Vec<u64>),
    Tuple(Vec<Value>),
    Set(Vec<Value>),
    Func(String),
    Composed(Box<Value>, Box<Value>),
    Id,
}

fn join(f: &mut fmt::Formatter<'_>, vs: &[Value]) -> fmt::Result {
    let mut first = true;
    for v in vs {
        if !first {
            write!(f, ", ")?;
        }
        first = false;
        write!(f, "{v}")?;
    }
    Ok(())
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nat(n) => write!(f, "{n}"),
            Value::Bool(true) => write!(f, "⊤"),
            Value::Bool(false) => write!(f, "⊥"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Nats(ns) => {
                write!(f, "[")?;
                let mut first = true;
                for n in ns {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{n}")?;
                }
                write!(f, "]")
            }
            Value::Tuple(vs) => {
                write!(f, "(")?;
                join(f, vs)?;
                write!(f, ")")
            }
            Value::Set(vs) => {
                write!(f, "{{")?;
                join(f, vs)?;
                write!(f, "}}")
            }
            Value::Func(name) => write!(f, "{name}"),
            Value::Composed(a, b) => write!(f, "({a} ∘ {b})"),
            Value::Id => write!(f, "id"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    Unbound {
        name: String,
    },
    Arity {
        name: String,
        want: usize,
        got: usize,
    },
    DivByZero,
    NotNatural,
    NotAFunction {
        got: String,
    },
    Unsimplifiable {
        what: String,
    },
    NoTrueGuard,
    WantBool {
        got: String,
    },
    NoSuchChar {
        n: u64,
    },
    IndexOutOfBounds {
        idx: u64,
        len: usize,
    },
    OutputClosed,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Unbound { name } => write!(f, "∄ binding for '{name}'"),
            RuntimeError::Arity { name, want, got } => {
                write!(f, "⊬ '{name}' wants {want}, given {got}")
            }
            RuntimeError::DivByZero => write!(f, "⊥: division by zero"),
            RuntimeError::NotNatural => write!(f, "∉ ℕ"),
            RuntimeError::NotAFunction { got } => write!(f, "⊬ {got} is not a definition"),
            RuntimeError::Unsimplifiable { what } => write!(f, "? unable to simplify {what}"),
            RuntimeError::NoTrueGuard => write!(f, "∄ true guard"),
            RuntimeError::WantBool { got } => write!(f, "⊬ expected truth value, got {got}"),
            RuntimeError::NoSuchChar { n } => write!(f, "∄ character {n}"),
            RuntimeError::IndexOutOfBounds { idx, len } => {
                write!(f, "∄ element {idx} of length {len}")
            }
            RuntimeError::OutputClosed => write!(f, "⊬ output closed"),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Debug, Clone, PartialEq)]
enum Pat {
    Any(String),
    Lit(u64),
    Offset(String, i128),
}

struct Env {
    scopes: Vec<HashMap<String, Value>>,
    funcs: HashMap<String, Vec<(Vec<Pat>, Expr)>>,
}

impl Env {
    fn new() -> Self {
        Env {
            scopes: vec![HashMap::new()],
            funcs: HashMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn set(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    fn has_var(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    fn has_func(&self, name: &str) -> bool {
        self.funcs.contains_key(name)
    }

    fn defun(&mut self, name: String, pats: Vec<Pat>, body: Expr) {
        self.funcs.entry(name).or_default().push((pats, body));
    }

    fn clauses(&self, name: &str) -> Option<Vec<(Vec<Pat>, Expr)>> {
        self.funcs.get(name).cloned()
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
}

pub struct Interpreter {
    env: Env,
    stdin_text: String,
    stdin_nats: Vec<u64>,
    source: String,
}

fn as_nat(value: Value) -> Result<u64, RuntimeError> {
    match value {
        Value::Nat(n) => Ok(n),
        _ => Err(RuntimeError::NotNatural),
    }
}

fn as_bool(value: Value) -> Result<bool, RuntimeError> {
    match value {
        Value::Bool(b) => Ok(b),
        v => Err(RuntimeError::WantBool {
            got: format!("{v:?}"),
        }),
    }
}

fn range_vals(lo: u64, hi: u64) -> Vec<Value> {
    if lo > hi {
        Vec::new()
    } else {
        (lo..=hi).map(Value::Nat).collect()
    }
}

fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let (mut lo, mut hi) = (1u64, n);
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        match mid.checked_mul(mid) {
            Some(sq) if sq == n => return mid,
            Some(sq) if sq < n => lo = mid + 1,
            _ => hi = mid - 1,
        }
    }
    hi
}

fn factorial(n: u64) -> Result<u64, RuntimeError> {
    (1..=n)
        .try_fold(1u64, |acc, k| acc.checked_mul(k))
        .ok_or(RuntimeError::NotNatural)
}

fn ceil_sqrt(n: u64) -> u64 {
    let r = isqrt(n);
    if r * r == n { r } else { r + 1 }
}

fn match_pat(pat: &Pat, value: &Value) -> Option<Vec<(String, Value)>> {
    match pat {
        Pat::Any(name) => Some(vec![(name.clone(), value.clone())]),
        Pat::Lit(n) => match value {
            Value::Nat(m) if m == n => Some(Vec::new()),
            _ => None,
        },
        Pat::Offset(name, add) => match value {
            Value::Nat(k) => {
                let n = (*k as i128) - *add;
                if n >= 0 {
                    Some(vec![(name.clone(), Value::Nat(n as u64))])
                } else {
                    None
                }
            }
            _ => None,
        },
    }
}

fn arg_pat(arg: &Expr) -> Option<Pat> {
    match arg {
        Expr::Var(name) => Some(Pat::Any(name.clone())),
        Expr::Num(n) => Some(Pat::Lit(*n)),
        _ => None,
    }
}

fn sub_pat(sub: &Expr) -> Option<Pat> {
    match sub {
        Expr::Var(name) => Some(Pat::Any(name.clone())),
        Expr::Num(n) => Some(Pat::Lit(*n)),
        Expr::Bin(BinOp::Add, l, r) => match (&**l, &**r) {
            (Expr::Var(name), Expr::Num(c)) | (Expr::Num(c), Expr::Var(name)) => {
                Some(Pat::Offset(name.clone(), *c as i128))
            }
            _ => None,
        },
        Expr::Bin(BinOp::Sub, l, r) => match (&**l, &**r) {
            (Expr::Var(name), Expr::Num(c)) => Some(Pat::Offset(name.clone(), -(*c as i128))),
            _ => None,
        },
        _ => None,
    }
}

fn bigop_err() -> RuntimeError {
    RuntimeError::Unsimplifiable {
        what: "big operator".into(),
    }
}

enum Items {
    Finite(std::vec::IntoIter<Value>),
    Count(u64),
}

impl Iterator for Items {
    type Item = Result<Value, RuntimeError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Items::Finite(it) => it.next().map(Ok),
            Items::Count(n) => {
                let cur = *n;
                match cur.checked_add(1) {
                    Some(next) => {
                        *n = next;
                        Some(Ok(Value::Nat(cur)))
                    }
                    None => Some(Err(RuntimeError::NotNatural)),
                }
            }
        }
    }
}

impl Interpreter {
    pub fn run_streaming(
        program: &Program,
        stdin: &str,
        source: &str,
        out: &mut dyn Write,
    ) -> Result<(), RuntimeError> {
        let mut it = Interpreter {
            env: Env::new(),
            stdin_text: stdin.to_string(),
            stdin_nats: stdin
                .split_whitespace()
                .filter_map(|w| w.parse::<u64>().ok())
                .collect(),
            source: source.to_string(),
        };
        for def in &program.defs {
            it.define(def)?;
        }
        for meta in &program.metas {
            it.meta(meta)?;
        }
        it.emit(&program.goal, out)
    }

    fn put(out: &mut dyn Write, bytes: &[u8]) -> Result<(), RuntimeError> {
        out.write_all(bytes)
            .and_then(|()| out.flush())
            .map_err(|_| RuntimeError::OutputClosed)
    }

    fn emit(&mut self, expr: &Expr, out: &mut dyn Write) -> Result<(), RuntimeError> {
        match expr {
            Expr::Big {
                op: BigOp::Concat,
                var,
                bound,
                hi,
                body,
            } => {
                for item in self.big_items(bound.as_ref(), hi.as_deref())? {
                    match self.big_body(var, body, item?)? {
                        Value::Str(s) => Self::put(out, s.as_bytes())?,
                        _ => return Err(bigop_err()),
                    }
                }
                Ok(())
            }
            Expr::Cases(branches) => {
                let i = self.pick_branch(branches)?;
                self.emit(&branches[i].value, out)
            }
            _ => {
                let text = self.eval(expr)?.to_string();
                Self::put(out, text.as_bytes())?;
                if !text.ends_with('\n') {
                    Self::put(out, b"\n")?;
                }
                Ok(())
            }
        }
    }

    fn define(&mut self, def: &Def) -> Result<(), RuntimeError> {
        let bad = || RuntimeError::Unsimplifiable {
            what: format!("definition '{}'", def.name),
        };
        match (&def.sub, def.args.as_slice()) {
            (None, []) => {
                let value = self.eval(&def.body)?;
                self.env.set(def.name.clone(), value);
                Ok(())
            }
            (None, args) => {
                let mut pats = Vec::with_capacity(args.len());
                for arg in args {
                    pats.push(arg_pat(arg).ok_or_else(&bad)?);
                }
                self.env.defun(def.name.clone(), pats, def.body.clone());
                Ok(())
            }
            (Some(sub), []) => {
                let pat = sub_pat(sub).ok_or_else(&bad)?;
                self.env
                    .defun(def.name.clone(), vec![pat], def.body.clone());
                Ok(())
            }
            _ => Err(bad()),
        }
    }

    fn meta(&mut self, stmt: &MetaStmt) -> Result<(), RuntimeError> {
        match &stmt.head {
            Some(MetaHead::Models) => {
                let body = match &stmt.body {
                    Some(b) => b,
                    None => {
                        return Err(RuntimeError::Unsimplifiable { what: "⊨".into() });
                    }
                };
                if !as_bool(self.eval(body)?)? {
                    return Err(RuntimeError::WantBool {
                        got: "false".into(),
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Num(n) => Ok(Value::Nat(*n)),
            Expr::Var(name) => match self.env.get(name) {
                Some(v) => Ok(v),
                None if self.env.has_func(name) => Ok(Value::Func(name.clone())),
                None => Err(RuntimeError::Unbound { name: name.clone() }),
            },
            Expr::Bin(op, lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                Self::arithmetic(op, lhs, rhs)
            }
            Expr::Cmp(op, lhs, rhs) => {
                let lhs = self.eval(lhs)?;
                let rhs = self.eval(rhs)?;
                Ok(Value::Bool(Self::compare(op, lhs, rhs)?))
            }
            Expr::Congruent { a, b, n } => {
                let (x, y, m) = (
                    as_nat(self.eval(a)?)?,
                    as_nat(self.eval(b)?)?,
                    as_nat(self.eval(n)?)?,
                );
                if m == 0 {
                    return Err(RuntimeError::DivByZero);
                }
                Ok(Value::Bool(x % m == y % m))
            }
            Expr::Call(fun, args) => self.apply_call(fun, args),
            Expr::Index(base, idx) => self.index(base, idx),
            Expr::Not(x) => Ok(Value::Bool(!as_bool(self.eval(x)?)?)),
            Expr::Sqrt(x) => Ok(Value::Nat(isqrt(as_nat(self.eval(x)?)?))),
            Expr::Floor(x) => Ok(Value::Nat(as_nat(self.eval(x)?)?)),
            Expr::Ceil(x) => match &**x {
                Expr::Sqrt(n) => Ok(Value::Nat(ceil_sqrt(as_nat(self.eval(n)?)?))),
                Expr::Bin(BinOp::Div, a, b) => {
                    let (x, y) = (as_nat(self.eval(a)?)?, as_nat(self.eval(b)?)?);
                    if y == 0 {
                        return Err(RuntimeError::DivByZero);
                    }
                    Ok(Value::Nat(x / y + if x % y == 0 { 0 } else { 1 }))
                }
                _ => Ok(Value::Nat(as_nat(self.eval(x)?)?)),
            },
            Expr::Fact(x) => Ok(Value::Nat(factorial(as_nat(self.eval(x)?)?)?)),
            Expr::Chi(args) => {
                let mut out = String::new();
                for arg in args {
                    match self.eval(arg)? {
                        Value::Nat(n) => out.push(
                            u32::try_from(n)
                                .ok()
                                .and_then(char::from_u32)
                                .ok_or(RuntimeError::NoSuchChar { n })?,
                        ),
                        Value::Str(s) => out.push_str(&s),
                        _ => return Err(RuntimeError::NotNatural),
                    }
                }
                Ok(Value::Str(out))
            }
            Expr::Dec(x) => Ok(Value::Str(as_nat(self.eval(x)?)?.to_string())),
            Expr::Len(x) => match self.eval(x)? {
                Value::Str(s) => Ok(Value::Nat(s.chars().count() as u64)),
                Value::Nats(ns) => Ok(Value::Nat(ns.len() as u64)),
                Value::Tuple(items) | Value::Set(items) => Ok(Value::Nat(items.len() as u64)),
                _ => Err(RuntimeError::NotNatural),
            },
            Expr::Eps => Ok(Value::Str(String::new())),
            Expr::True => Ok(Value::Bool(true)),
            Expr::False => Ok(Value::Bool(false)),
            Expr::Infinity => Err(RuntimeError::Unsimplifiable { what: "∞".into() }),
            Expr::Ellipsis => Err(RuntimeError::Unsimplifiable { what: "…".into() }),
            Expr::InputChars => Ok(Value::Str(self.stdin_text.clone())),
            Expr::InputNums => Ok(Value::Nats(self.stdin_nats.clone())),
            Expr::Source => Ok(Value::Str(self.source.clone())),
            Expr::Range(a, b) => {
                let (x, y) = (as_nat(self.eval(a)?)?, as_nat(self.eval(b)?)?);
                Ok(Value::Set(range_vals(x, y)))
            }
            Expr::Tuple(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.eval(item)?);
                }
                Ok(Value::Tuple(out))
            }
            Expr::Set(items) => Ok(Value::Set(self.set_items(items)?)),
            Expr::Cases(branches) => self.cases(branches),
            Expr::Big {
                op,
                var,
                bound,
                hi,
                body,
            } => self.big(op, var, bound.as_ref(), hi.as_deref(), body),
            Expr::Iterate { func, count } => {
                let f = self.eval(func)?;
                match f {
                    Value::Func(_) | Value::Composed(_, _) | Value::Id => {}
                    _ => {
                        return Err(RuntimeError::NotAFunction { got: f.to_string() });
                    }
                }
                let mut acc = Value::Id;
                for _ in 0..as_nat(self.eval(count)?)? {
                    acc = Value::Composed(Box::new(f.clone()), Box::new(acc));
                }
                Ok(acc)
            }
            Expr::Forall { var, set, body } => self.forall(var, set, body),
            Expr::Exists { var, set, body } => self.exists(var, set, body),
            Expr::Mu { var, body } => self.mu(var, body),
        }
    }

    fn arithmetic(op: &BinOp, lhs: Value, rhs: Value) -> Result<Value, RuntimeError> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
                let (x, y) = (as_nat(lhs)?, as_nat(rhs)?);
                match op {
                    BinOp::Add => x.checked_add(y),
                    BinOp::Sub => x.checked_sub(y),
                    BinOp::Mul => x.checked_mul(y),
                    BinOp::Div if y == 0 => return Err(RuntimeError::DivByZero),
                    BinOp::Div => x.checked_div(y),
                    BinOp::Pow => match x {
                        0 => Some(if y == 0 { 1 } else { 0 }),
                        1 => Some(1),
                        _ => y.try_into().ok().and_then(|e| x.checked_pow(e)),
                    },
                    _ => return Err(bigop_err()),
                }
                .map(Value::Nat)
                .ok_or(RuntimeError::NotNatural)
            }
            BinOp::Juxt => match (lhs, rhs) {
                (Value::Nat(x), Value::Nat(y)) => x
                    .checked_mul(y)
                    .map(Value::Nat)
                    .ok_or(RuntimeError::NotNatural),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
                _ => Err(RuntimeError::Unsimplifiable {
                    what: "juxtaposition".into(),
                }),
            },
            BinOp::Concat => match (lhs, rhs) {
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
                _ => Err(RuntimeError::Unsimplifiable {
                    what: "concatenation".into(),
                }),
            },
            BinOp::And => match (lhs, rhs) {
                (Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(x && y)),
                (Value::Bool(_), other) => Err(RuntimeError::WantBool {
                    got: format!("{other:?}"),
                }),
                (other, _) => Err(RuntimeError::WantBool {
                    got: format!("{other:?}"),
                }),
            },
            BinOp::Union => match (lhs, rhs) {
                (Value::Set(mut a), Value::Set(b)) => {
                    for x in b {
                        if !a.contains(&x) {
                            a.push(x);
                        }
                    }
                    Ok(Value::Set(a))
                }
                _ => Err(RuntimeError::Unsimplifiable {
                    what: "union".into(),
                }),
            },
            BinOp::Intersect => match (lhs, rhs) {
                (Value::Set(a), Value::Set(b)) => Ok(Value::Set(
                    a.into_iter().filter(|x| b.contains(x)).collect(),
                )),
                _ => Err(RuntimeError::Unsimplifiable {
                    what: "intersection".into(),
                }),
            },
            BinOp::SetMinus => match (lhs, rhs) {
                (Value::Set(a), Value::Set(b)) => Ok(Value::Set(
                    a.into_iter().filter(|x| !b.contains(x)).collect(),
                )),
                _ => Err(RuntimeError::Unsimplifiable {
                    what: "difference".into(),
                }),
            },
            BinOp::Compose => match (&lhs, &rhs) {
                (
                    Value::Func(_) | Value::Composed(_, _) | Value::Id,
                    Value::Func(_) | Value::Composed(_, _) | Value::Id,
                ) => Ok(Value::Composed(Box::new(lhs), Box::new(rhs))),
                _ => Err(RuntimeError::Unsimplifiable {
                    what: "composition".into(),
                }),
            },
            BinOp::Or => match (lhs, rhs) {
                (Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(x || y)),
                (Value::Bool(_), other) => Err(RuntimeError::WantBool {
                    got: format!("{other:?}"),
                }),
                (other, _) => Err(RuntimeError::WantBool {
                    got: format!("{other:?}"),
                }),
            },
        }
    }

    fn compare(op: &Cmp, lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
        match op {
            Cmp::Eq => Ok(lhs == rhs),
            Cmp::Ne => Ok(lhs != rhs),
            Cmp::Lt => Ok(as_nat(lhs)? < as_nat(rhs)?),
            Cmp::Gt => Ok(as_nat(lhs)? > as_nat(rhs)?),
            Cmp::Le => Ok(as_nat(lhs)? <= as_nat(rhs)?),
            Cmp::Ge => Ok(as_nat(lhs)? >= as_nat(rhs)?),
            Cmp::In => match rhs {
                Value::Set(items) | Value::Tuple(items) => Ok(items.contains(&lhs)),
                Value::Nats(ns) => match lhs {
                    Value::Nat(n) => Ok(ns.contains(&n)),
                    _ => Ok(false),
                },
                _ => Err(RuntimeError::Unsimplifiable { what: "∈".into() }),
            },
            Cmp::Subset => match (lhs, rhs) {
                (Value::Set(a), Value::Set(b)) => Ok(a.iter().all(|x| b.contains(x))),
                _ => Err(RuntimeError::Unsimplifiable { what: "⊆".into() }),
            },
            Cmp::Divides => Self::divides(lhs, rhs),
            Cmp::NotDivides => Ok(!Self::divides(lhs, rhs)?),
        }
    }

    fn divides(lhs: Value, rhs: Value) -> Result<bool, RuntimeError> {
        let (x, y) = (as_nat(lhs)?, as_nat(rhs)?);
        if x == 0 {
            return Err(RuntimeError::DivByZero);
        }
        Ok(y % x == 0)
    }

    fn apply_call(&mut self, fun: &Expr, args: &[Expr]) -> Result<Value, RuntimeError> {
        let f = self.eval(fun)?;
        let mut argv = Vec::with_capacity(args.len());
        for arg in args {
            argv.push(self.eval(arg)?);
        }
        self.apply_value(f, argv)
    }

    fn apply_value(&mut self, fun: Value, argv: Vec<Value>) -> Result<Value, RuntimeError> {
        match fun {
            Value::Func(name) => self.apply(&name, argv),
            Value::Composed(_, _) => {
                let mut chain = vec![fun];
                let mut order = Vec::new();
                while let Some(v) = chain.pop() {
                    match v {
                        Value::Composed(a, b) => {
                            chain.push(*a);
                            chain.push(*b);
                        }
                        other => order.push(other),
                    }
                }
                let mut acc = argv;
                for f in order {
                    acc = vec![self.apply_value(f, acc)?];
                }
                acc.into_iter().next().ok_or(RuntimeError::Unsimplifiable {
                    what: "composition".into(),
                })
            }
            Value::Id => match argv.as_slice() {
                [x] => Ok(x.clone()),
                _ => Err(RuntimeError::Arity {
                    name: "id".into(),
                    want: 1,
                    got: argv.len(),
                }),
            },
            other => Err(RuntimeError::NotAFunction {
                got: other.to_string(),
            }),
        }
    }

    fn apply(&mut self, name: &str, argv: Vec<Value>) -> Result<Value, RuntimeError> {
        let clauses = self.env.clauses(name).ok_or(RuntimeError::Unbound {
            name: name.to_string(),
        })?;
        for (pats, body) in &clauses {
            if pats.len() != argv.len() {
                continue;
            }
            let mut bindings = Vec::new();
            let mut matched = true;
            for (pat, val) in pats.iter().zip(argv.iter()) {
                match match_pat(pat, val) {
                    Some(mut bound) => bindings.append(&mut bound),
                    None => {
                        matched = false;
                        break;
                    }
                }
            }
            if !matched {
                continue;
            }
            self.env.push_scope();
            for (param, value) in bindings {
                self.env.set(param, value);
            }
            let out = self.eval(body);
            self.env.pop_scope();
            return out;
        }
        if clauses.iter().any(|(pats, _)| pats.len() == argv.len()) {
            Err(RuntimeError::NoTrueGuard)
        } else {
            Err(RuntimeError::Arity {
                name: name.to_string(),
                want: clauses.first().map(|(pats, _)| pats.len()).unwrap_or(0),
                got: argv.len(),
            })
        }
    }

    fn index(&mut self, base: &Expr, idx: &Expr) -> Result<Value, RuntimeError> {
        let k = as_nat(self.eval(idx)?)?;
        if let Expr::Var(name) = base {
            if !self.env.has_var(name) && self.env.has_func(name) {
                return self.apply(name, vec![Value::Nat(k)]);
            }
        }
        let oob = |len: usize| RuntimeError::IndexOutOfBounds { idx: k, len };
        match self.eval(base)? {
            Value::Str(s) => {
                let len = s.chars().count();
                if k == 0 || k > len as u64 {
                    return Err(oob(len));
                }
                s.chars()
                    .nth((k - 1) as usize)
                    .map(|c| Value::Str(c.to_string()))
                    .ok_or(oob(len))
            }
            Value::Nats(ns) => {
                let len = ns.len();
                if k == 0 || k > len as u64 {
                    return Err(oob(len));
                }
                Ok(Value::Nat(ns[(k - 1) as usize]))
            }
            Value::Tuple(items) => {
                let len = items.len();
                if k == 0 || k > len as u64 {
                    return Err(oob(len));
                }
                Ok(items[(k - 1) as usize].clone())
            }
            _ => Err(RuntimeError::Unsimplifiable {
                what: "index".into(),
            }),
        }
    }

    fn pick_branch(&mut self, branches: &[Branch]) -> Result<usize, RuntimeError> {
        for (i, branch) in branches.iter().enumerate() {
            let take = match &branch.guard {
                None => true,
                Some(guard) => as_bool(self.eval(guard)?)?,
            };
            if take {
                return Ok(i);
            }
        }
        Err(RuntimeError::NoTrueGuard)
    }

    fn cases(&mut self, branches: &[Branch]) -> Result<Value, RuntimeError> {
        let i = self.pick_branch(branches)?;
        self.eval(&branches[i].value)
    }

    fn big_items(
        &mut self,
        bound: Option<&Bound>,
        hi: Option<&Expr>,
    ) -> Result<Items, RuntimeError> {
        match bound {
            None => Err(bigop_err()),
            Some(Bound::Eq(e)) => match self.eval(e)? {
                Value::Nat(lo) => {
                    let h = hi.ok_or_else(bigop_err)?;
                    if matches!(h, Expr::Infinity) {
                        Ok(Items::Count(lo))
                    } else {
                        Ok(Items::Finite(
                            range_vals(lo, as_nat(self.eval(h)?)?).into_iter(),
                        ))
                    }
                }
                Value::Set(elts) => {
                    if hi.is_some() {
                        return Err(bigop_err());
                    }
                    Ok(Items::Finite(elts.into_iter()))
                }
                _ => Err(RuntimeError::NotNatural),
            },
            Some(Bound::In(e)) => match self.eval(e)? {
                Value::Set(elts) | Value::Tuple(elts) => Ok(Items::Finite(elts.into_iter())),
                Value::Nats(ns) => Ok(Items::Finite(
                    ns.into_iter()
                        .map(Value::Nat)
                        .collect::<Vec<_>>()
                        .into_iter(),
                )),
                _ => Err(bigop_err()),
            },
        }
    }

    fn big_body(&mut self, var: &str, body: &Expr, item: Value) -> Result<Value, RuntimeError> {
        self.env.push_scope();
        self.env.set(var.to_string(), item);
        let got = self.eval(body);
        self.env.pop_scope();
        got
    }

    fn big(
        &mut self,
        op: &BigOp,
        var: &str,
        bound: Option<&Bound>,
        hi: Option<&Expr>,
        body: &Expr,
    ) -> Result<Value, RuntimeError> {
        let items = self.big_items(bound, hi)?;
        match op {
            BigOp::Sum => {
                let mut total = 0u64;
                for item in items {
                    total = total
                        .checked_add(as_nat(self.big_body(var, body, item?)?)?)
                        .ok_or(RuntimeError::NotNatural)?;
                }
                Ok(Value::Nat(total))
            }
            BigOp::Prod => {
                let mut total = 1u64;
                for item in items {
                    total = total
                        .checked_mul(as_nat(self.big_body(var, body, item?)?)?)
                        .ok_or(RuntimeError::NotNatural)?;
                }
                Ok(Value::Nat(total))
            }
            BigOp::Concat => {
                let mut out = String::new();
                for item in items {
                    match self.big_body(var, body, item?)? {
                        Value::Str(s) => out.push_str(&s),
                        _ => return Err(bigop_err()),
                    }
                }
                Ok(Value::Str(out))
            }
        }
    }

    fn forall(&mut self, var: &str, set: &Expr, body: &Expr) -> Result<Value, RuntimeError> {
        let items = match self.eval(set)? {
            Value::Set(items) | Value::Tuple(items) => items,
            Value::Nats(ns) => ns.into_iter().map(Value::Nat).collect(),
            _ => {
                return Err(RuntimeError::Unsimplifiable { what: "∀".into() });
            }
        };
        for item in items {
            self.env.push_scope();
            self.env.set(var.to_string(), item);
            let got = self.eval(body);
            self.env.pop_scope();
            if !as_bool(got?)? {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    fn exists(&mut self, var: &str, set: &Expr, body: &Expr) -> Result<Value, RuntimeError> {
        let items = match self.eval(set)? {
            Value::Set(items) | Value::Tuple(items) => items,
            Value::Nats(ns) => ns.into_iter().map(Value::Nat).collect(),
            _ => {
                return Err(RuntimeError::Unsimplifiable { what: "∃".into() });
            }
        };
        for item in items {
            self.env.push_scope();
            self.env.set(var.to_string(), item);
            let got = self.eval(body);
            self.env.pop_scope();
            if as_bool(got?)? {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    fn mu(&mut self, var: &str, body: &Expr) -> Result<Value, RuntimeError> {
        let mut n = 0u64;
        loop {
            self.env.push_scope();
            self.env.set(var.to_string(), Value::Nat(n));
            let got = self.eval(body);
            self.env.pop_scope();
            if as_bool(got?)? {
                return Ok(Value::Nat(n));
            }
            n = n.checked_add(1).ok_or(RuntimeError::NotNatural)?;
        }
    }

    fn set_items(&mut self, items: &[Expr]) -> Result<Vec<Value>, RuntimeError> {
        if items.len() == 3 && matches!(items[1], Expr::Ellipsis) {
            let x = as_nat(self.eval(&items[0])?)?;
            let y = as_nat(self.eval(&items[2])?)?;
            return Ok(range_vals(x, y));
        }
        if items.iter().any(|item| matches!(item, Expr::Ellipsis)) {
            return Err(RuntimeError::Unsimplifiable { what: "…".into() });
        }
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(self.eval(item)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check(src: &str, stdin: &str) -> Result<String, String> {
        let toks = Lexer::new(src).lex().map_err(|e| e.to_string())?;
        let prog = Parser::new(toks).parse().map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        Interpreter::run_streaming(&prog, stdin, src, &mut buf).map_err(|e| e.to_string())?;
        String::from_utf8(buf).map_err(|e| e.to_string())
    }

    fn run(src: &str) -> String {
        check(src, "")
            .unwrap_or_else(|e| panic!("{src}\n{e}"))
            .to_string()
    }

    fn run_in(src: &str, stdin: &str) -> String {
        check(src, stdin)
            .unwrap_or_else(|e| panic!("{src}\n{e}"))
            .to_string()
    }

    fn fails(src: &str) -> String {
        check(src, "")
            .err()
            .unwrap_or_else(|| panic!("should fail:\n{src}"))
    }

    #[test]
    fn arithmetic() {
        assert_eq!(run("∴ 2 + 3 * 4 ∎"), "14\n");
    }

    #[test]
    fn function() {
        assert_eq!(run("f(x) := x² + 1\n∴ f(3) ∎"), "10\n");
    }

    #[test]
    fn recursive_function() {
        assert_eq!(
            run("F(0) := 0\nF(1) := 1\nF(n) := F(n-1) + F(n-2)\n∴ F(10) ∎"),
            "55\n"
        );
    }

    #[test]
    fn booleans() {
        assert_eq!(run("∴ 6 < 10 ∎"), "⊤\n");
        assert_eq!(run("∴ ¬(6 < 10) ∎"), "⊥\n");
    }

    #[test]
    fn cases() {
        assert_eq!(run("f(n) := { 0 | n = 0\n1 | n ≠ 0 }\n∴ f(42) ∎"), "1\n");
        assert!(fails("∴ { 1 | 1 = 0 } ∎").contains("true guard"));
    }

    #[test]
    fn juxtaposition() {
        assert_eq!(run("x := 10\n∴ 3x + 1 ∎"), "31\n");
        assert_eq!(run("∴ χ(72)χ(105) ∎"), "Hi\n");
    }

    #[test]
    fn factorial_sqrt_floor() {
        assert_eq!(run("∴ 5! ∎"), "120\n");
        assert_eq!(run("∴ 0! ∎"), "1\n");
        assert_eq!(run("∴ √81 ∎"), "9\n");
        assert_eq!(run("∴ √2 ∎"), "1\n");
        assert_eq!(run("∴ ⌊√100⌋ ∎"), "10\n");
        assert_eq!(run("∴ floor(sqrt(50)) ∎"), "7\n");
        assert_eq!(
            run("F(n) := { 1 | n = 0\nnF(n-1) | n > 0 }\n∴ F(5) ∎"),
            "120\n"
        );
    }

    #[test]
    fn strings() {
        assert_eq!(run("∴ χ(72,101,108,108,111) ∎"), "Hello\n");
        assert_eq!(run("∴ δ(123) ∎"), "123\n");
        assert_eq!(run("∴ χ(72,101) ⊕ χ(108,108,111) ∎"), "Hello\n");
        assert_eq!(run("∴ χ(χ(72)) ∎"), "H\n");
    }

    #[test]
    fn index_is_one_based() {
        assert_eq!(run("∴ χ(65,66,67)_2 ∎"), "B\n");
        assert!(fails("∴ χ(65)_0 ∎").contains('∄'));
        assert!(fails("∴ χ(65)_5 ∎").contains('∄'));
    }

    #[test]
    fn big_operators() {
        assert_eq!(run("∴ ∑ᵢ₌₁¹⁰ i ∎"), "55\n");
        assert_eq!(run("∴ ⨁ᵢ₌₁¹⁰ δ(i) ∎"), "12345678910");
    }

    #[test]
    fn fibonacci_readme() {
        assert_eq!(
            run("F₀ := 0\nF₁ := 1\nF_{n+2} := F_{n+1} + F_n\n∴ ⨁ᵢ₌₀¹⁰ (δ(Fᵢ) ⊕ χ(10)) ∎"),
            "0\n1\n1\n2\n3\n5\n8\n13\n21\n34\n55\n"
        );
    }

    #[test]
    fn hello_cruel_world() {
        assert_eq!(
            run(
                "w := χ(72,101,108,108,111,44,32,99,114,117,101,108,32,119,111,114,108,100,46,46,46,10)\n∴ ⨁ᵢ₌₁²² χ(wᵢ) ∎"
            ),
            "Hello, cruel world...\n"
        );
    }

    #[test]
    fn cat_copies_stdin() {
        assert_eq!(run_in("∴ 𝒞 ∎", "hi\n"), "hi\n");
    }

    #[test]
    fn sum_reads_numbers() {
        assert_eq!(run_in("∴ ∑_{x∈𝒩} x ∎", "1 2 3 4"), "10\n");
    }

    #[test]
    fn reverse_readme() {
        assert_eq!(run_in("∴ ⨁ᵢ₌₁^{∣𝒞∣} 𝒞_{∣𝒞∣-i+1} ∎", "abc"), "cba");
    }

    #[test]
    fn truth_machine() {
        assert_eq!(
            run_in("∴ { χ(48) | N₁ = 0\n⨁ₙ₌₁∞ χ(49) | N₁ = 1 } ∎", "0"),
            "0\n"
        );
    }

    struct Cap {
        buf: Vec<u8>,
        cap: usize,
    }

    impl Write for Cap {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.buf.len() >= self.cap {
                return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "full"));
            }
            let room = self.cap - self.buf.len();
            let n = room.min(buf.len());
            self.buf.extend_from_slice(&buf[..n]);
            Ok(n)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn truth_machine_streams_ones_forever() {
        let toks = Lexer::new("∴ { χ(48) | N₁ = 0\n⨁ₙ₌₁∞ χ(49) | N₁ = 1 } ∎")
            .lex()
            .expect("lex");
        let prog = Parser::new(toks).parse().expect("parse");
        let mut cap = Cap {
            buf: Vec::new(),
            cap: 5,
        };
        let err = Interpreter::run_streaming(&prog, "1", "", &mut cap)
            .err()
            .expect("should stop");
        assert!(err.to_string().contains("closed"));
        assert_eq!(cap.buf, b"11111");
    }

    #[test]
    fn fizzbuzz_readme() {
        let out = run(
            "f := χ(70)χ(105)χ(122)χ(122)\nb := χ(66)χ(117)χ(122)χ(122)\ng(n) := { fb | 15 ∣ n\nf | 3 ∣ n\nb | 5 ∣ n\nδ(n) | ⊤ }\n∴ ⨁ₙ₌₁¹⁰⁰ (g(n) ⊕ χ(10)) ∎",
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 100);
        assert_eq!(lines[0], "1");
        assert_eq!(lines[2], "Fizz");
        assert_eq!(lines[4], "Buzz");
        assert_eq!(lines[14], "FizzBuzz");
        assert_eq!(lines[99], "Buzz");
    }

    #[test]
    fn primes_readme() {
        assert_eq!(
            run(
                "P(n) := { 1 | n ≥ 2 ∧ ∀d ∈ {2,…,⌊√n⌋} : d ∤ n\n0 | ⊤ }\n∴ ⨁ₙ₌₁¹⁰⁰ { δ(n) ⊕ χ(10) | P(n) = 1\nε | ⊤ } ∎"
            ),
            "2\n3\n5\n7\n11\n13\n17\n19\n23\n29\n31\n37\n41\n43\n47\n53\n59\n61\n67\n71\n73\n79\n83\n89\n97\n"
        );
    }

    #[test]
    fn mu_search() {
        assert_eq!(run("∴ μx. x² > 100 ∎"), "11\n");
    }

    #[test]
    fn quine() {
        assert_eq!(run("∴ 𝒮 ∎"), "∴ 𝒮 ∎\n");
    }

    #[test]
    fn product() {
        assert_eq!(run("∴ ∏ᵢ₌₁⁵ i ∎"), "120\n");
        assert_eq!(run("|- prod_{i=1}^{5} i ."), "120\n");
    }

    #[test]
    fn exists() {
        assert_eq!(run("∴ ∃d ∈ {2,3} : d ∣ 7 ∎"), "⊥\n");
        assert_eq!(run("∴ ∃d ∈ {2,3} : d ∣ 8 ∎"), "⊤\n");
        assert_eq!(run("|- exists d in {2,3} : d divides 8 ."), "⊤\n");
    }

    #[test]
    fn ceiling() {
        assert_eq!(run("∴ ⌈√50⌉ ∎"), "8\n");
        assert_eq!(run("∴ ⌈√81⌉ ∎"), "9\n");
        assert_eq!(run("∴ ceil(sqrt(50)) ∎"), "8\n");
    }

    #[test]
    fn congruence() {
        assert_eq!(run("∴ 10 ≡ 3 mod 7 ∎"), "⊤\n");
        assert_eq!(run("∴ 10 ≡ 4 mod 7 ∎"), "⊥\n");
        assert_eq!(run("∴ equiv(10, 3, 7) ∎"), "⊤\n");
        assert!(fails("∴ 1 ≡ 2 mod 0 ∎").contains('⊥'));
    }

    #[test]
    fn set_algebra() {
        assert_eq!(run("∴ {1,2} ∪ {2,3} ∎"), "{1, 2, 3}\n");
        assert_eq!(run("∴ {1,2} ∩ {2,3} ∎"), "{2}\n");
        assert_eq!(run("∴ {1,2,3} \\ {2} ∎"), "{1, 3}\n");
        assert_eq!(run("∴ {1,2} ⊆ {1,2,3} ∎"), "⊤\n");
        assert_eq!(run("∴ {1,2} ⊆ {3} ∎"), "⊥\n");
        assert_eq!(run("|- {1,2} union {2,3} ."), "{1, 2, 3}\n");
        assert_eq!(run("|- {1,2} inter {2,3} ."), "{2}\n");
        assert_eq!(run("|- {1,2} subset {1,2,3} ."), "⊤\n");
    }

    #[test]
    fn composition() {
        assert_eq!(run("f(x) := x + 1\ng(x) := x * 2\n∴ (f∘g)(3) ∎"), "7\n");
        assert_eq!(
            run("f(x) := x + 1\ng(x) := x * 2\n|- (f compose g)(3) ."),
            "7\n"
        );
    }

    #[test]
    fn functions_are_values() {
        assert_eq!(run("f(x) := x + 1\n∴ f ∎"), "f\n");
        assert!(fails("T(n) := n\n∴ T^2 ∎").contains('ℕ'));
    }

    #[test]
    fn iteration() {
        assert_eq!(run("D(n) := n + 1\n∴ D^{∘3}(0) ∎"), "3\n");
        assert_eq!(run("D(n) := n + 1\n|- D^{compose 3}(0) ."), "3\n");
    }

    #[test]
    fn collatz_composed() {
        let out = run(
            "T(n) := { n/2 | 2 ∣ n\n3n+1 | 2 ∤ n }\nτ(n) := μk. T^{∘k}(n) = 1\n∴ ⨁ᵢ₌₀^{τ(27)} (δ(T^{∘i}(27)) ⊕ χ(10)) ∎",
        );
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 112);
        assert_eq!(lines[0], "27");
        assert_eq!(lines[111], "1");
    }

    #[test]
    fn runtime_errors() {
        assert!(fails("∴ 1/0 ∎").contains('⊥'));
        assert!(fails("∴ y ∎").contains('∄'));
        assert!(fails("∴ 2^64 ∎").contains('ℕ'));
        assert!(fails("f(x) := x\n∴ f(1, 2) ∎").contains('⊬'));
        assert!(fails("∴ χ(2000000) ∎").contains("character"));
        assert!(fails("∴ 1 ∧ 2 ∎").contains("truth value"));
    }

    #[test]
    fn meta_prose_runs() {
        assert_eq!(run("x := 1\nℳ{ anything ∴ goes }\n∴ x ∎"), "1\n");
        assert_eq!(run("meta{ ascii prose }\n∴ 2 ∎"), "2\n");
        assert_eq!(run("∴ 1 ℳ{one} + 2 ∎"), "3\n");
    }

    #[test]
    fn models_asserts() {
        assert_eq!(run("F(x) := x\nℳ ⊨ F(5) = 5\n∴ F(1) ∎"), "1\n");
        assert_eq!(run("ℳ ⊨ F(5) = 5\nF(x) := x\n∴ F(1) ∎"), "1\n");
        assert_eq!(run("F(x) := x\nmeta |= F(5) = 5\n∴ F(1) ∎"), "1\n");
        assert!(fails("ℳ ⊨ 1 = 2\n∴ 1 ∎").contains("false"));
    }

    #[test]
    fn meta_hints_ignored() {
        assert_eq!(
            run("ℳ ⊢ F(1) = 1\nℳ ↓ F\nℳ ⊥\nℳ ∎\nℳ₀ lazy\nF(x) := x\n∴ F(1) ∎"),
            "1\n"
        );
    }
}
