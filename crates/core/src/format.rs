//! Result formatting: decimal/hex/binary bases, scientific/engineering/financial
//! notation, significant figures, thousands separators and two's-complement.

use crate::number::DecParts;
use crate::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Base {
    Dec,
    Hex,
    Bin,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SciMode {
    Auto,
    Never,
    Always,
    Eng,
    Prefix,
    Finance,
}

/// Display preferences. Mirrors the original's Options dialog.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub base: Base,
    pub sci: SciMode,
    pub sigfigs: usize,
    /// Thousands separator character, if enabled.
    pub thousands: Option<char>,
    /// Decimal point character ('.' normally, ',' in european mode).
    pub decimal_char: char,
    pub twos_complement: bool,
    pub bits: u32,
    /// Fixed decimal places used by Finance mode.
    pub finance_places: usize,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        DisplaySettings {
            base: Base::Dec,
            sci: SciMode::Auto,
            sigfigs: 10,
            thousands: None,
            decimal_char: '.',
            twos_complement: false,
            bits: 32,
            finance_places: 2,
        }
    }
}

/// Format a full value (number + dimension) for display.
pub fn format_value(v: &Value, s: &DisplaySettings) -> String {
    let mut out = format_number(v, s);
    if !v.is_scalar() {
        out.push_str(&format!(" [{}]", v.dim.describe()));
    }
    out
}

