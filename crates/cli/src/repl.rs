//! A simple line-oriented REPL. Reads expressions from stdin and prints results
//! to stdout. Pipe-friendly (`echo "2+3" | ccalc --repl`) and used as a fallback
//! when a full terminal UI is not desired.

use std::io::{BufRead, IsTerminal, Write};

use ccalc_core::{Engine, Eval};

pub fn run(engine: &mut Engine) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let interactive = stdin.is_terminal();

    if interactive {
        writeln!(stdout, "Console Calculator (type 'exit' to quit, 'help' for commands)")?;
        write!(stdout, "> ")?;
        stdout.flush()?;
    }

    for line in stdin.lock().lines() {
        let line = line?;
        for ev in engine.run_line(&line) {
            match ev {
                Eval::Exit => {
                    return Ok(());
                }
                Eval::Clear => {
                    // best-effort clear
                    print!("\x1b[2J\x1b[H");
                }
                other => {
                    if let Some(s) = engine.format_eval(&other) {
                        writeln!(stdout, "{s}")?;
                    }
                }
            }
        }
        if interactive {
            write!(stdout, "> ")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
