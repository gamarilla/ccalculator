//! Console Calculator command-line entry point.

mod mcp;
mod persist;
mod repl;
mod script;
mod tui;

use ccalc_core::Engine;

const HELP: &str = "\
Console Calculator (ccalc)

USAGE:
    ccalc                     Launch the interactive terminal UI
    ccalc --repl              Launch a simple line REPL (pipe-friendly)
    ccalc -e <expr>           Evaluate an expression and print the result
    ccalc <script> [-o out] [-q]
                              Run a script file (optionally write output, quiet)
    ccalc --mcp               Run as an MCP server over stdio (for AI assistants)

OPTIONS:
    -e, --eval <expr>   Evaluate <expr> and exit
    -o <file>           Write script session output to <file>
    -q                  Quiet: do not print script output to stdout
    --no-store          Do not load or save persistent state
    -h, --help          Show this help
    -V, --version       Show version
";

fn main() {
    if let Err(e) = real_main() {
        eprintln!("ccalc: {e}");
        std::process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut mode_mcp = false;
    let mut mode_repl = false;
    let mut eval_expr: Option<String> = None;
    let mut script_file: Option<String> = None;
    let mut output: Option<String> = None;
    let mut quiet = false;
    let mut no_store = false;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("ccalc {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--mcp" => mode_mcp = true,
            "--repl" => mode_repl = true,
            "--tui" => {}
            "--no-store" => no_store = true,
            "-q" => quiet = true,
            "-e" | "--eval" => {
                i += 1;
                eval_expr = Some(args.get(i).cloned().unwrap_or_default());
            }
            "-o" => {
                i += 1;
                output = args.get(i).cloned();
            }
            other => {
                if other.starts_with('-') {
                    anyhow::bail!("unknown option '{other}' (try --help)");
                }
                script_file = Some(other.to_string());
            }
        }
        i += 1;
    }

    if mode_mcp {
        return mcp::serve();
    }

    let mut engine = Engine::new();
    if !no_store {
        persist::load_into(&mut engine);
    }

    if let Some(expr) = eval_expr {
        for ev in engine.run_line(&expr) {
            if let Some(s) = engine.format_eval(&ev) {
                println!("{s}");
            }
        }
        return Ok(());
    }

    if let Some(file) = script_file {
        script::run_file(&mut engine, &file, output.as_deref(), quiet)?;
        return Ok(());
    }

    if mode_repl {
        repl::run(&mut engine)?;
    } else {
        tui::run(&mut engine)?;
    }

    if !no_store {
        persist::save(&engine);
    }
    Ok(())
}
