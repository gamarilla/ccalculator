//! Modern terminal UI for Console Calculator (ratatui).
//!
//! A full-screen console: scrollback history of inputs and results, an input
//! line with command-history recall (up/down), and a status bar showing the
//! current base and angle mode. Colors come from the engine's active theme.

use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers,
};
use ratatui::crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::io::{stdout, Stdout};

use ccalc_core::theme::Rgb;
use ccalc_core::{Angle, Base, Engine, Eval, InputLayout, SciMode};

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

#[derive(Clone)]
enum LineKind {
    Prompt,
    Result,
    Error,
    Info,
}

struct App<'e> {
    engine: &'e mut Engine,
    lines: Vec<(LineKind, String)>,
    input: String,
    cursor: usize,
    history: Vec<String>,
    hist_idx: Option<usize>,
    scroll: u16,
    should_quit: bool,
}

pub fn run(engine: &mut Engine) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let history = crate::persist::load_history();
    let mut app = App {
        engine,
        lines: vec![
            (LineKind::Info, "Console Calculator — type an expression, 'help', or 'exit'.".to_string()),
        ],
        input: String::new(),
        cursor: 0,
        history,
        hist_idx: None,
        scroll: 0,
        should_quit: false,
    };

    let res = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    crate::persist::save_history(&app.history, 1000);
    res
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui(f, app))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }
            handle_key(app, key);
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if ctrl => app.should_quit = true,
        KeyCode::Char('d') if ctrl && app.input.is_empty() => app.should_quit = true,
        KeyCode::Char('1') if ctrl => set_base(app, Base::Bin),
        KeyCode::Char('2') if ctrl => set_base(app, Base::Dec),
        KeyCode::Char('3') if ctrl => set_base(app, Base::Hex),
        KeyCode::Char('l') if ctrl => app.lines.clear(),
        KeyCode::Enter => submit(app),
        KeyCode::Backspace => {
            if app.cursor > 0 {
                app.cursor -= 1;
                app.input.remove(app.cursor);
            }
        }
        KeyCode::Delete => {
            if app.cursor < app.input.len() {
                app.input.remove(app.cursor);
            }
        }
        KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
        KeyCode::Right => {
            if app.cursor < app.input.len() {
                app.cursor += 1;
            }
        }
        KeyCode::Home => app.cursor = 0,
        KeyCode::End => app.cursor = app.input.len(),
        KeyCode::Up => recall_prev(app),
        KeyCode::Down => recall_next(app),
        KeyCode::PageUp => app.scroll = app.scroll.saturating_add(5),
        KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(5),
        KeyCode::Char(c) => {
            app.input.insert(app.cursor, c);
            app.cursor += 1;
        }
        _ => {}
    }
}

fn set_base(app: &mut App, base: Base) {
    app.engine.settings.base = base;
    let name = match base {
        Base::Bin => "binary",
        Base::Dec => "decimal",
        Base::Hex => "hexadecimal",
    };
    app.lines.push((LineKind::Info, format!("display base: {name}")));
}

fn submit(app: &mut App) {
    let line = std::mem::take(&mut app.input);
    app.cursor = 0;
    app.hist_idx = None;
    app.scroll = 0;
    if line.trim().is_empty() {
        return;
    }
    app.lines.push((LineKind::Prompt, format!("> {line}")));
    app.history.push(line.clone());

    for ev in app.engine.run_line(&line) {
        match ev {
            Eval::Exit => app.should_quit = true,
            Eval::Clear => app.lines.clear(),
            Eval::Quiet => {}
            ref other => {
                if let Some(s) = app.engine.format_eval(other) {
                    let kind = match other {
                        Eval::Message(m) if m.starts_with("Error:") => LineKind::Error,
                        Eval::Message(_) => LineKind::Info,
                        _ => LineKind::Result,
                    };
                    app.lines.push((kind, s));
                }
            }
        }
    }
}

fn recall_prev(app: &mut App) {
    if app.history.is_empty() {
        return;
    }
    let idx = match app.hist_idx {
        None => app.history.len() - 1,
        Some(0) => 0,
        Some(i) => i - 1,
    };
    app.hist_idx = Some(idx);
    app.input = app.history[idx].clone();
    app.cursor = app.input.len();
}

