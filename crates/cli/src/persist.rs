//! Loading and saving persistent calculator state (settings, user definitions,
//! and input history) between sessions.

use std::fs;
use std::path::PathBuf;

use ccalc_core::{Angle, DisplaySettings, Engine, InputLayout};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SessionConfig {
    settings: DisplaySettings,
    angle: String,
    european: bool,
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    layout: Option<String>,
}

fn dirs() -> Option<PathBuf> {
    let pd = directories::ProjectDirs::from("com", "ZoeSoft", "ccalc")?;
    let dir = pd.config_dir().to_path_buf();
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn state_path() -> Option<PathBuf> {
    Some(dirs()?.join("state.json"))
}

fn defs_path() -> Option<PathBuf> {
    Some(dirs()?.join("ccalc_functions.txt"))
}

fn history_path() -> Option<PathBuf> {
    Some(dirs()?.join("history.txt"))
}

/// Load saved settings and replay saved definitions into the engine.
pub fn load_into(engine: &mut Engine) {
    if let Some(p) = state_path() {
        if let Ok(txt) = fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<SessionConfig>(&txt) {
                engine.settings = cfg.settings;
                engine.angle = if cfg.angle.eq_ignore_ascii_case("deg") {
                    Angle::Deg
                } else {
                    Angle::Rad
                };
                engine.set_european(cfg.european);
                if let Some(t) = cfg.theme {
                    if ccalc_core::theme::find(&t).is_some() {
                        engine.theme = t;
                    }
                }
                engine.input_layout = match cfg.layout.as_deref() {
                    Some("inline") => InputLayout::Inline,
                    _ => InputLayout::Bottom,
                };
            }
        }
    }
    if let Some(p) = defs_path() {
        if let Ok(txt) = fs::read_to_string(&p) {
            for line in txt.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // replay silently; ignore individual failures
                let _ = engine.run_line(line);
            }
        }
    }
}

/// Persist current settings and user definitions.
pub fn save(engine: &Engine) {
    if let Some(p) = state_path() {
        let cfg = SessionConfig {
            settings: engine.settings.clone(),
            angle: match engine.angle {
                Angle::Deg => "deg".to_string(),
                Angle::Rad => "rad".to_string(),
            },
            european: engine.european,
            theme: Some(engine.theme.clone()),
            layout: Some(match engine.input_layout {
                InputLayout::Inline => "inline".to_string(),
                InputLayout::Bottom => "bottom".to_string(),
            }),
        };
        if let Ok(txt) = serde_json::to_string_pretty(&cfg) {
            let _ = fs::write(p, txt);
        }
    }
    if let Some(p) = defs_path() {
        let header = "# Console Calculator user definitions (auto-generated)\n";
        let _ = fs::write(p, format!("{header}{}\n", engine.definitions_script()));
    }
}

/// Load input history (most recent last).
pub fn load_history() -> Vec<String> {
    history_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|t| t.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

/// Save input history, keeping at most `max` recent entries.
pub fn save_history(history: &[String], max: usize) {
    if let Some(p) = history_path() {
        let start = history.len().saturating_sub(max);
        let txt = history[start..].join("\n");
        let _ = fs::write(p, txt);
    }
}
