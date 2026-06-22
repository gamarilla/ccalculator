//! Run a script file of expressions/commands. Mirrors the original's
//! command-line interface: `ccalc [inputfile] [-o outputfile] [-q]`.

use std::fs;

use ccalc_core::{Engine, Eval};

pub fn run_file(
    engine: &mut Engine,
    path: &str,
    output: Option<&str>,
    quiet: bool,
) -> anyhow::Result<()> {
    let src = fs::read_to_string(path)?;
    let mut session = String::new();

    for line in src.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        for ev in engine.run_line(line) {
            if matches!(ev, Eval::Exit) {
                break;
            }
            if let Some(s) = engine.format_eval(&ev) {
                session.push_str(&s);
                session.push('\n');
            }
        }
    }

    if let Some(out) = output {
        fs::write(out, &session)?;
    }
    if !quiet {
        print!("{session}");
    }
    Ok(())
}
