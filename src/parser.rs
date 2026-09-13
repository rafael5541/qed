// qed's parser. plain recursive descent

use std::fmt;

use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    In,
    Divides,
    NotDivides,
    Subset,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Juxt,
    And,
    Or,
    Union,
    Intersect,
    SetMinus,
    Compose,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BigOp {
    Sum,
    Prod,
    Concat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    Eq(Box<Expr>),
    In(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub value: Expr,
    pub guard: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(u64),
    Var(String),
    Eps,
    True,
    False,
    Infinity,
    Ellipsis,
    InputChars,
    InputNums,
    Source,
    Chi(Vec<Expr>),
    Dec(Box<Expr>),
    Not(Box<Expr>),
    Sqrt(Box<Expr>),
    Floor(Box<Expr>),
    Ceil(Box<Expr>),
    Len(Box<Expr>),
    Fact(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Cmp(Cmp, Box<Expr>, Box<Expr>),
    Congruent {
        a: Box<Expr>,
        b: Box<Expr>,
        n: Box<Expr>,
    },
    Range(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Iterate {
        func: Box<Expr>,
        count: Box<Expr>,
    },
    Big {
        op: BigOp,
        var: String,
        bound: Option<Bound>,
        hi: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Forall {
        var: String,
        set: Box<Expr>,
        body: Box<Expr>,
    },
    Exists {
        var: String,
        set: Box<Expr>,
        body: Box<Expr>,
    },
    Mu {
        var: String,
        body: Box<Expr>,
    },
    Tuple(Vec<Expr>),
    Set(Vec<Expr>),
    Cases(Vec<Branch>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetaHead {
    Turnstile,
    Halts,
    Models,
    Bottom,
    Qed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaStmt {
    pub level: Option<u64>,
    pub head: Option<MetaHead>,
    pub body: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Def {
    pub name: String,
    pub sub: Option<Expr>,
    pub args: Vec<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub defs: Vec<Def>,
    pub metas: Vec<MetaStmt>,
    pub goal: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Unexpected { got: String, want: String },
    UnexpectedEnd { want: String },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Unexpected { got, want } => {
                write!(f, "⊬ unexpected {got}, expected {want}")
            }
            ParseError::UnexpectedEnd { want } => {
                write!(f, "⊬ unexpected end of proof, expected {want}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

fn starts_atom(t: &Token) -> bool {
    matches!(
        t,
        Token::Num(_)
            | Token::Ident(_)
            | Token::LParen
            | Token::Chi
            | Token::Dec
            | Token::Eps
            | Token::InputChars
            | Token::InputNums
            | Token::Source
            | Token::True
            | Token::False
            | Token::Infinity
            | Token::Ellipsis
            | Token::FloorL
            | Token::CeilL
            | Token::Sqrt
            | Token::Sum
            | Token::Prod
            | Token::BigConcat
            | Token::Forall
            | Token::Exists
            | Token::Mu
            | Token::Not
            | Token::FloorFn
            | Token::SqrtFn
            | Token::CeilFn
            | Token::EquivFn
    )
}

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Self { toks, pos: 0 }
    }

    fn peek(&self) -> Token {
        self.toks.get(self.pos).cloned().unwrap_or(Token::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.peek();
        if self.pos < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, want: Token) -> bool {
        if self.peek() == want {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_nl(&mut self) {
        while self.peek() == Token::Newline {
            self.bump();
        }
    }

    fn err(&self, want: &str) -> ParseError {
        match self.peek() {
            Token::Eof => ParseError::UnexpectedEnd { want: want.into() },
            t => ParseError::Unexpected {
                got: format!("{t:?}"),
                want: want.into(),
            },
        }
    }

    fn expect(&mut self, want: Token, name: &str) -> Result<(), ParseError> {
        if self.eat(want) {
            Ok(())
        } else {
            Err(self.err(name))
        }
    }

    fn ident(&mut self) -> Result<String, ParseError> {
        if let Token::Ident(s) = self.peek() {
            self.bump();
            Ok(s)
        } else {
            Err(self.err("identifier"))
        }
    }

    fn num(&mut self) -> Result<u64, ParseError> {
        if let Token::Num(n) = self.peek() {
            self.bump();
            Ok(n)
        } else {
            Err(self.err("number"))
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut defs = Vec::new();
        let mut metas = Vec::new();
        loop {
            self.skip_nl();
            match self.peek() {
                Token::Eof | Token::Therefore => break,
                Token::Meta => metas.push(self.parse_meta()?),
                _ => defs.push(self.parse_def()?),
            }
        }
        if !self.eat(Token::Therefore) {
            return Err(self.err("'∴'"));
        }
        let goal = self.parse_expr()?;
        self.skip_nl();
        self.expect(Token::End, "'∎'")?;
        self.skip_nl();
        match self.peek() {
            Token::Eof => Ok(Program { defs, metas, goal }),
            _ => Err(self.err("end of proof")),
        }
    }

    fn parse_meta(&mut self) -> Result<MetaStmt, ParseError> {
        self.bump();
        let level = if self.eat(Token::Underscore) {
            if self.eat(Token::LBrace) {
                let n = self.num()?;
                self.expect(Token::RBrace, "'}'")?;
                Some(n)
            } else {
                Some(self.num()?)
            }
        } else {
            None
        };
        if self.eat(Token::Turnstile) || self.eat(Token::Therefore) {
            let body = self.parse_pred()?;
            return Ok(MetaStmt {
                level,
                head: Some(MetaHead::Turnstile),
                body: Some(body),
            });
        }
        if self.eat(Token::Models) {
            let body = self.parse_pred()?;
            return Ok(MetaStmt {
                level,
                head: Some(MetaHead::Models),
                body: Some(body),
            });
        }
        if self.eat(Token::Halts) {
            let body = self.parse_pred()?;
            return Ok(MetaStmt {
                level,
                head: Some(MetaHead::Halts),
                body: Some(body),
            });
        }
        if self.eat(Token::False) {
            return Ok(MetaStmt {
                level,
                head: Some(MetaHead::Bottom),
                body: None,
            });
        }
        if self.eat(Token::End) {
            return Ok(MetaStmt {
                level,
                head: Some(MetaHead::Qed),
                body: None,
            });
        }
        let body = self.parse_expr()?;
        Ok(MetaStmt {
            level,
            head: None,
            body: Some(body),
        })
    }

    fn parse_def(&mut self) -> Result<Def, ParseError> {
        let name = self.ident()?;
        let sub = if self.eat(Token::Underscore) {
            Some(self.parse_index()?)
        } else {
            None
        };
        let mut args = Vec::new();
        if self.eat(Token::LParen) {
            if !self.eat(Token::RParen) {
                loop {
                    args.push(self.parse_expr()?);
                    if self.eat(Token::Comma) {
                        continue;
                    }
                    self.expect(Token::RParen, "')'")?;
                    break;
                }
            }
        }
        self.expect(Token::Define, "':='")?;
        self.skip_nl();
        let body = self.parse_expr()?;
        Ok(Def {
            name,
            sub,
            args,
            body,
        })
    }

    fn parse_index(&mut self) -> Result<Expr, ParseError> {
        if self.eat(Token::LBrace) {
            let e = self.parse_expr()?;
            self.expect(Token::RBrace, "'}'")?;
            Ok(e)
        } else {
            self.parse_atom()
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or(true)
    }

    fn parse_pred(&mut self) -> Result<Expr, ParseError> {
        self.parse_or(false)
    }

    fn parse_or(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let mut e = self.parse_and(jx)?;
        while self.eat(Token::Or) {
            let r = self.parse_and(jx)?;
            e = Expr::Bin(BinOp::Or, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_and(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let mut e = self.parse_cmp(jx)?;
        while self.eat(Token::And) {
            let r = self.parse_cmp(jx)?;
            e = Expr::Bin(BinOp::And, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_cmp(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let l = self.parse_range(jx)?;
        if self.eat(Token::Equiv) {
            let b = self.parse_range(jx)?;
            self.expect(Token::Mod, "'mod'")?;
            let n = self.parse_range(jx)?;
            return Ok(Expr::Congruent {
                a: Box::new(l),
                b: Box::new(b),
                n: Box::new(n),
            });
        }
        let op = match self.peek() {
            Token::Equals => Cmp::Eq,
            Token::Ne => Cmp::Ne,
            Token::Lt => Cmp::Lt,
            Token::Gt => Cmp::Gt,
            Token::Le => Cmp::Le,
            Token::Ge => Cmp::Ge,
            Token::In => Cmp::In,
            Token::Subset => Cmp::Subset,
            Token::Divides if !jx => Cmp::Divides,
            Token::NotDiv if !jx => Cmp::NotDivides,
            _ => return Ok(l),
        };
        self.bump();
        let r = self.parse_range(jx)?;
        Ok(Expr::Cmp(op, Box::new(l), Box::new(r)))
    }

    fn parse_range(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let l = self.parse_concat(jx)?;
        if self.eat(Token::DotDot) {
            let r = self.parse_concat(jx)?;
            Ok(Expr::Range(Box::new(l), Box::new(r)))
        } else {
            Ok(l)
        }
    }

    fn parse_concat(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let mut e = self.parse_add(jx)?;
        while self.eat(Token::Concat) {
            let r = self.parse_add(jx)?;
            e = Expr::Bin(BinOp::Concat, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn parse_add(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let mut e = self.parse_mul(jx)?;
        loop {
            if self.eat(Token::Plus) {
                let r = self.parse_mul(jx)?;
                e = Expr::Bin(BinOp::Add, Box::new(e), Box::new(r));
            } else if self.eat(Token::Minus) {
                let r = self.parse_mul(jx)?;
                e = Expr::Bin(BinOp::Sub, Box::new(e), Box::new(r));
            } else if self.eat(Token::Union) {
                let r = self.parse_mul(jx)?;
                e = Expr::Bin(BinOp::Union, Box::new(e), Box::new(r));
            } else if self.eat(Token::Intersect) {
                let r = self.parse_mul(jx)?;
                e = Expr::Bin(BinOp::Intersect, Box::new(e), Box::new(r));
            } else if self.eat(Token::SetMinus) {
                let r = self.parse_mul(jx)?;
                e = Expr::Bin(BinOp::SetMinus, Box::new(e), Box::new(r));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_mul(&mut self, jx: bool) -> Result<Expr, ParseError> {
        let mut e = self.parse_pow()?;
        loop {
            if self.eat(Token::Star) {
                let r = self.parse_pow()?;
                e = Expr::Bin(BinOp::Mul, Box::new(e), Box::new(r));
            } else if self.eat(Token::Slash) {
                let r = self.parse_pow()?;
                e = Expr::Bin(BinOp::Div, Box::new(e), Box::new(r));
            } else if self.eat(Token::Compose) {
                let r = self.parse_pow()?;
                e = Expr::Bin(BinOp::Compose, Box::new(e), Box::new(r));
            } else if jx && starts_atom(&self.peek()) {
                let r = self.parse_pow()?;
                e = Expr::Bin(BinOp::Juxt, Box::new(e), Box::new(r));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_pow(&mut self) -> Result<Expr, ParseError> {
        let b = self.parse_postfix()?;
        if !self.eat(Token::Caret) {
            return Ok(b);
        }
        if self.eat(Token::LBrace) {
            self.skip_nl();
            if self.eat(Token::Compose) {
                let n = self.parse_expr()?;
                self.skip_nl();
                self.expect(Token::RBrace, "'}'")?;
                return self.parse_postfix_ops(Expr::Iterate {
                    func: Box::new(b),
                    count: Box::new(n),
                });
            }
            let e = self.parse_expr()?;
            self.skip_nl();
            self.expect(Token::RBrace, "'}'")?;
            return self.parse_postfix_ops(Expr::Bin(BinOp::Pow, Box::new(b), Box::new(e)));
        }
        if self.eat(Token::Compose) {
            let n = self.parse_pred()?;
            return self.parse_postfix_ops(Expr::Iterate {
                func: Box::new(b),
                count: Box::new(n),
            });
        }
        let e = self.parse_pow()?;
        self.parse_postfix_ops(Expr::Bin(BinOp::Pow, Box::new(b), Box::new(e)))
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let e = self.parse_atom()?;
        self.parse_postfix_ops(e)
    }

    fn parse_postfix_ops(&mut self, mut e: Expr) -> Result<Expr, ParseError> {
        loop {
            if self.eat(Token::Factorial) {
                e = Expr::Fact(Box::new(e));
            } else if self.peek() == Token::Underscore {
                self.bump();
                let i = self.parse_index()?;
                e = Expr::Index(Box::new(e), Box::new(i));
            } else if self.peek() == Token::LParen {
                self.bump();
                let mut args = Vec::new();
                if !self.eat(Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.eat(Token::Comma) {
                            continue;
                        }
                        self.expect(Token::RParen, "')'")?;
                        break;
                    }
                }
                e = Expr::Call(Box::new(e), args);
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        if let Token::Num(_) = self.peek() {
            return Ok(Expr::Num(self.num()?));
        }
        if let Token::Ident(_) = self.peek() {
            return Ok(Expr::Var(self.ident()?));
        }
        match self.peek() {
            Token::Eps => {
                self.bump();
                Ok(Expr::Eps)
            }
            Token::True => {
                self.bump();
                Ok(Expr::True)
            }
            Token::False => {
                self.bump();
                Ok(Expr::False)
            }
            Token::Infinity => {
                self.bump();
                Ok(Expr::Infinity)
            }
            Token::Ellipsis => {
                self.bump();
                Ok(Expr::Ellipsis)
            }
            Token::InputChars => {
                self.bump();
                Ok(Expr::InputChars)
            }
            Token::InputNums => {
                self.bump();
                Ok(Expr::InputNums)
            }
            Token::Source => {
                self.bump();
                Ok(Expr::Source)
            }
            Token::Not => {
                self.bump();
                Ok(Expr::Not(Box::new(self.parse_atom()?)))
            }
            Token::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                if self.eat(Token::Comma) {
                    let mut items = vec![e];
                    loop {
                        items.push(self.parse_expr()?);
                        if self.eat(Token::Comma) {
                            continue;
                        }
                        break;
                    }
                    self.expect(Token::RParen, "')'")?;
                    Ok(Expr::Tuple(items))
                } else {
                    self.expect(Token::RParen, "')'")?;
                    Ok(e)
                }
            }
            Token::LBrace => self.parse_braces(),
            Token::FloorL => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(Token::FloorR, "'⌋'")?;
                Ok(Expr::Floor(Box::new(e)))
            }
            Token::CeilL => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(Token::CeilR, "'⌉'")?;
                Ok(Expr::Ceil(Box::new(e)))
            }
            Token::Sqrt => {
                self.bump();
                Ok(Expr::Sqrt(Box::new(self.parse_postfix()?)))
            }
            Token::Sum => self.parse_big(BigOp::Sum),
            Token::Prod => self.parse_big(BigOp::Prod),
            Token::BigConcat => self.parse_big(BigOp::Concat),
            Token::Forall => self.parse_forall(),
            Token::Exists => self.parse_exists(),
            Token::Mu => self.parse_mu(),
            Token::Chi => {
                self.bump();
                self.expect(Token::LParen, "'('")?;
                let mut args = Vec::new();
                if !self.eat(Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.eat(Token::Comma) {
                            continue;
                        }
                        self.expect(Token::RParen, "')'")?;
                        break;
                    }
                }
                Ok(Expr::Chi(args))
            }
            Token::Dec => self.parse_paren1(Expr::Dec),
            Token::FloorFn => self.parse_paren1(Expr::Floor),
            Token::SqrtFn => self.parse_paren1(Expr::Sqrt),
            Token::CeilFn => self.parse_paren1(Expr::Ceil),
            Token::EquivFn => {
                self.bump();
                self.expect(Token::LParen, "'('")?;
                let a = self.parse_expr()?;
                self.expect(Token::Comma, "','")?;
                let b = self.parse_expr()?;
                self.expect(Token::Comma, "','")?;
                let n = self.parse_expr()?;
                self.expect(Token::RParen, "')'")?;
                Ok(Expr::Congruent {
                    a: Box::new(a),
                    b: Box::new(b),
                    n: Box::new(n),
                })
            }
            Token::Divides => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(Token::Divides, "'|'")?;
                Ok(Expr::Len(Box::new(e)))
            }
            _ => Err(self.err("expression")),
        }
    }

    fn parse_big(&mut self, op: BigOp) -> Result<Expr, ParseError> {
        self.bump();
        let (var, bound) = if self.eat(Token::Underscore) {
            self.skip_nl();
            if self.eat(Token::LBrace) {
                self.skip_nl();
                let v = self.ident()?;
                let b = self.parse_bound()?;
                self.skip_nl();
                self.expect(Token::RBrace, "'}'")?;
                (v, Some(b))
            } else {
                let v = self.ident()?;
                let b = if self.eat(Token::Equals) {
                    Some(Bound::Eq(Box::new(self.parse_atom()?)))
                } else if self.eat(Token::In) {
                    Some(Bound::In(Box::new(self.parse_atom()?)))
                } else {
                    None
                };
                (v, b)
            }
        } else {
            (String::new(), None)
        };
        let hi = if self.eat(Token::Caret) {
            if self.eat(Token::LBrace) {
                self.skip_nl();
                let e = self.parse_expr()?;
                self.skip_nl();
                self.expect(Token::RBrace, "'}'")?;
                Some(Box::new(e))
            } else {
                Some(Box::new(self.parse_atom()?))
            }
        } else if self.peek() == Token::Infinity {
            self.bump();
            Some(Box::new(Expr::Infinity))
        } else {
            None
        };
        let body = self.parse_expr()?;
        Ok(Expr::Big {
            op,
            var,
            bound,
            hi,
            body: Box::new(body),
        })
    }

    fn parse_paren1(&mut self, f: impl FnOnce(Box<Expr>) -> Expr) -> Result<Expr, ParseError> {
        self.bump();
        self.expect(Token::LParen, "'('")?;
        let e = self.parse_expr()?;
        self.expect(Token::RParen, "')'")?;
        Ok(f(Box::new(e)))
    }

    fn parse_bound(&mut self) -> Result<Bound, ParseError> {
        if self.eat(Token::Equals) {
            Ok(Bound::Eq(Box::new(self.parse_pred()?)))
        } else if self.eat(Token::In) {
            Ok(Bound::In(Box::new(self.parse_pred()?)))
        } else {
            Err(self.err("'=' or '∈'"))
        }
    }

    fn parse_forall(&mut self) -> Result<Expr, ParseError> {
        self.bump();
        let var = self.ident()?;
        self.expect(Token::In, "'∈'")?;
        let set = self.parse_pred()?;
        self.expect(Token::Colon, "':'")?;
        let body = self.parse_pred()?;
        Ok(Expr::Forall {
            var,
            set: Box::new(set),
            body: Box::new(body),
        })
    }

    fn parse_exists(&mut self) -> Result<Expr, ParseError> {
        self.bump();
        let var = self.ident()?;
        self.expect(Token::In, "'∈'")?;
        let set = self.parse_pred()?;
        self.expect(Token::Colon, "':'")?;
        let body = self.parse_pred()?;
        Ok(Expr::Exists {
            var,
            set: Box::new(set),
            body: Box::new(body),
        })
    }

    fn parse_mu(&mut self) -> Result<Expr, ParseError> {
        self.bump();
        let var = self.ident()?;
        self.expect(Token::End, "'.'")?;
        let body = self.parse_pred()?;
        Ok(Expr::Mu {
            var,
            body: Box::new(body),
        })
    }

    fn parse_braces(&mut self) -> Result<Expr, ParseError> {
        self.bump();
        self.skip_nl();
        let first = self.parse_expr()?;
        if self.eat(Token::Divides) {
            let mut bs = vec![Branch {
                value: first,
                guard: Some(self.parse_pred()?),
            }];
            loop {
                self.skip_nl();
                match self.peek() {
                    Token::RBrace => {
                        self.bump();
                        break;
                    }
                    Token::Eof => return Err(self.err("'}'")),
                    _ => {
                        let v = self.parse_expr()?;
                        let g = if self.eat(Token::Divides) {
                            Some(self.parse_pred()?)
                        } else {
                            None
                        };
                        bs.push(Branch { value: v, guard: g });
                    }
                }
            }
            Ok(Expr::Cases(bs))
        } else if self.eat(Token::Comma) {
            let mut items = vec![first];
            loop {
                self.skip_nl();
                items.push(self.parse_expr()?);
                if self.eat(Token::Comma) {
                    continue;
                }
                break;
            }
            self.skip_nl();
            self.expect(Token::RBrace, "'}'")?;
            Ok(Expr::Set(items))
        } else {
            self.skip_nl();
            self.expect(Token::RBrace, "'}'")?;
            Ok(Expr::Set(vec![first]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Result<Program, String> {
        let toks = Lexer::new(src).lex().map_err(|e| e.to_string())?;
        Parser::new(toks).parse().map_err(|e| e.to_string())
    }

    fn ok(src: &str) -> Program {
        parse(src).unwrap_or_else(|e| panic!("should parse:\n{src}\n{e}"))
    }

    #[test]
    fn define_and_call() {
        let p = ok("f(x) := x² + 1\n∴ f(3) ∎");
        assert_eq!(p.defs.len(), 1);
        assert_eq!(p.defs[0].name, "f");
        assert_eq!(p.defs[0].args, vec![Expr::Var("x".into())]);
        assert!(matches!(
            &p.defs[0].body,
            Expr::Bin(BinOp::Add, l, r)
            if matches!(&**l, Expr::Bin(BinOp::Pow, _, _))
                && matches!(&**r, Expr::Num(1))
        ));
        match &p.goal {
            Expr::Call(f, args) => {
                assert_eq!(**f, Expr::Var("f".into()));
                assert_eq!(*args, vec![Expr::Num(3)]);
            }
            g => panic!("want call, got {g:?}"),
        }
    }

    #[test]
    fn three_spellings_parse() {
        ok("∴ ⨁ᵢ₌₁¹⁴ χ(wᵢ) ∎");
        ok("∴ ⨁_{i=1}^{14} χ(w_i) ∎");
        ok("|- bigoplus_{i=1..14} chi(w_i) .");
    }

    #[test]
    fn hello() {
        let p = ok("w := χ(72, 101, 108)\n∴ ⨁ᵢ₌₁³ χ(wᵢ) ∎");
        match &p.defs[0].body {
            Expr::Chi(args) => assert_eq!(args.len(), 3),
            b => panic!("want chi, got {b:?}"),
        }
        assert!(matches!(p.goal, Expr::Big { .. }));
    }

    #[test]
    fn truth_machine() {
        let p = ok("∴ { χ(48) | N₁ = 0\n⨁ₙ₌₁∞ χ(49) | N₁ = 1 } ∎");
        match &p.goal {
            Expr::Cases(bs) => {
                assert_eq!(bs.len(), 2);
                assert!(bs.iter().all(|b| b.guard.is_some()));
            }
            g => panic!("want cases, got {g:?}"),
        }
    }

    #[test]
    fn cat() {
        assert_eq!(ok("∴ 𝒞 ∎").goal, Expr::InputChars);
    }

    #[test]
    fn reverse_both_spellings() {
        ok("∴ ⨁ᵢ₌₁^{∣𝒞∣} 𝒞_{∣𝒞∣-i+1} ∎");
        ok("|- bigoplus_{i=1}^{|C|} C_{|C| - i+1} .");
    }

    #[test]
    fn sum_inputs() {
        let p = ok("∴ ∑_{x∈𝒩} x ∎");
        match &p.goal {
            Expr::Big {
                op,
                var,
                bound,
                hi,
                body,
            } => {
                assert_eq!(*op, BigOp::Sum);
                assert_eq!(var, "x");
                assert!(matches!(bound, Some(Bound::In(s)) if **s == Expr::InputNums));
                assert!(hi.is_none());
                assert_eq!(**body, Expr::Var("x".into()));
            }
            g => panic!("want sum, got {g:?}"),
        }
    }

    #[test]
    fn infinite_bounds() {
        let p = ok("∴ ∑ᵢ₌₁∞ 𝒩ᵢ ∎");
        assert!(matches!(&p.goal, Expr::Big { hi: Some(hi), .. } if **hi == Expr::Infinity));
        ok("∴ ⨁ₙ₌₀∞ δ(n) ∎");
    }

    #[test]
    fn factorial() {
        let p = ok("F(n) := { 1 | n = 0\nnF(n-1) | n > 0 }\n∴ F(5) ∎");
        match &p.defs[0].body {
            Expr::Cases(bs) => assert_eq!(bs.len(), 2),
            b => panic!("want cases, got {b:?}"),
        }
        assert_eq!(ok("∴ 5! ∎").goal, Expr::Fact(Box::new(Expr::Num(5))));
    }

    #[test]
    fn fibonacci() {
        let p = ok("F₀ := 0\nF₁ := 1\nF_{n+2} := F_{n+1} + F_n\n∴ ⨁ᵢ₌₀¹⁰ (δ(Fᵢ) ⊕ χ(10)) ∎");
        assert_eq!(p.defs.len(), 3);
        assert_eq!(p.defs[0].body, Expr::Num(0));
        assert_eq!(p.defs[1].body, Expr::Num(1));
        assert!(p.defs[2].sub.is_some());
    }

    #[test]
    fn squares() {
        let p = ok("∴ ⨁ₙ₌₁¹⁰ (δ(n²) ⊕ χ(10)) ∎");
        assert!(matches!(p.goal, Expr::Big { .. }));
    }

    #[test]
    fn fizzbuzz() {
        let p = ok(
            "f := χ(70)χ(105)χ(122)χ(122)\nb := χ(66)χ(117)χ(122)χ(122)\ng(n) := { fb | 15 ∣ n\nf | 3 ∣ n\nb | 5 ∣ n\nδ(n) | ⊤ }\n∴ ⨁ₙ₌₁¹⁰⁰ (g(n) ⊕ χ(10)) ∎",
        );
        assert_eq!(p.defs.len(), 3);
        match &p.defs[2].body {
            Expr::Cases(bs) => assert_eq!(bs.len(), 4),
            b => panic!("want cases, got {b:?}"),
        }
    }

    #[test]
    fn primes_both_spellings() {
        let u = ok(
            "P(n) := { 1 | n ≥ 2 ∧ ∀d ∈ {2,…,⌊√n⌋} : d ∤ n\n0 | ⊤ }\n∴ ⨁ₙ₌₁¹⁰⁰ { δ(n) ⊕ χ(10) | P(n) = 1\nε | ⊤ } ∎",
        );
        match &u.defs[0].body {
            Expr::Cases(bs) => assert_eq!(bs.len(), 2),
            b => panic!("want cases, got {b:?}"),
        }
        ok(
            "P(n) := { 1 | n >= 2 & forall d in {2,...,floor(sqrt(n))} : d notdiv n\n0 | true }\n|- bigoplus_{n=1..100} { dec(n) ++ chi(10) | P(n) = 1\neps | true } .",
        );
    }

    #[test]
    fn euclid() {
        let p = ok("G(a,0) := a\nG(a,b) := G(b, a - ⌊a/b⌋b)\n∴ G(48,18) ∎");
        assert_eq!(p.defs.len(), 2);
        assert_eq!(p.defs[0].args, vec![Expr::Var("a".into()), Expr::Num(0)]);
    }

    #[test]
    fn collatz() {
        let p = ok(
            "T(n) := { n/2 | 2 ∣ n\n3n+1 | 2 ∤ n }\nτ(n) := μk. Tᵏ(n) = 1\n∴ ⨁ᵢ₌₀^{τ(27)} (δ(Tⁱ(27)) ⊕ χ(10)) ∎",
        );
        assert_eq!(p.defs.len(), 2);
        match &p.defs[0].body {
            Expr::Cases(bs) => {
                assert_eq!(bs.len(), 2);
                assert_eq!(
                    bs[1].value,
                    Expr::Bin(
                        BinOp::Add,
                        Box::new(Expr::Bin(
                            BinOp::Juxt,
                            Box::new(Expr::Num(3)),
                            Box::new(Expr::Var("n".into())),
                        )),
                        Box::new(Expr::Num(1)),
                    )
                );
            }
            b => panic!("want cases, got {b:?}"),
        }
    }

    #[test]
    fn unbounded_search() {
        let p = ok("∴ μx. x² > 100 ∎");
        assert!(matches!(p.goal, Expr::Mu { .. }));
        ok("∴ μx. x ≠ x ∎");
    }

    #[test]
    fn fragments_fail() {
        for src in [
            "x := e",
            "f(x) := e",
            "χ(108)² = χ(108)χ(108)",
            "T(n) := { n/2 | 2 ∣ n\n3n+1 | 2 ∤ n }",
        ] {
            assert!(parse(src).is_err(), "fragment should fail:\n{src}");
        }
    }

    #[test]
    fn sugar_splits() {
        let p = ok("∴ χ(wᵢ) ∎");
        assert!(matches!(p.goal, Expr::Chi(ref a) if matches!(&a[0],
            Expr::Index(b, i) if **b == Expr::Var("w".into())
                && **i == Expr::Var("i".into()))));
    }

    #[test]
    fn bigop_bound_hi() {
        let p = ok("∴ ⨁ₙ₌₁¹⁰ δ(n) ∎");
        assert!(matches!(p.goal,
            Expr::Big { bound: Some(Bound::Eq(lo)), hi: Some(hi), .. }
            if *lo == Expr::Num(1) && *hi == Expr::Num(10)));
    }

    #[test]
    fn juxtaposition_stays_on_its_line() {
        let p = ok("F₀ := 0\nF₁ := 1\n∴ F₁ ∎");
        assert_eq!(p.defs.len(), 2);
        assert_eq!(p.defs[0].body, Expr::Num(0));
        assert_eq!(p.defs[1].body, Expr::Num(1));
    }

    #[test]
    fn both_bars_mean_length() {
        assert_eq!(ok("∴ ∣𝒞∣ ∎").goal, ok("∴ |C| ∎").goal);
        assert_eq!(ok("∴ ∣𝒞∣ ∎").goal, Expr::Len(Box::new(Expr::InputChars)));
    }

    #[test]
    fn range_is_not_chained() {
        ok("∴ ⨁_{i=1..3} χ(i) ∎");
        assert!(parse("∴ 1..10..20 ∎").is_err());
    }

    #[test]
    fn juxtaposition_inside_call_args() {
        let p = ok("∴ g(a b, c) ∎");
        match &p.goal {
            Expr::Call(f, args) => {
                assert_eq!(**f, Expr::Var("g".into()));
                assert_eq!(
                    *args,
                    vec![
                        Expr::Bin(
                            BinOp::Juxt,
                            Box::new(Expr::Var("a".into())),
                            Box::new(Expr::Var("b".into())),
                        ),
                        Expr::Var("c".into()),
                    ]
                );
            }
            g => panic!("want call, got {g:?}"),
        }
    }

    #[test]
    fn negation_starts_a_factor() {
        assert_eq!(
            ok("∴ x ¬y ∎").goal,
            Expr::Bin(
                BinOp::Juxt,
                Box::new(Expr::Var("x".into())),
                Box::new(Expr::Not(Box::new(Expr::Var("y".into())))),
            )
        );
    }

    #[test]
    fn starts_atom_covers_parse_atom() {
        for t in [
            Token::Num(0),
            Token::Ident("x".into()),
            Token::LParen,
            Token::Chi,
            Token::Dec,
            Token::Eps,
            Token::InputChars,
            Token::InputNums,
            Token::Source,
            Token::True,
            Token::False,
            Token::Infinity,
            Token::Ellipsis,
            Token::FloorL,
            Token::CeilL,
            Token::Sqrt,
            Token::Sum,
            Token::Prod,
            Token::BigConcat,
            Token::Forall,
            Token::Exists,
            Token::Mu,
            Token::Not,
            Token::FloorFn,
            Token::SqrtFn,
            Token::CeilFn,
            Token::EquivFn,
        ] {
            assert!(starts_atom(&t), "{t:?} should start a factor");
        }
        for t in [
            Token::Define,
            Token::Therefore,
            Token::End,
            Token::RParen,
            Token::Comma,
            Token::LBrace,
            Token::RBrace,
            Token::Underscore,
            Token::Caret,
            Token::Equals,
            Token::DotDot,
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::Newline,
            Token::Concat,
            Token::And,
            Token::Or,
            Token::In,
            Token::NotDiv,
            Token::Ne,
            Token::FloorR,
            Token::CeilR,
            Token::Lt,
            Token::Gt,
            Token::Le,
            Token::Ge,
            Token::Divides,
            Token::Equiv,
            Token::Mod,
            Token::Union,
            Token::Intersect,
            Token::SetMinus,
            Token::Subset,
            Token::Compose,
            Token::Colon,
            Token::Factorial,
            Token::Meta,
            Token::Turnstile,
            Token::Halts,
            Token::Models,
            Token::Eof,
        ] {
            assert!(!starts_atom(&t), "{t:?} should not start a factor");
        }
    }

    #[test]
    fn meta_prose_is_skipped() {
        let p = ok("x := 1\nℳ{ hi ∴ } ℳ₀{ lvl }\n∴ x ∎");
        assert_eq!(p.defs.len(), 1);
        assert!(p.metas.is_empty());
        assert!(parse("∴ 1 ∎ ℳ{oops").is_err());
    }

    #[test]
    fn meta_statements() {
        let p = ok("ℳ ⊢ F(5) = 120\nℳ ↓ F\nℳ ⊥\nℳ ∎\nℳ₀ lazy\nmeta |- x = x\nf(x) := x\n∴ f(1) ∎");
        assert_eq!(p.metas.len(), 6);
        let heads: Vec<_> = p.metas.iter().map(|m| m.head.clone()).collect();
        assert_eq!(
            heads,
            vec![
                Some(MetaHead::Turnstile),
                Some(MetaHead::Halts),
                Some(MetaHead::Bottom),
                Some(MetaHead::Qed),
                None,
                Some(MetaHead::Turnstile),
            ]
        );
        assert_eq!(p.metas[4].level, Some(0));
        assert!(p.metas[0].level.is_none());
    }
}
