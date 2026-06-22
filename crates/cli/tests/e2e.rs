//! End-to-end tests that drive the compiled `ccalc` binary: one-shot eval,
//! script execution, and the MCP stdio server.

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_ccalc"));
    // Isolate persistent state into a throwaway dir.
    let tmp = std::env::temp_dir().join(format!("ccalc-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    c.env("HOME", &tmp);
    c.env("XDG_CONFIG_HOME", &tmp);
    c
}

fn run_stdin(args: &[&str], input: &str) -> String {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ccalc");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn eval_flag() {
    let out = bin()
        .args(["--no-store", "-e", "4+5*4"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ans = 24");
}

#[test]
fn repl_pipe() {
    let out = run_stdin(&["--no-store", "--repl"], "x=10\nx*x\nexit\n");
    assert!(out.contains("x = 10"), "got: {out}");
    assert!(out.contains("ans = 100"), "got: {out}");
}

#[test]
fn script_file() {
    let tmp = std::env::temp_dir().join(format!("ccalc-script-{}.txt", std::process::id()));
    std::fs::write(
        &tmp,
        "# a script\na = 3;\nb = 4;\nsqrt(a^2 + b^2)\n",
    )
    .unwrap();
    let out = bin()
        .args(["--no-store", tmp.to_str().unwrap()])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    // only the final expression prints (the two assignments are suppressed by ;)
    assert_eq!(s.trim(), "ans = 5");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn mcp_protocol() {
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"evaluate","arguments":{"expression":"3^3"}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"evaluate","arguments":{"expression":"x = 21"}}}"#,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"evaluate","arguments":{"expression":"x * 2"}}}"#,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"convert_units","arguments":{"value":"10","from":"in","to":"cm"}}}"#,
    ]
    .join("\n");

    let out = run_stdin(&["--mcp"], &format!("{requests}\n"));
    let lines: Vec<serde_json::Value> = out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("valid json response"))
        .collect();

    // initialize
    assert_eq!(lines[0]["result"]["serverInfo"]["name"], "ccalc");
    // tools/list contains evaluate
    let tools = lines[1]["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "evaluate"));
    // 3^3
    assert_eq!(lines[2]["result"]["content"][0]["text"], "ans = 27");
    // session state: x=21 then x*2 = 42
    assert_eq!(lines[4]["result"]["content"][0]["text"], "ans = 42");
    // unit conversion
    assert_eq!(lines[5]["result"]["content"][0]["text"], "ans = 25.4");
}

#[test]
fn persistence_round_trip() {
    let tmp = std::env::temp_dir().join(format!("ccalc-persist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // First session: define a function and a variable (state should be saved).
    let mut c1 = Command::new(env!("CARGO_BIN_EXE_ccalc"));
    c1.env("HOME", &tmp).env("XDG_CONFIG_HOME", &tmp);
    let out1 = c1
        .args(["--repl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map(|mut ch| {
            ch.stdin
                .as_mut()
                .unwrap()
                .write_all(b"sq(n) = n*n\nk = 7\nexit\n")
                .unwrap();
            ch.wait_with_output().unwrap()
        })
        .unwrap();
    assert!(out1.status.success());

    // Second session: the function and variable should still be available.
    let mut c2 = Command::new(env!("CARGO_BIN_EXE_ccalc"));
    c2.env("HOME", &tmp).env("XDG_CONFIG_HOME", &tmp);
    let out2 = c2
        .args(["--repl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map(|mut ch| {
            ch.stdin
                .as_mut()
                .unwrap()
                .write_all(b"sq(k)\nexit\n")
                .unwrap();
            ch.wait_with_output().unwrap()
        })
        .unwrap();
    let s = String::from_utf8_lossy(&out2.stdout);
    assert!(s.contains("ans = 49"), "expected sq(7)=49, got: {s}");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn theme_persists_across_sessions() {
    let tmp = std::env::temp_dir().join(format!("ccalc-theme-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let session = |input: &[u8]| -> String {
        let mut c = Command::new(env!("CARGO_BIN_EXE_ccalc"));
        c.env("HOME", &tmp).env("XDG_CONFIG_HOME", &tmp);
        let mut ch = c
            .args(["--repl"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        ch.stdin.as_mut().unwrap().write_all(input).unwrap();
        let out = ch.wait_with_output().unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    session(b"theme solarized-light\nprompt inline\nexit\n");
    let s = session(b"theme\nprompt\nexit\n");
    assert!(
        s.contains("current theme: solarized-light"),
        "theme did not persist, got: {s}"
    );
    assert!(
        s.contains("prompt layout: inline"),
        "prompt layout did not persist, got: {s}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}
