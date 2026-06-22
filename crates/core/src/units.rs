//! Built-in unit definitions for the units converter.
//!
//! Every unit is stored as a [`Value`]: its numeric value expressed in base SI
//! units (meter, gram, second, ampere, kelvin, mole, candela, radian) together
//! with its dimension vector. Converting `a -> b` then reduces to `a.num / b.num`
//! after checking the dimensions match.

use std::collections::HashMap;

use crate::number::{Ctx, Num};
use crate::value::{Dim, Value, NDIM};

/// Parse a scale string that may use `e` notation (e.g. "1.495978707e11"),
/// independent of the backend's own parser quirks.
pub fn parse_scale(ctx: &mut Ctx, s: &str) -> Num {
    let (mant, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, e.parse::<i64>().unwrap_or(0)),
        None => (s, 0),
    };
    let mut n = Num::parse(mant, 10, ctx).unwrap_or_else(|_| Num::zero());
    if exp != 0 {
        let ten = Num::from_i64(10);
        let e = Num::from_i64(exp.unsigned_abs() as i64);
        let scale = ten.pow(&e, ctx);
        if exp > 0 {
            n = n.mul(&scale, ctx);
        } else {
            n = n.div(&scale, ctx);
        }
    }
    n
}

fn dim(spec: &[(usize, i8)]) -> Dim {
    let mut d = [0i8; NDIM];
    for &(i, e) in spec {
        d[i] = e;
    }
    Dim(d)
}

// Base-unit indices.
const M: usize = 0; // length, meter
const G: usize = 1; // mass, gram
const S: usize = 2; // time, second
const A: usize = 3; // current, ampere
const K: usize = 4; // temperature, kelvin
const MOL: usize = 5; // amount
const CD: usize = 6; // luminous intensity
const RAD: usize = 7; // angle

