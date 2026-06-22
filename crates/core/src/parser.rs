//! Pratt parser producing statements/expressions from tokens.

use crate::ast::{BinOp, Expr, Stmt};
use crate::lexer::{lex, Tok};

pub fn parse_stmt(input: &str) -> Result<Stmt, String> {
    let toks = lex(input)?;
    let mut p = Parser { toks, pos: 0 };
    let stmt = p.parse_statement()?;
    if p.pos != p.toks.len() {
        return Err(format!("unexpected trailing input near token {:?}", p.peek()));
    }
    Ok(stmt)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, t: &Tok) -> Result<(), String> {
        if self.peek() == Some(t) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected {:?}, found {:?}", t, self.peek()))
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        // Assignment: Ident '=' expr
        if let Some(Tok::Ident(name)) = self.peek().cloned() {
            if self.peek_at(1) == Some(&Tok::Assign) {
                self.pos += 2;
                let e = self.expr(0)?;
                return Ok(Stmt::Assign(name, e));
            }
            // Function definition: Ident '(' params ')' '=' expr
            if self.peek_at(1) == Some(&Tok::LParen) {
                if let Some((params, after)) = self.try_match_funcdef_header() {
                    if self.toks.get(after) == Some(&Tok::Assign) {
                        self.pos = after + 1;
                        let e = self.expr(0)?;
                        return Ok(Stmt::FuncDef(name, params, e));
                    }
                }
            }
        }
        let e = self.expr(0)?;
        Ok(Stmt::Expr(e))
    }

    /// If tokens starting at pos look like `Ident ( ident, ident ) =`, return the
    /// parameter list and the index just past ')'. Does not consume.
    fn try_match_funcdef_header(&self) -> Option<(Vec<String>, usize)> {
        // self.pos -> Ident, pos+1 -> '('
        let mut i = self.pos + 2;
        let mut params = Vec::new();
        // empty param list not allowed for a function definition
        loop {
            match self.toks.get(i) {
                Some(Tok::Ident(p)) => {
                    params.push(p.clone());
                    i += 1;
                }
                _ => return None,
            }
            match self.toks.get(i) {
                Some(Tok::Comma) => {
                    i += 1;
                    continue;
                }
                Some(Tok::RParen) => {
                    i += 1;
                    break;
                }
                _ => return None,
            }
        }
        Some((params, i))
    }

    /// Pratt expression parser. `min_bp` is the minimum binding power to continue.
    #[allow(clippy::while_let_loop)]
    fn expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.prefix()?;

        loop {
            let Some(tok) = self.peek() else { break };
            // Postfix factorial
            if tok == &Tok::Bang {
                self.pos += 1;
                lhs = Expr::Fact(Box::new(lhs));
                continue;
            }
            // Infix or implicit multiplication
            let (op, lbp, rbp) = match self.infix_op(tok) {
                Some(x) => x,
                None => {
                    // implicit multiplication when next token starts a primary
                    if self.starts_primary(tok) {
                        (ImOp::Mul, 21, 22)
                    } else {
                        break;
                    }
                }
            };
            if lbp < min_bp {
                break;
            }
            match op {
                ImOp::Mul => {
                    // implicit multiply: do not consume a token
                    let rhs = self.expr(rbp)?;
                    lhs = Expr::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
                }
                ImOp::Convert => {
                    self.pos += 1;
                    let rhs = self.expr(rbp)?;
                    lhs = Expr::Convert(Box::new(lhs), Box::new(rhs));
                }
                ImOp::Bin(b) => {
                    self.pos += 1;
                    let rhs = self.expr(rbp)?;
                    lhs = Expr::Bin(b, Box::new(lhs), Box::new(rhs));
                }
            }
        }
        Ok(lhs)
    }

    fn prefix(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Minus) => {
                self.pos += 1;
                let e = self.expr(20)?; // unary binds tighter than binary, looser than ^
                Ok(Expr::Neg(Box::new(e)))
            }
            Some(Tok::Plus) => {
                self.pos += 1;
                self.expr(20)
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(m, r, si)) => Ok(Expr::Num {
                mantissa: m,
                radix: r,
                si_exp: si,
            }),
            Some(Tok::Ident(name)) => {
                if self.peek() == Some(&Tok::LParen) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.expr(0)?);
                            match self.peek() {
                                Some(&Tok::Comma) => {
                                    self.pos += 1;
                                }
                                _ => break,
                            }
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(Tok::LParen) => {
                let e = self.expr(0)?;
                self.expect(&Tok::RParen)?;
                Ok(e)
            }
            Some(Tok::LBrack) => {
                let e = self.expr(0)?;
                self.expect(&Tok::RBrack)?;
                Ok(e)
            }
            other => Err(format!("expected a value, found {:?}", other)),
        }
    }

    fn starts_primary(&self, tok: &Tok) -> bool {
        matches!(tok, Tok::Num(..) | Tok::Ident(_) | Tok::LParen | Tok::LBrack)
    }

    fn infix_op(&self, tok: &Tok) -> Option<(ImOp, u8, u8)> {
        // (operator, left bp, right bp). Right bp > left bp => left associative.
        Some(match tok {
            Tok::Arrow => (ImOp::Convert, 1, 2),
            Tok::OrOr => (ImOp::Bin(BinOp::LogOr), 3, 4),
            Tok::AndAnd => (ImOp::Bin(BinOp::LogAnd), 5, 6),
            Tok::Pipe => (ImOp::Bin(BinOp::Or), 7, 8),
            Tok::At => (ImOp::Bin(BinOp::Xor), 9, 10),
            Tok::Amp => (ImOp::Bin(BinOp::And), 11, 12),
            Tok::EqEq => (ImOp::Bin(BinOp::Eq), 13, 14),
            Tok::Ne => (ImOp::Bin(BinOp::Ne), 13, 14),
            Tok::Lt => (ImOp::Bin(BinOp::Lt), 15, 16),
            Tok::Le => (ImOp::Bin(BinOp::Le), 15, 16),
            Tok::Gt => (ImOp::Bin(BinOp::Gt), 15, 16),
            Tok::Ge => (ImOp::Bin(BinOp::Ge), 15, 16),
            Tok::Shl => (ImOp::Bin(BinOp::Shl), 17, 18),
            Tok::Shr => (ImOp::Bin(BinOp::Shr), 17, 18),
            Tok::Plus => (ImOp::Bin(BinOp::Add), 19, 20),
            Tok::Minus => (ImOp::Bin(BinOp::Sub), 19, 20),
            Tok::Star => (ImOp::Bin(BinOp::Mul), 21, 22),
            Tok::Slash => (ImOp::Bin(BinOp::Div), 21, 22),
            Tok::Percent => (ImOp::Bin(BinOp::Mod), 21, 22),
            Tok::Caret => (ImOp::Bin(BinOp::Pow), 26, 25), // right associative
            _ => return None,
        })
    }
}

enum ImOp {
    Bin(BinOp),
    Mul,
    Convert,
}
