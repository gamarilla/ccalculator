//! Abstract syntax tree.

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Xor,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    LogAnd,
    LogOr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num {
        mantissa: String,
        radix: u32,
        si_exp: i32,
    },
    Ident(String),
    Call(String, Vec<Expr>),
    Neg(Box<Expr>),
    Fact(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    /// Unit conversion `lhs -> rhs`.
    Convert(Box<Expr>, Box<Expr>),
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Pow => "^",
            BinOp::Mod => "%",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::And => "&",
            BinOp::Or => "|",
            BinOp::Xor => "@",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::LogAnd => "&&",
            BinOp::LogOr => "||",
        };
        f.write_str(s)
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Num {
                mantissa,
                radix,
                si_exp,
            } => {
                match radix {
                    16 => write!(f, "0x{mantissa}")?,
                    2 => write!(f, "0b{mantissa}")?,
                    8 => write!(f, "0o{mantissa}")?,
                    _ => write!(f, "{mantissa}")?,
                }
                if *si_exp != 0 {
                    write!(f, "e{si_exp}")?;
                }
                Ok(())
            }
            Expr::Ident(n) => f.write_str(n),
            Expr::Call(n, args) => {
                write!(f, "{n}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{a}")?;
                }
                f.write_str(")")
            }
            Expr::Neg(e) => write!(f, "-({e})"),
            Expr::Fact(e) => write!(f, "({e})!"),
            Expr::Bin(op, a, b) => write!(f, "({a} {op} {b})"),
            Expr::Convert(a, b) => write!(f, "({a} -> {b})"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Assign(String, Expr),
    FuncDef(String, Vec<String>, Expr),
}
