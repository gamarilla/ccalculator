//! Behavioral tests covering the documented Console Calculator feature set.

use ccalc_core::{Engine, InputLayout};

/// Evaluate a line and return the single visible result string (last output).
fn run(e: &mut Engine, line: &str) -> String {
    let outs = e.run_line(line);
    outs.iter()
        .filter_map(|ev| e.format_eval(ev))
        .next_back()
        .unwrap_or_default()
}

/// Fresh engine, evaluate one line, return last visible output.
fn one(line: &str) -> String {
    let mut e = Engine::new();
    run(&mut e, line)
}

#[test]
fn basic_arithmetic() {
    assert_eq!(one("4+5*4"), "ans = 24");
    assert_eq!(one("2+3"), "ans = 5");
    assert_eq!(one("10/4"), "ans = 2.5");
    assert_eq!(one("2-5"), "ans = -3");
    assert_eq!(one("(1+2)*3"), "ans = 9");
    assert_eq!(one("[1+2]*3"), "ans = 9");
}

#[test]
fn operators() {
    assert_eq!(one("3^3"), "ans = 27");
    assert_eq!(one("7%3"), "ans = 1");
    assert_eq!(one("4!"), "ans = 24");
    assert_eq!(one("2^10"), "ans = 1024");
    // bitwise
    assert_eq!(one("0xFF + 0b1001"), "ans = 264");
    assert_eq!(one("12 & 10"), "ans = 8");
    assert_eq!(one("12 | 3"), "ans = 15");
    assert_eq!(one("12 @ 10"), "ans = 6"); // xor
    assert_eq!(one("1 << 4"), "ans = 16");
    assert_eq!(one("256 >> 2"), "ans = 64");
}

#[test]
fn comparisons_and_logic() {
    assert_eq!(one("3 < 5"), "ans = 1");
    assert_eq!(one("3 > 5"), "ans = 0");
    assert_eq!(one("3 == 3"), "ans = 1");
    assert_eq!(one("3 != 3"), "ans = 0");
    assert_eq!(one("3 <= 3"), "ans = 1");
    assert_eq!(one("1 && 0"), "ans = 0");
    assert_eq!(one("1 || 0"), "ans = 1");
}

#[test]
fn precedence() {
    // unary minus vs power: -3^2 = -(3^2) = -9
    assert_eq!(one("-3^2"), "ans = -9");
    // power right-assoc: 2^3^2 = 2^9 = 512
    assert_eq!(one("2^3^2"), "ans = 512");
    assert_eq!(one("2+3*4"), "ans = 14");
}

#[test]
fn constants_and_pi() {
    let out = one("2*pi");
    assert!(out.starts_with("ans = 6.283185307"), "got {out}");
}

#[test]
fn ans_variable_and_shortcuts() {
    let mut e = Engine::new();
    assert_eq!(run(&mut e, "4+5*4"), "ans = 24");
    assert_eq!(run(&mut e, "ans / 2"), "ans = 12");
    // leading operator inserts ans
    assert_eq!(run(&mut e, "+8"), "ans = 20");
    // "--" becomes "ans -"
    assert_eq!(run(&mut e, "--5"), "ans = 15");
}

#[test]
fn variables() {
    let mut e = Engine::new();
    assert_eq!(run(&mut e, "m = 25"), "m = 25");
    assert_eq!(run(&mut e, "m + 5"), "ans = 30");
}

#[test]
fn semicolon_suppresses() {
    // "m = 5; m+2" -> only ans = 7 visible
    let mut e = Engine::new();
    let outs = e.run_line("m = 5; m+2");
    let visible: Vec<String> = outs.iter().filter_map(|ev| e.format_eval(ev)).collect();
    assert_eq!(visible, vec!["ans = 7".to_string()]);
}

#[test]
fn user_functions() {
    let mut e = Engine::new();
    run(&mut e, "par(x,y) = x*y/(x+y)");
    assert_eq!(run(&mut e, "par(10,10)"), "ans = 5");
}

#[test]
fn builtin_functions() {
    assert_eq!(one("sqrt(16)"), "ans = 4");
    assert_eq!(one("abs(-7)"), "ans = 7");
    assert_eq!(one("floor(3.7)"), "ans = 3");
    assert_eq!(one("ceil(3.2)"), "ans = 4");
    assert_eq!(one("round(3.5)"), "ans = 4");
    assert_eq!(one("log(1000)"), "ans = 3");
    assert_eq!(one("log2(8)"), "ans = 3");
    assert_eq!(one("exp(0)"), "ans = 1");
    assert_eq!(one("ln(1)"), "ans = 0");
    assert_eq!(one("mod(7,3)"), "ans = 1");
    assert_eq!(one("max(3,9)"), "ans = 9");
    assert_eq!(one("min(3,9)"), "ans = 3");
}

