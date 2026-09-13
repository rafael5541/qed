// qed's lexer.
// eu sei que não é bonito, ok???

use std::fmt;

use crate::normalize::{subscript, superscript};

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnknownSymbol { ch: char, pos: usize },
    NumeralExceedsNaturals { pos: usize },
    EmptyDerivation,
    UnterminatedMeta { pos: usize },
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnknownSymbol { ch, pos } => {
                write!(f, "∄ interpretation of '{ch}' at {pos}")
            }
            LexError::NumeralExceedsNaturals { pos } => {
                write!(f, "numeral exceeds ℕ at {pos}")
            }
            LexError::EmptyDerivation => {
                write!(f, "∅ is not a derivation")
            }
            LexError::UnterminatedMeta { pos } => {
                write!(f, "∄ '}}' for 'ℳ{{' at {pos}")
            }
        }
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Num(u64),
    Define,
    Therefore,
    End,
    LParen,
    RParen,
    Comma,
    LBrace,
    RBrace,
    Underscore,
    Caret,
    Equals,
    DotDot,
    Plus,
    Minus,
    Star,
    Slash,
    Newline,
    Chi,
    Dec,
    Eps,
    FloorFn,
    SqrtFn,
    CeilFn,
    EquivFn,
    Concat,
    BigConcat,
    InputChars,
    InputNums,
    Source,
    Mu,
    And,
    Or,
    Not,
    True,
    False,
    Forall,
    Exists,
    In,
    NotDiv,
    Ne,
    FloorL,
    FloorR,
    CeilL,
    CeilR,
    Sqrt,
    Sum,
    Prod,
    Lt,
    Gt,
    Le,
    Ge,
    Divides,
    Equiv,
    Mod,
    Union,
    Intersect,
    SetMinus,
    Subset,
    Compose,
    Meta,
    Turnstile,
    Halts,
    Models,
    Colon,
    Ellipsis,
    Infinity,
    Factorial,
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    pending: Vec<Token>,
}