fn recall_next(app: &mut App) {
    match app.hist_idx {
        None => {}
        Some(i) if i + 1 < app.history.len() => {
            app.hist_idx = Some(i + 1);
            app.input = app.history[i + 1].clone();
            app.cursor = app.input.len();
        }
        Some(_) => {
            app.hist_idx = None;
            app.input.clear();
            app.cursor = 0;
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let pal = app.engine.palette();
    let bg = rgb(pal.background);
    let base_style = Style::default().bg(bg).fg(rgb(pal.foreground));
    let inline = app.engine.input_layout == InputLayout::Inline;

    // Layout: inline hides the bottom input box.
    let constraints: &[Constraint] = if inline {
        &[Constraint::Min(1), Constraint::Length(1)]
    } else {
        &[Constraint::Min(1), Constraint::Length(3), Constraint::Length(1)]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());
    let hist_area = chunks[0];
    let status_area = chunks[chunks.len() - 1];

    // Paint the whole frame in the theme background first.
    f.render_widget(Block::default().style(base_style), f.area());

    let prompt_style = Style::default().fg(rgb(pal.prompt)).add_modifier(Modifier::BOLD);
    let line_style = |kind: &LineKind| match kind {
        LineKind::Prompt => prompt_style,
        LineKind::Result => Style::default().fg(rgb(pal.result)).add_modifier(Modifier::BOLD),
        LineKind::Error => Style::default().fg(rgb(pal.error)),
        LineKind::Info => Style::default().fg(rgb(pal.info)),
    };

    let block = || {
        Block::default()
            .borders(Borders::ALL)
            .title(" Console Calculator ")
            .title_style(prompt_style)
            .border_style(Style::default().fg(rgb(pal.border)))
            .style(base_style)
    };

    if inline {
        render_inline(f, app, hist_area, base_style, bg, prompt_style, &line_style, block());
    } else {
        render_bottom(f, app, &chunks, base_style, bg, prompt_style, &line_style, block());
    }

    // Status bar
    let base = match app.engine.settings.base {
        Base::Dec => "DEC",
        Base::Hex => "HEX",
        Base::Bin => "BIN",
    };
    let angle = match app.engine.angle {
        Angle::Rad => "RAD",
        Angle::Deg => "DEG",
    };
    let sci = match app.engine.settings.sci {
        SciMode::Auto => "auto",
        SciMode::Never => "never",
        SciMode::Always => "sci",
        SciMode::Eng => "eng",
        SciMode::Prefix => "prefix",
        SciMode::Finance => "fin",
    };
    let status = format!(
        " {base} | {angle} | sci:{sci} | sigfigs:{} | {}  —  Ctrl-1/2/3 base · ↑↓ history · Ctrl-C quit ",
        app.engine.settings.sigfigs, app.engine.theme
    );
    let status_bar = Paragraph::new(Span::raw(status))
        .style(Style::default().bg(rgb(pal.status_bg)).fg(rgb(pal.status_fg)));
    f.render_widget(status_bar, status_area);
}

type LineStyler<'a> = dyn Fn(&LineKind) -> Style + 'a;

/// Bottom layout: scrollback above a fixed input box.
#[allow(clippy::too_many_arguments)]
fn render_bottom(
    f: &mut ratatui::Frame,
    app: &App,
    chunks: &[ratatui::layout::Rect],
    base_style: Style,
    bg: Color,
    prompt_style: Style,
    line_style: &LineStyler,
    block: Block,
) {
    let text: Vec<Line> = app
        .lines
        .iter()
        .map(|(kind, s)| Line::from(Span::styled(s.clone(), line_style(kind).bg(bg))))
        .collect();

    let total = text.len() as u16;
    let view_h = chunks[0].height.saturating_sub(2);
    let max_top = total.saturating_sub(view_h);
    let top = max_top.saturating_sub(app.scroll.min(max_top));

    let history = Paragraph::new(text)
        .style(base_style)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((top, 0));
    f.render_widget(history, chunks[0]);

    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", prompt_style),
        Span::styled(&app.input, Style::default().fg(base_style.fg.unwrap_or(Color::Reset))),
    ]))
    .style(base_style)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(base_style.fg.unwrap_or(Color::Reset)))
            .style(base_style),
    );
    f.render_widget(input, chunks[1]);

    let cx = chunks[1].x + 2 + 1 + app.cursor as u16; // border + "> "
    let cy = chunks[1].y + 1;
    f.set_cursor_position((cx, cy));
}

/// Inline layout: the prompt is the last line of the scrollback, original style.
#[allow(clippy::too_many_arguments)]
fn render_inline(
    f: &mut ratatui::Frame,
    app: &App,
    area: ratatui::layout::Rect,
    base_style: Style,
    bg: Color,
    prompt_style: Style,
    line_style: &LineStyler,
    block: Block,
) {
    // Build all content lines plus their character lengths (for wrap math).
    let mut lines: Vec<Line> = Vec::with_capacity(app.lines.len() + 1);
    let mut lens: Vec<usize> = Vec::with_capacity(app.lines.len() + 1);
    for (kind, s) in &app.lines {
        lines.push(Line::from(Span::styled(s.clone(), line_style(kind).bg(bg))));
        lens.push(s.chars().count());
    }
    // The live inline prompt.
    lines.push(Line::from(vec![
        Span::styled("> ", prompt_style),
        Span::styled(&app.input, base_style),
    ]));
    lens.push(2 + app.input.chars().count());

    let inner_w = area.width.saturating_sub(2) as usize;
    let inner_h = area.height.saturating_sub(2) as usize;
    let inner_left = area.x + 1;
    let inner_top = area.y + 1;

    let rows_for = |len: usize| -> usize {
        if inner_w == 0 {
            1
        } else {
            len.div_ceil(inner_w).max(1)
        }
    };

    // Pin to the bottom: keep the most recent lines that fit.
    let mut acc = 0usize;
    let mut start = lines.len();
    for i in (0..lines.len()).rev() {
        let r = rows_for(lens[i]);
        if acc + r > inner_h && acc > 0 {
            break;
        }
        acc += r;
        start = i;
        if acc >= inner_h {
            break;
        }
    }
    let visible: Vec<Line> = lines[start..].to_vec();

    let para = Paragraph::new(visible)
        .style(base_style)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);

    // Cursor: rows occupied by visible lines before the prompt line.
    let mut y_rel = 0usize;
    for &len in &lens[start..lens.len() - 1] {
        y_rel += rows_for(len);
    }
    let col = 2 + app.cursor; // after "> "
    if let (Some(within_row), Some(within_col)) = (col.checked_div(inner_w), col.checked_rem(inner_w))
    {
        let py = inner_top as usize + (y_rel + within_row).min(inner_h.saturating_sub(1));
        let px = inner_left as usize + within_col;
        f.set_cursor_position((px as u16, py as u16));
    }
}