#[test]
fn trig_radians_and_degrees() {
    // sin(0) = 0
    assert_eq!(one("sin(0)"), "ans = 0");
    let mut e = Engine::new();
    run(&mut e, "mode deg");
    let s = run(&mut e, "sin(30)");
    // sin 30deg = 0.5
    assert!(s.starts_with("ans = 0.5"), "got {s}");
    let c = run(&mut e, "cos(60)");
    assert!(c.starts_with("ans = 0.5"), "got {c}");
}

#[test]
fn si_prefixes() {
    assert_eq!(one("5M + 100k"), "ans = 5100000");
    assert_eq!(one("1k"), "ans = 1000");
}

#[test]
fn hex_and_binary_input() {
    assert_eq!(one("15 + 0x5 + 0b101"), "ans = 25");
}

#[test]
fn base_display() {
    let mut e = Engine::new();
    run(&mut e, "display hex");
    assert_eq!(run(&mut e, "255"), "ans = 0xFF");
    run(&mut e, "display bin");
    assert_eq!(run(&mut e, "5"), "ans = 0b101");
    run(&mut e, "display dec");
    assert_eq!(run(&mut e, "0xFF"), "ans = 255");
}

#[test]
fn units_conversion() {
    assert_eq!(one("10 in->cm"), "ans = 25.4");
    assert_eq!(one("in -> cm"), "ans = 2.54");
    assert_eq!(one("20*5 in -> cm"), "ans = 254");
    let mph = one("70 mi/h->m/s");
    assert!(mph.starts_with("ans = 31.2928"), "got {mph}");
    assert_eq!(one("50 N*m->millijoules"), "ans = 50000");
    let acre = one("10000 m^2->acre");
    assert!(acre.starts_with("ans = 2.4710538"), "got {acre}");
}

#[test]
fn custom_unit() {
    let mut e = Engine::new();
    assert_eq!(run(&mut e, "unit furlong2 = 201.168 meter"), "new unit created!");
    assert_eq!(run(&mut e, "1 furlong2 -> meter"), "ans = 201.168");
}

#[test]
fn incompatible_units_error() {
    let out = one("10 m -> s");
    assert!(out.starts_with("Error:"), "got {out}");
}

#[test]
fn commands_del_and_list() {
    let mut e = Engine::new();
    run(&mut e, "x = 5");
    run(&mut e, "del x");
    let out = run(&mut e, "x");
    assert!(out.starts_with("Error:"), "got {out}");
}

#[test]
fn high_precision() {
    // sqrt(2) to many digits — at least 50 correct digits
    let mut e = Engine::new();
    run(&mut e, "sigfigs 60");
    let out = run(&mut e, "sqrt(2)");
    assert!(
        out.starts_with("ans = 1.4142135623730950488016887242096980785696718753769480731766"),
        "got {out}"
    );
}

#[test]
fn scientific_notation_modes() {
    let mut e = Engine::new();
    run(&mut e, "scimode always");
    let out = run(&mut e, "123.88");
    assert!(out.starts_with("ans = 1.2388e2"), "got {out}");
}

#[test]
fn factorial_large() {
    // 10! = 3628800
    assert_eq!(one("10!"), "ans = 3628800");
}

#[test]
fn theme_command() {
    let mut e = Engine::new();
    // default is dark
    assert_eq!(e.theme, "dark");
    let dark = e.palette();

    // switch to a light theme
    assert_eq!(run(&mut e, "theme light"), "theme set to light");
    assert_eq!(e.theme, "light");
    assert_ne!(e.palette().background, dark.background);

    // unknown theme is an error and does not change the current theme
    let out = run(&mut e, "theme nope");
    assert!(out.starts_with("Error:"), "got {out}");
    assert_eq!(e.theme, "light");

    // listing shows the known themes and marks the current one
    let list = run(&mut e, "theme list");
    assert!(list.contains("dark"));
    assert!(list.contains("light"));
    assert!(list.contains("nord"));
    assert!(list.contains("solarized-light"));
    assert!(list.contains("* light") || list.contains("*  light"));
}

#[test]
fn prompt_layout_command() {
    let mut e = Engine::new();
    // default layout is bottom
    assert_eq!(e.input_layout, InputLayout::Bottom);

    assert_eq!(run(&mut e, "prompt inline"), "prompt layout set to inline");
    assert_eq!(e.input_layout, InputLayout::Inline);

    assert_eq!(run(&mut e, "prompt bottom"), "prompt layout set to bottom");
    assert_eq!(e.input_layout, InputLayout::Bottom);

    // 'layout' is an accepted alias
    assert_eq!(run(&mut e, "layout inline"), "prompt layout set to inline");
    assert_eq!(e.input_layout, InputLayout::Inline);

    // unknown value errors and keeps the current layout
    let out = run(&mut e, "prompt sideways");
    assert!(out.starts_with("Error:"), "got {out}");
    assert_eq!(e.input_layout, InputLayout::Inline);

    // no-arg shows current
    assert!(run(&mut e, "prompt").contains("inline"));
}