impl Lexer {
    pub fn new(s: &str) -> Self {
        Self {
            input: s.chars().collect(),
            pos: 0,
            pending: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn peek_at(&self, i: usize) -> Option<char> {
        self.input.get(i).copied()
    }

    fn skip_prose(&mut self) -> Result<bool, LexError> {
        let mut i = self.pos;
        loop {
            match self.peek_at(i) {
                Some(c) if c.is_whitespace() || subscript(c).is_some() || superscript(c).is_some() => {
                    i += 1
                }
                _ => break,
            }
        }
        if self.peek_at(i) != Some('{') {
            return Ok(false);
        }
        let brace = i;
        let mut depth = 0usize;
        loop {
            match self.peek_at(i) {
                None => return Err(LexError::UnterminatedMeta { pos: brace }),
                Some('{') => depth += 1,
                Some('}') => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos = i + 1;
                        return Ok(true);
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        if let Some(t) = self.pending.pop() {
            return Ok(t);
        }
        let mut nl = false;
        while let Some(c) = self.peek() {
            if c == '\n' {
                nl = true;
                self.pos += 1;
            } else if c.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if nl {
            return Ok(Token::Newline);
        }
        let err_pos = self.pos;
        let c = match self.bump() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };
        if c == 'ℳ' {
            if self.skip_prose()? {
                return self.next_token();
            }
            return Ok(Token::Meta);
        }
        if subscript(c).is_some() || superscript(c).is_some() {
            self.expand(c)?;
            return self.next_token();
        }
        Ok(match c {
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '_' => Token::Underscore,
            '^' => Token::Caret,
            '*' => Token::Star,
            '/' => Token::Slash,
            '=' => Token::Equals,
            '&' => Token::And,
            '~' => Token::Not,
            '∴' => Token::Therefore,
            '∎' => Token::End,
            'χ' => Token::Chi,
            'δ' => Token::Dec,
            'ε' => Token::Eps,
            '⊕' => Token::Concat,
            '⨁' => Token::BigConcat,
            '𝒞' => Token::InputChars,
            '𝒩' => Token::InputNums,
            '𝒮' => Token::Source,
            'μ' => Token::Mu,
            '∧' => Token::And,
            '∨' => Token::Or,
            '¬' => Token::Not,
            '⊤' => Token::True,
            '⊥' => Token::False,
            '∀' => Token::Forall,
            '∃' => Token::Exists,
            '⊢' => Token::Turnstile,
            '↓' => Token::Halts,
            '⊨' => Token::Models,
            '∈' => Token::In,
            '∤' => Token::NotDiv,
            '≠' => Token::Ne,
            '⌊' => Token::FloorL,
            '⌋' => Token::FloorR,
            '⌈' => Token::CeilL,
            '⌉' => Token::CeilR,
            '√' => Token::Sqrt,
            '∑' => Token::Sum,
            '∏' => Token::Prod,
            '≤' => Token::Le,
            '≥' => Token::Ge,
            '∣' => Token::Divides,
            '≡' => Token::Equiv,
            '∪' => Token::Union,
            '∩' => Token::Intersect,
            '\\' => Token::SetMinus,
            '⊆' => Token::Subset,
            '∘' => Token::Compose,
            '…' => Token::Ellipsis,
            '∞' => Token::Infinity,
            '+' => {
                if self.peek() == Some('+') {
                    self.bump();
                    Token::Concat
                } else {
                    Token::Plus
                }
            }
            '-' => Token::Minus,
            ':' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::Define
                } else {
                    Token::Colon
                }
            }
            '|' => {
                if self.peek() == Some('-') {
                    self.bump();
                    Token::Therefore
                } else if self.peek() == Some('=') {
                    self.bump();
                    Token::Models
                } else {
                    Token::Divides
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::Ne
                } else {
                    Token::Factorial
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::Le
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Token::Ge
                } else {
                    Token::Gt
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.bump();
                    if self.peek() == Some('.') {
                        self.bump();
                        Token::Ellipsis
                    } else {
                        Token::DotDot
                    }
                } else {
                    Token::End
                }
            }
            c if c.is_ascii_digit() => {
                let mut n = c as u64 - '0' as u64;
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.bump();
                        let digit = d as u64 - '0' as u64;
                        n = n
                            .checked_mul(10)
                            .and_then(|v| v.checked_add(digit))
                            .ok_or(LexError::NumeralExceedsNaturals { pos: err_pos })?;
                    } else {
                        break;
                    }
                }
                Token::Num(n)
            }
            c if c.is_alphabetic() => {
                let mut s = String::new();
                s.push(c);
                while let Some(d) = self.peek() {
                    if d.is_alphanumeric() && subscript(d).is_none() && superscript(d).is_none() {
                        s.push(d);
                        self.bump();
                    } else {
                        break;
                    }
                }
                match s.as_str() {
                    "chi" => Token::Chi,
                    "dec" => Token::Dec,
                    "eps" => Token::Eps,
                    "bigoplus" => Token::BigConcat,
                    "prod" => Token::Prod,
                    "source" => Token::Source,
                    "C" => Token::InputChars,
                    "N" => Token::InputNums,
                    "mu" => Token::Mu,
                    "or" => Token::Or,
                    "true" => Token::True,
                    "false" => Token::False,
                    "forall" => Token::Forall,
                    "exists" => Token::Exists,
                    "in" => Token::In,
                    "divides" => Token::Divides,
                    "notdiv" => Token::NotDiv,
                    "equiv" => Token::EquivFn,
                    "mod" => Token::Mod,
                    "union" => Token::Union,
                    "inter" => Token::Intersect,
                    "subset" => Token::Subset,
                    "compose" => Token::Compose,
                    "ceil" => Token::CeilFn,
                    "inf" => Token::Infinity,
                    "floor" => Token::FloorFn,
                    "sqrt" => Token::SqrtFn,
                    "halts" => Token::Halts,
                    "meta" => {
                        if self.skip_prose()? {
                            return self.next_token();
                        }
                        Token::Meta
                    }
                    _ if s.chars().count() == 1 => Token::Ident(s),
                    _ => {
                        for ch in s.chars().rev() {
                            self.pending.push(Token::Ident(ch.to_string()));
                        }
                        return self.next_token();
                    }
                }
            }
            _ => {
                return Err(LexError::UnknownSymbol {
                    ch: c,
                    pos: err_pos,
                });
            }
        })
    }

    fn expand(&mut self, first: char) -> Result<(), LexError> {
        let sub = subscript(first).is_some();
        let map: fn(char) -> Option<char> = if sub { subscript } else { superscript };
        let mut s = String::new();
        match map(first) {
            Some(m) => s.push(m),
            None => {
                return Err(LexError::UnknownSymbol {
                    ch: first,
                    pos: self.pos.saturating_sub(1),
                });
            }
        }
        while let Some(d) = self.peek() {
            match map(d) {
                Some(n) => {
                    s.push(n);
                    self.bump();
                }
                None => break,
            }
        }
        let mut inner = Lexer::new(&s);
        let mut toks = Vec::new();
        loop {
            let t = inner.next_token()?;
            if t == Token::Eof {
                break;
            }
            toks.push(t);
        }
        let mut out = Vec::new();
        out.push(if sub { Token::Underscore } else { Token::Caret });
        if toks.len() == 1 || toks.iter().any(|t| *t == Token::Equals) {
            out.extend(toks);
        } else {
            out.push(Token::LParen);
            out.extend(toks);
            out.push(Token::RParen);
        }
        self.pending.extend(out.into_iter().rev());
        Ok(())
    }

    pub fn lex(&mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        loop {
            let t = self.next_token()?;
            let done = t == Token::Eof;
            out.push(t);
            if done {
                break;
            }
        }
        if out.iter().all(|t| *t == Token::Eof || *t == Token::Newline) {
            return Err(LexError::EmptyDerivation);
        }
        Ok(out)
    }
}