fn format_number(v: &Value, s: &DisplaySettings) -> String {
    if v.num.is_nan() {
        return "NaN".to_string();
    }
    if v.num.is_inf() {
        return if v.num.is_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    // Hex / Bin display only makes sense for integers; fall back to decimal otherwise.
    if matches!(s.base, Base::Hex | Base::Bin) && v.is_scalar() {
        if let Some(i) = v.num.to_i128() {
            return format_radix_int(i, s);
        }
    }
    let parts = match v.num.decompose() {
        Some(p) => p,
        None => return "?".to_string(),
    };
    format_decimal(&parts, s)
}

fn format_radix_int(i: i128, s: &DisplaySettings) -> String {
    let (prefix, radix) = match s.base {
        Base::Hex => ("0x", 16u32),
        Base::Bin => ("0b", 2u32),
        Base::Dec => ("", 10u32),
    };
    if i < 0 && s.twos_complement {
        let mask: u128 = if s.bits >= 128 {
            u128::MAX
        } else {
            (1u128 << s.bits) - 1
        };
        let u = (i as u128) & mask;
        return format!("{prefix}{}", to_radix_u128(u, radix));
    }
    if i < 0 {
        format!("-{prefix}{}", to_radix_u128(i.unsigned_abs(), radix))
    } else {
        format!("{prefix}{}", to_radix_u128(i as u128, radix))
    }
}

fn to_radix_u128(mut u: u128, radix: u32) -> String {
    if u == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8] = b"0123456789ABCDEF";
    let mut buf = Vec::new();
    while u > 0 {
        buf.push(DIGITS[(u % radix as u128) as usize]);
        u /= radix as u128;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap()
}

/// Render a decimal value honoring scientific-notation mode and separators.
fn format_decimal(parts: &DecParts, s: &DisplaySettings) -> String {
    if parts.is_zero() {
        return "0".to_string();
    }
    let is_integer = parts.exp10 >= 0;
    // Order of magnitude (exponent if written in scientific form).
    let oom = parts.digits.len() as i64 - 1 + parts.exp10;

    match s.sci {
        SciMode::Never => render_plain(&parts.round_sig(if is_integer { 0 } else { s.sigfigs }), s),
        SciMode::Always => render_sci(&parts.round_sig(s.sigfigs), s, 1),
        SciMode::Eng => render_sci(&parts.round_sig(s.sigfigs), s, 3),
        SciMode::Prefix => render_prefix(&parts.round_sig(s.sigfigs), s),
        SciMode::Finance => render_finance(parts, s),
        SciMode::Auto => {
            // Integers are shown in full unless astronomically large.
            if is_integer && oom < 21 {
                render_plain(parts, s)
            } else if !(-4..=15).contains(&oom) {
                render_sci(&parts.round_sig(s.sigfigs), s, 1)
            } else {
                render_plain(&parts.round_sig(s.sigfigs), s)
            }
        }
    }
}

/// Plain positional decimal, e.g. 12345.678 with optional thousands separators.
fn render_plain(parts: &DecParts, s: &DisplaySettings) -> String {
    let digits = &parts.digits;
    let dlen = digits.len() as i64;
    let (int_str, frac_str) = if parts.exp10 >= 0 {
        // integer, append zeros
        let mut i = digits.clone();
        for _ in 0..parts.exp10 {
            i.push('0');
        }
        (i, String::new())
    } else {
        let point = dlen + parts.exp10; // number of integer digits
        if point > 0 {
            let p = point as usize;
            (digits[..p].to_string(), digits[p..].to_string())
        } else {
            let zeros = (-point) as usize;
            let mut f = "0".repeat(zeros);
            f.push_str(digits);
            ("0".to_string(), f)
        }
    };
    let int_str = group_thousands(&int_str, s);
    let mut out = String::new();
    if parts.neg {
        out.push('-');
    }
    out.push_str(&int_str);
    if !frac_str.is_empty() {
        out.push(s.decimal_char);
        out.push_str(&frac_str);
    }
    out
}

fn group_thousands(int_str: &str, s: &DisplaySettings) -> String {
    let sep = match s.thousands {
        Some(c) => c,
        None => return int_str.to_string(),
    };
    let bytes = int_str.as_bytes();
    let mut out = String::new();
    let n = bytes.len();
    for (idx, b) in bytes.iter().enumerate() {
        if idx > 0 && (n - idx).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(*b as char);
    }
    out
}

/// Scientific / engineering: mantissa with exponent a multiple of `step`.
fn render_sci(parts: &DecParts, s: &DisplaySettings, step: i64) -> String {
    let (mantissa, exp) = mantissa_with_exp(parts, s, step);
    format!("{mantissa}e{exp}")
}

/// Engineering-prefix notation: like engineering but exponent shown as SI prefix.
fn render_prefix(parts: &DecParts, s: &DisplaySettings) -> String {
    let (mantissa, exp) = mantissa_with_exp(parts, s, 3);
    if let Some(p) = si_prefix(exp) {
        if p.is_empty() {
            mantissa
        } else {
            format!("{mantissa}{p}")
        }
    } else {
        format!("{mantissa}e{exp}")
    }
}

/// Produce a mantissa string and exponent where exponent is a multiple of `step`.
fn mantissa_with_exp(parts: &DecParts, s: &DisplaySettings, step: i64) -> (String, i64) {
    let oom = parts.digits.len() as i64 - 1 + parts.exp10;
    // exponent rounded down to nearest multiple of step
    let exp = (oom.div_euclid(step)) * step;
    let shift = oom - exp; // number of integer digits - 1 in mantissa
    // Build mantissa: place decimal point so there are (shift+1) integer digits.
    let intdigits = (shift + 1) as usize;
    let mut digits = parts.digits.clone();
    while digits.len() < intdigits {
        digits.push('0');
    }
    let (ip, fp) = digits.split_at(intdigits.min(digits.len()));
    let mut m = String::new();
    if parts.neg {
        m.push('-');
    }
    m.push_str(ip);
    if !fp.is_empty() {
        m.push(s.decimal_char);
        m.push_str(fp);
    }
    (m, exp)
}

fn render_finance(parts: &DecParts, s: &DisplaySettings) -> String {
    // Fixed number of decimal places, with thousands separators, never scientific.
    let places = s.finance_places;
    // Round to `places` decimal places: value*10^places rounded to integer.
    let target_exp = -(places as i64);
    let rounded = round_to_exp(parts, target_exp);
    render_plain(&rounded, s)
}

/// Round a decimal to a fixed power-of-ten place (e.g. exp10 = -2 for cents).
fn round_to_exp(parts: &DecParts, target_exp: i64) -> DecParts {
    if parts.exp10 >= target_exp {
        return parts.clone();
    }
    // number of digits to drop
    let drop = (target_exp - parts.exp10) as usize;
    if drop >= parts.digits.len() {
        // Rounds to zero or to 1 in the last place
        let total = parts.digits.len();
        let round_up = if drop == total {
            parts.digits.as_bytes()[0] >= b'5'
        } else {
            false
        };
        if round_up {
            return DecParts {
                neg: parts.neg,
                digits: "1".to_string(),
                exp10: target_exp,
            };
        }
        return DecParts {
            neg: false,
            digits: "0".to_string(),
            exp10: 0,
        };
    }
    let sig = parts.digits.len() - drop;
    let mut r = parts.round_sig(sig);
    // round_sig keeps `sig` significant digits; pad exp back to target if needed
    if r.exp10 < target_exp {
        r = r.round_sig(r.digits.len().saturating_sub((target_exp - r.exp10) as usize));
    }
    r
}

fn si_prefix(exp: i64) -> Option<&'static str> {
    Some(match exp {
        15 => "P",
        12 => "T",
        9 => "G",
        6 => "M",
        3 => "k",
        0 => "",
        -3 => "m",
        -6 => "u",
        -9 => "n",
        -12 => "p",
        -15 => "f",
        -18 => "a",
        _ => return None,
    })
}
