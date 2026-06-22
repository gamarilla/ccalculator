//! Tokenizer for calculator expressions.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    /// A numeric literal: (mantissa string, radix, si-prefix power-of-ten).
    Num(String, u32, i32),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    Shl,
    Shr,
    Amp,
    Pipe,
    At,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    Ne,
    AndAnd,
    OrOr,
    LParen,
    RParen,
    LBrack,
    RBrack,
    Comma,
    Semi,
    Assign,
    Arrow,
}

pub fn lex(input: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut toks = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // comment to end of line
        if c == '#' {
            break;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let (tok, ni) = lex_number(&chars, i)?;
            toks.push(tok);
            i = ni;
            continue;
        }
        if c == '_' || c.is_alphabetic() {
            let start = i;
            while i < chars.len() && (chars[i] == '_' || chars[i].is_alphanumeric()) {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            toks.push(Tok::Ident(s));
            continue;
        }
        // multi-char operators
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        match two.as_str() {
            "<<" => {
                toks.push(Tok::Shl);
                i += 2;
                continue;
            }
            ">>" => {
                toks.push(Tok::Shr);
                i += 2;
                continue;
            }
            "<=" => {
                toks.push(Tok::Le);
                i += 2;
                continue;
            }
            ">=" => {
                toks.push(Tok::Ge);
                i += 2;
                continue;
            }
            "==" => {
                toks.push(Tok::EqEq);
                i += 2;
                continue;
            }
            "!=" => {
                toks.push(Tok::Ne);
                i += 2;
                continue;
            }
            "&&" => {
                toks.push(Tok::AndAnd);
                i += 2;
                continue;
            }
            "||" => {
                toks.push(Tok::OrOr);
                i += 2;
                continue;
            }
            "->" => {
                toks.push(Tok::Arrow);
                i += 2;
                continue;
            }
            _ => {}
        }
        let t = match c {
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '^' => Tok::Caret,
            '%' => Tok::Percent,
            '!' => Tok::Bang,
            '&' => Tok::Amp,
            '|' => Tok::Pipe,
            '@' => Tok::At,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '[' => Tok::LBrack,
            ']' => Tok::RBrack,
            ',' => Tok::Comma,
            ';' => Tok::Semi,
            '=' => Tok::Assign,
            _ => return Err(format!("unexpected character '{c}'")),
        };
        toks.push(t);
        i += 1;
    }
    Ok(toks)
}

const PREFIXES: &[(char, i32)] = &[
    ('P', 15),
    ('T', 12),
    ('G', 9),
    ('M', 6),
    ('k', 3),
    ('c', -2),
    ('m', -3),
    ('u', -6),
    ('n', -9),
    ('p', -12),
    ('f', -15),
    ('a', -18),
];

fn lex_number(chars: &[char], start: usize) -> Result<(Tok, usize), String> {
    let mut i = start;
    // hex / binary / octal
    if chars[i] == '0' && i + 1 < chars.len() {
        let p = chars[i + 1];
        if p == 'x' || p == 'X' {
            let s = i + 2;
            let mut j = s;
            while j < chars.len() && chars[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j == s {
                return Err("malformed hex literal".into());
            }
            let lit: String = chars[s..j].iter().collect();
            return Ok((Tok::Num(lit, 16, 0), j));
        }
        if p == 'b' || p == 'B' {
            let s = i + 2;
            let mut j = s;
            while j < chars.len() && (chars[j] == '0' || chars[j] == '1') {
                j += 1;
            }
            if j == s {
                return Err("malformed binary literal".into());
            }
            let lit: String = chars[s..j].iter().collect();
            return Ok((Tok::Num(lit, 2, 0), j));
        }
        if p == 'o' || p == 'O' {
            let s = i + 2;
            let mut j = s;
            while j < chars.len() && ('0'..='7').contains(&chars[j]) {
                j += 1;
            }
            if j == s {
                return Err("malformed octal literal".into());
            }
            let lit: String = chars[s..j].iter().collect();
            return Ok((Tok::Num(lit, 8, 0), j));
        }
    }
    // decimal integer part
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    // fraction
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    // scientific exponent: e[+/-]digits, only when immediately followed by digits/sign
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        let mut j = i + 1;
        if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
            j += 1;
        }
        if j < chars.len() && chars[j].is_ascii_digit() {
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let mantissa: String = chars[start..i].iter().collect();
    // optional SI prefix scale: a single prefix char NOT followed by another
    // identifier character (so "5cm" stays as 5 * cm, but "5k" becomes 5000).
    let mut si_exp = 0i32;
    if i < chars.len() {
        let pc = chars[i];
        let next_is_ident = i + 1 < chars.len() && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '_');
        if !next_is_ident {
            if let Some(&(_, e)) = PREFIXES.iter().find(|&&(p, _)| p == pc) {
                si_exp = e;
                i += 1;
            }
        }
    }
    Ok((Tok::Num(mantissa, 10, si_exp), i))
}
