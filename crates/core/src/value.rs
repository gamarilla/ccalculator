//! The evaluator's value type: a high-precision number tagged with a physical
//! dimension. Plain math uses dimensionless values; the units converter uses the
//! dimension vector to validate and perform conversions.

use crate::number::{Ctx, Num};

/// Number of SI base dimensions we track.
/// Order: length(m), mass(g), time(s), current(A), temperature(K), amount(mol),
/// luminous intensity(cd), angle(rad).
pub const NDIM: usize = 8;

pub const DIM_NAMES: [&str; NDIM] = ["m", "g", "s", "A", "K", "mol", "cd", "rad"];

/// A physical dimension expressed as integer exponents over the base units.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dim(pub [i8; NDIM]);

impl Dim {
    pub const NONE: Dim = Dim([0; NDIM]);

    pub fn is_none(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }

    pub fn base(i: usize) -> Dim {
        let mut d = [0i8; NDIM];
        d[i] = 1;
        Dim(d)
    }

    pub fn mul(&self, o: &Dim) -> Dim {
        let mut d = self.0;
        d.iter_mut().zip(o.0.iter()).for_each(|(x, y)| *x += y);
        Dim(d)
    }

    pub fn div(&self, o: &Dim) -> Dim {
        let mut d = self.0;
        d.iter_mut().zip(o.0.iter()).for_each(|(x, y)| *x -= y);
        Dim(d)
    }

    pub fn powi(&self, n: i8) -> Dim {
        let mut d = self.0;
        for x in d.iter_mut() {
            *x *= n;
        }
        Dim(d)
    }

    /// Human-readable dimension string like "m/s" or "m^2-kilogram/...".
    pub fn describe(&self) -> String {
        let mut num = String::new();
        let mut den = String::new();
        for (name, &e) in DIM_NAMES.iter().zip(self.0.iter()) {
            if e == 0 {
                continue;
            }
            let mag = e.unsigned_abs();
            let token = if mag == 1 {
                name.to_string()
            } else {
                format!("{name}^{mag}")
            };
            if e > 0 {
                if !num.is_empty() {
                    num.push('*');
                }
                num.push_str(&token);
            } else {
                if !den.is_empty() {
                    den.push('*');
                }
                den.push_str(&token);
            }
        }
        if num.is_empty() {
            num.push('1');
        }
        if den.is_empty() {
            num
        } else {
            format!("{num}/{den}")
        }
    }
}

/// A value: a number plus its dimension. The number is always normalized into
/// base SI units (meter, gram, second, ...), so converting between units is just
/// scaling by the target unit's factor.
#[derive(Clone, Debug)]
pub struct Value {
    pub num: Num,
    pub dim: Dim,
}

impl Value {
    pub fn scalar(num: Num) -> Value {
        Value {
            num,
            dim: Dim::NONE,
        }
    }

    pub fn from_f64(f: f64) -> Value {
        Value::scalar(Num::from_f64(f))
    }

    pub fn from_i64(i: i64) -> Value {
        Value::scalar(Num::from_i64(i))
    }

    pub fn is_scalar(&self) -> bool {
        self.dim.is_none()
    }

    pub fn require_scalar(&self, op: &str) -> Result<&Num, String> {
        if self.is_scalar() {
            Ok(&self.num)
        } else {
            Err(format!(
                "operator/function '{op}' requires a dimensionless value, got [{}]",
                self.dim.describe()
            ))
        }
    }

    pub fn add(&self, o: &Value, ctx: &Ctx) -> Result<Value, String> {
        if self.dim != o.dim {
            return Err(format!(
                "cannot add incompatible quantities [{}] and [{}]",
                self.dim.describe(),
                o.dim.describe()
            ));
        }
        Ok(Value {
            num: self.num.add(&o.num, ctx),
            dim: self.dim,
        })
    }

    pub fn sub(&self, o: &Value, ctx: &Ctx) -> Result<Value, String> {
        if self.dim != o.dim {
            return Err(format!(
                "cannot subtract incompatible quantities [{}] and [{}]",
                self.dim.describe(),
                o.dim.describe()
            ));
        }
        Ok(Value {
            num: self.num.sub(&o.num, ctx),
            dim: self.dim,
        })
    }

    pub fn mul(&self, o: &Value, ctx: &Ctx) -> Value {
        Value {
            num: self.num.mul(&o.num, ctx),
            dim: self.dim.mul(&o.dim),
        }
    }

    pub fn div(&self, o: &Value, ctx: &Ctx) -> Value {
        Value {
            num: self.num.div(&o.num, ctx),
            dim: self.dim.div(&o.dim),
        }
    }

    pub fn neg(&self) -> Value {
        Value {
            num: self.num.neg(),
            dim: self.dim,
        }
    }
}