/// Build the full built-in unit table.
pub fn build_units(ctx: &mut Ctx) -> HashMap<String, Value> {
    let mut map: HashMap<String, Value> = HashMap::new();

    let mut add = |ctx: &mut Ctx, names: &[&str], scale: &str, d: Dim| {
        let num = parse_scale(ctx, scale);
        for n in names {
            map.insert((*n).to_string(), Value { num: num.clone(), dim: d });
        }
    };

    // ---- length (m) ----
    let len = dim(&[(M, 1)]);
    add(ctx, &["m", "meter", "meters", "metre"], "1", len);
    add(ctx, &["km", "kilometer", "kilometers"], "1000", len);
    add(ctx, &["dm", "decimeter"], "0.1", len);
    add(ctx, &["cm", "centimeter", "centimeters"], "0.01", len);
    add(ctx, &["mm", "millimeter", "millimeters"], "0.001", len);
    add(ctx, &["um", "micron", "micrometer"], "1e-6", len);
    add(ctx, &["nm", "nanometer"], "1e-9", len);
    add(ctx, &["angstrom"], "1e-10", len);
    add(ctx, &["fermi"], "1e-15", len);
    add(ctx, &["in", "inch", "inches"], "0.0254", len);
    add(ctx, &["ft", "foot", "feet"], "0.3048", len);
    add(ctx, &["yd", "yard", "yards"], "0.9144", len);
    add(ctx, &["mi", "mile", "miles"], "1609.344", len);
    add(ctx, &["nmi", "nauticalmile"], "1852", len);
    add(ctx, &["fathom"], "1.8288", len);
    add(ctx, &["rod"], "5.0292", len);
    add(ctx, &["furlong"], "201.168", len);
    add(ctx, &["pc", "parsec"], "3.0856775814913673e16", len);
    add(ctx, &["ly", "lightyear"], "9.4607304725808e15", len);
    add(ctx, &["au"], "1.495978707e11", len);

    // ---- mass (g) ----
    let mass = dim(&[(G, 1)]);
    add(ctx, &["g", "gram", "grams"], "1", mass);
    add(ctx, &["kg", "kilogram", "kilograms"], "1000", mass);
    add(ctx, &["mg", "milligram"], "0.001", mass);
    add(ctx, &["ug", "microgram"], "1e-6", mass);
    add(ctx, &["t", "tonne", "metricton"], "1e6", mass);
    add(ctx, &["lb", "lbm", "pound", "pounds"], "453.59237", mass);
    add(ctx, &["oz", "ounce", "ounces"], "28.349523125", mass);
    add(ctx, &["ton"], "907184.74", mass);
    add(ctx, &["slug"], "14593.9029", mass);
    add(ctx, &["stone"], "6350.29318", mass);
    add(ctx, &["grain"], "0.06479891", mass);
    add(ctx, &["carat"], "0.2", mass);

    // ---- time (s) ----
    let time = dim(&[(S, 1)]);
    add(ctx, &["s", "sec", "second", "seconds"], "1", time);
    add(ctx, &["ms", "millisecond"], "0.001", time);
    add(ctx, &["us", "microsecond"], "1e-6", time);
    add(ctx, &["ns", "nanosecond"], "1e-9", time);
    add(ctx, &["min", "minute", "minutes"], "60", time);
    add(ctx, &["h", "hr", "hour", "hours"], "3600", time);
    add(ctx, &["day", "days"], "86400", time);
    add(ctx, &["wk", "week", "weeks"], "604800", time);
    add(ctx, &["yr", "year", "years"], "31557600", time);
    add(ctx, &["fortnight"], "1209600", time);

    // ---- current (A) ----
    let cur = dim(&[(A, 1)]);
    add(ctx, &["A", "ampere", "amp", "amps"], "1", cur);
    add(ctx, &["mA", "milliampere"], "0.001", cur);
    add(ctx, &["kA"], "1000", cur);

    // ---- temperature (K) ----
    let temp = dim(&[(K, 1)]);
    add(ctx, &["K", "kelvin"], "1", temp);

    // ---- amount (mol) ----
    let amt = dim(&[(MOL, 1)]);
    add(ctx, &["mol", "mole", "moles"], "1", amt);
    add(ctx, &["mmol"], "0.001", amt);
    add(ctx, &["kmol"], "1000", amt);

    // ---- luminous intensity (cd) ----
    add(ctx, &["cd", "candela"], "1", dim(&[(CD, 1)]));

    // ---- area (m^2) ----
    let area = dim(&[(M, 2)]);
    add(ctx, &["acre", "acres"], "4046.8564224", area);
    add(ctx, &["hectare", "ha"], "10000", area);
    add(ctx, &["are"], "100", area);
    add(ctx, &["barn"], "1e-28", area);

    // ---- volume (m^3) ----
    let vol = dim(&[(M, 3)]);
    add(ctx, &["L", "liter", "litre", "liters"], "0.001", vol);
    add(ctx, &["mL", "milliliter", "cc"], "1e-6", vol);
    add(ctx, &["gal", "gallon", "gallons"], "0.003785411784", vol);
    add(ctx, &["qt", "quart"], "0.000946352946", vol);
    add(ctx, &["pt", "pint"], "0.000473176473", vol);
    add(ctx, &["cup"], "0.0002365882365", vol);
    add(ctx, &["floz", "fluidounce"], "2.95735295625e-5", vol);
    add(ctx, &["tbsp", "tablespoon"], "1.47867647813e-5", vol);
    add(ctx, &["tsp", "teaspoon"], "4.92892159375e-6", vol);
    add(ctx, &["barrel"], "0.158987294928", vol);
    add(ctx, &["impgal"], "0.00454609", vol);

    // ---- frequency (1/s) ----
    let freq = dim(&[(S, -1)]);
    add(ctx, &["Hz", "hertz"], "1", freq);
    add(ctx, &["kHz"], "1000", freq);
    add(ctx, &["MHz"], "1e6", freq);
    add(ctx, &["GHz"], "1e9", freq);

    // ---- force (g·m/s^2) ----
    let force = dim(&[(G, 1), (M, 1), (S, -2)]);
    add(ctx, &["N", "newton", "newtons"], "1000", force);
    add(ctx, &["kN"], "1e6", force);
    add(ctx, &["dyne"], "0.01", force);
    add(ctx, &["lbf", "poundforce"], "4448.2216152605", force);
    add(ctx, &["kgf"], "9806.65", force);

    // ---- energy (g·m^2/s^2) ----
    let energy = dim(&[(G, 1), (M, 2), (S, -2)]);
    add(ctx, &["J", "joule", "joules"], "1000", energy);
    add(ctx, &["kJ", "kilojoule"], "1e6", energy);
    add(ctx, &["MJ"], "1e9", energy);
    add(ctx, &["mJ", "millijoule", "millijoules"], "1", energy);
    add(ctx, &["cal", "calorie"], "4184", energy);
    add(ctx, &["kcal", "Cal"], "4184000", energy);
    add(ctx, &["eV"], "1.602176634e-16", energy);
    add(ctx, &["BTU"], "1055060", energy);
    add(ctx, &["Wh"], "3.6e6", energy);
    add(ctx, &["kWh"], "3.6e9", energy);
    add(ctx, &["erg"], "1e-4", energy);

    // ---- power (g·m^2/s^3) ----
    let power = dim(&[(G, 1), (M, 2), (S, -3)]);
    add(ctx, &["W", "watt", "watts"], "1000", power);
    add(ctx, &["kW", "kilowatt"], "1e6", power);
    add(ctx, &["MW"], "1e9", power);
    add(ctx, &["mW"], "1", power);
    add(ctx, &["hp", "horsepower"], "745699.872", power);

    // ---- pressure (g/(m·s^2)) ----
    let pres = dim(&[(G, 1), (M, -1), (S, -2)]);
    add(ctx, &["Pa", "pascal"], "1000", pres);
    add(ctx, &["kPa"], "1e6", pres);
    add(ctx, &["MPa"], "1e9", pres);
    add(ctx, &["hPa"], "1e5", pres);
    add(ctx, &["bar"], "1e8", pres);
    add(ctx, &["mbar"], "1e5", pres);
    add(ctx, &["atm"], "1.01325e8", pres);
    add(ctx, &["psi"], "6.894757293e6", pres);
    add(ctx, &["torr", "mmHg"], "133322.368", pres);

    // ---- charge (A·s) ----
    let charge = dim(&[(A, 1), (S, 1)]);
    add(ctx, &["C", "coulomb"], "1", charge);
    add(ctx, &["mC"], "0.001", charge);
    add(ctx, &["uC"], "1e-6", charge);
    add(ctx, &["Ah"], "3600", charge);
    add(ctx, &["mAh"], "3.6", charge);

    // ---- voltage (g·m^2/(s^3·A)) ----
    let volt = dim(&[(G, 1), (M, 2), (S, -3), (A, -1)]);
    add(ctx, &["V", "volt", "volts"], "1000", volt);
    add(ctx, &["mV"], "1", volt);
    add(ctx, &["kV"], "1e6", volt);

    // ---- resistance ----
    let ohm = dim(&[(G, 1), (M, 2), (S, -3), (A, -2)]);
    add(ctx, &["ohm", "ohms"], "1000", ohm);
    add(ctx, &["kohm"], "1e6", ohm);
    add(ctx, &["Mohm"], "1e9", ohm);

    // ---- capacitance ----
    let farad = dim(&[(G, -1), (M, -2), (S, 4), (A, 2)]);
    add(ctx, &["F", "farad"], "0.001", farad);
    add(ctx, &["mF"], "1e-6", farad);
    add(ctx, &["uF"], "1e-9", farad);
    add(ctx, &["nF"], "1e-12", farad);
    add(ctx, &["pF"], "1e-15", farad);

    // ---- conductance ----
    add(ctx, &["S", "siemens"], "0.001", dim(&[(G, -1), (M, -2), (S, 3), (A, 2)]));

    // ---- magnetic flux ----
    let wb = dim(&[(G, 1), (M, 2), (S, -2), (A, -1)]);
    add(ctx, &["Wb", "weber"], "1000", wb);
    add(ctx, &["Mx", "maxwell"], "1e-5", wb);

    // ---- magnetic flux density ----
    let tesla = dim(&[(G, 1), (S, -2), (A, -1)]);
    add(ctx, &["T", "tesla"], "1000", tesla);
    add(ctx, &["G", "gauss"], "0.1", tesla);
    add(ctx, &["mT"], "1", tesla);

    // ---- inductance ----
    let henry = dim(&[(G, 1), (M, 2), (S, -2), (A, -2)]);
    add(ctx, &["H", "henry"], "1000", henry);
    add(ctx, &["mH"], "1", henry);
    add(ctx, &["uH"], "0.001", henry);

    // ---- illumination ----
    add(ctx, &["lm", "lumen"], "1", dim(&[(CD, 1)]));
    add(ctx, &["lx", "lux"], "1", dim(&[(CD, 1), (M, -2)]));

    // ---- angle (rad) ----
    let ang = dim(&[(RAD, 1)]);
    add(ctx, &["rad", "radian", "radians"], "1", ang);
    // angle units derived from pi
    let pi = ctx.pi();
    let n180 = Num::from_i64(180);
    let n200 = Num::from_i64(200);
    let deg = pi.div(&n180, ctx);
    let grad = pi.div(&n200, ctx);
    let two = Num::from_i64(2);
    let rev = pi.mul(&two, ctx);
    for n in ["deg", "degree", "degrees"] {
        map.insert(n.to_string(), Value { num: deg.clone(), dim: ang });
    }
    for n in ["grad", "gradian"] {
        map.insert(n.to_string(), Value { num: grad.clone(), dim: ang });
    }
    for n in ["rev", "revolution", "turn"] {
        map.insert(n.to_string(), Value { num: rev.clone(), dim: ang });
    }
    // arcminute / arcsecond
    let n60 = Num::from_i64(60);
    let arcmin = deg.div(&n60, ctx);
    let arcsec = arcmin.div(&n60, ctx);
    map.insert("arcmin".to_string(), Value { num: arcmin, dim: ang });
    map.insert("arcsec".to_string(), Value { num: arcsec, dim: ang });

    map
}
