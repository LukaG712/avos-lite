use crate::send_rpc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};
use serde_json::json;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const BG_PRIMARY: Color = Color::Rgb(35, 35, 35); // #232323
const BG_WELL: Color = Color::Rgb(26, 26, 26); // #1A1A1A
const BORDER_HAIRLINE: Color = Color::Rgb(121, 120, 120); // #797878
const FG_PRIMARY: Color = Color::Rgb(242, 242, 240); // #F2F2F0
const FG_MUTED: Color = Color::Rgb(150, 150, 148);
const ACCENT_GREEN: Color = Color::Rgb(86, 204, 132);
const ACCENT_AMBER: Color = Color::Rgb(235, 180, 70);
const ACCENT_CYAN: Color = Color::Rgb(104, 204, 232);
const ACCENT_RED: Color = Color::Rgb(232, 96, 96);

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const AVOS_BANNER: &[&str] = &[
    "   █████╗ ██╗   ██╗ ██████╗ ███████╗",
    "  ██╔══██╗██║   ██║██╔═══██╗██╔════╝",
    "  ███████║██║   ██║██║   ██║███████╗",
    "  ██╔══██║╚██╗ ██╔╝██║   ██║╚════██║",
    "  ██║  ██║ ╚████╔╝ ╚██████╔╝███████║",
    "  ╚═╝  ╚═╝  ╚═══╝   ╚═════╝ ╚══════╝",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenMode {
    StartupAnimation,
    Welcome,
    Workspace,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub tag: String,
    pub color: Color,
    pub message: String,
}

pub struct LiteTuiApp {
    pub socket_path: PathBuf,
    pub daemon_connected: bool,
    pub autonomy_level: u8,
    pub screen: ScreenMode,
    pub show_sidebar: bool,
    pub boot_step: usize,
    pub tick: usize,
    pub input: String,
    pub cursor: usize,
    pub scroll_offset: u16,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub logs: Vec<LogEntry>,
    pub panes: Vec<(String, String)>,
    pub should_quit: bool,
}

impl LiteTuiApp {
    pub fn new(socket_path: PathBuf) -> Self {
        let mut app = Self {
            socket_path,
            daemon_connected: false,
            autonomy_level: 1,
            screen: ScreenMode::StartupAnimation,
            show_sidebar: true,
            boot_step: 0,
            tick: 0,
            input: String::new(),
            cursor: 0,
            scroll_offset: 0,
            history: Vec::new(),
            history_idx: None,
            logs: Vec::new(),
            panes: vec![
                ("lead-droid".into(), "READY".into()),
                ("qa-droid".into(), "READY".into()),
                ("aegis-droid".into(), "ARMED".into()),
            ],
            should_quit: false,
        };
        app.refresh_from_daemon();
        app
    }

    pub fn refresh_from_daemon(&mut self) {
        match send_rpc(&self.socket_path, "system.health", json!({})) {
            Ok(health) => {
                self.daemon_connected = true;
                if let Some(lvl) = health.get("autonomy_level").and_then(|v| v.as_u64()) {
                    self.autonomy_level = lvl as u8;
                }
                if let Ok(pane_res) = send_rpc(&self.socket_path, "pane.list", json!({}))
                    && let Some(items) = pane_res.get("items").and_then(|v| v.as_array())
                    && !items.is_empty()
                {
                    let mut live: Vec<(String, String)> = items
                        .iter()
                        .filter_map(|p| {
                            let name = p.get("name")?.as_str()?.to_string();
                            let status = p.get("status")?.as_str()?.to_string();
                            Some((name, status))
                        })
                        .collect();
                    live.sort_by(|a, b| a.0.cmp(&b.0));
                    self.panes = live;
                }
            }
            Err(_) => {
                self.daemon_connected = false;
            }
        }
    }

    pub fn push_log(&mut self, tag: impl Into<String>, color: Color, message: impl Into<String>) {
        self.logs.push(LogEntry {
            tag: tag.into(),
            color,
            message: message.into(),
        });
        self.scroll_offset = self.logs.len().saturating_sub(12) as u16;
        if self.screen == ScreenMode::Welcome {
            self.screen = ScreenMode::Workspace;
        }
    }

    pub fn submit_input(&mut self) {
        let raw = self.input.trim().to_string();
        if raw.is_empty() {
            return;
        }
        self.history.push(raw.clone());
        self.history_idx = None;
        self.input.clear();
        self.cursor = 0;

        if raw.starts_with('/') {
            let parts: Vec<&str> = raw.split_whitespace().collect();
            match parts.first().copied() {
                Some("/quit") | Some("/exit") | Some("/q") => self.should_quit = true,
                Some("/clear") => {
                    self.logs.clear();
                    self.scroll_offset = 0;
                    self.push_log("SYSTEM", ACCENT_CYAN, "Cleared mission feed.");
                }
                Some("/welcome") => self.screen = ScreenMode::Welcome,
                Some("/sidebar") => self.show_sidebar = !self.show_sidebar,
                Some("/help") => self.push_log(
                    "HELP",
                    ACCENT_CYAN,
                    "Commands: /mission <prompt> | /autonomy <0-4> | /panes | /loops | /health | /sidebar | /welcome | /clear | /quit",
                ),
                Some("/panes") => {
                    self.refresh_from_daemon();
                    self.push_log("PANES", ACCENT_CYAN, format!("Active panes: {:?}", self.panes));
                }
                Some("/loops") => match send_rpc(&self.socket_path, "loop.list", json!({})) {
                    Ok(res) => {
                        self.daemon_connected = true;
                        self.push_log("LOOPS", ACCENT_GREEN, format!("{res}"));
                    }
                    Err(e) => {
                        self.daemon_connected = false;
                        self.push_log("LOOPS", ACCENT_AMBER, format!("Daemon offline ({e})"));
                    }
                },
                Some("/health") => match send_rpc(&self.socket_path, "system.health", json!({})) {
                    Ok(res) => {
                        self.daemon_connected = true;
                        self.push_log("DAEMON", ACCENT_GREEN, format!("Health OK: {res}"));
                    }
                    Err(e) => {
                        self.daemon_connected = false;
                        self.push_log("DAEMON", ACCENT_AMBER, format!("Standalone ({e})"));
                    }
                },
                Some("/autonomy") => {
                    if let Some(Ok(lvl)) = parts.get(1).map(|s| s.parse::<u8>())
                        && lvl <= 4
                    {
                        self.autonomy_level = lvl;
                        let _ = send_rpc(&self.socket_path, "autonomy.set", json!({ "level": lvl }));
                        self.push_log("POLICY", ACCENT_GREEN, format!("Autonomy level set to L{lvl}"));
                    } else {
                        self.push_log("ERROR", ACCENT_RED, "Usage: /autonomy <0-4>");
                    }
                }
                Some("/mission") => {
                    let prompt = raw.trim_start_matches("/mission").trim();
                    if prompt.is_empty() {
                        self.push_log("ERROR", ACCENT_RED, "Usage: /mission <prompt>");
                    } else {
                        self.dispatch_mission(prompt);
                    }
                }
                Some(other) => {
                    self.push_log("ERROR", ACCENT_RED, format!("Unknown command: {other}. Try /help"));
                }
                None => {}
            }
        } else {
            self.dispatch_mission(&raw);
        }
    }

    fn dispatch_mission(&mut self, prompt: &str) {
        self.push_log("USER", FG_PRIMARY, prompt.to_string());
        match send_rpc(
            &self.socket_path,
            "mission.create",
            json!({ "prompt": prompt }),
        ) {
            Ok(res) => {
                self.daemon_connected = true;
                self.panes[0].1 = "WORKING".into();
                self.push_log("MISSION", ACCENT_GREEN, format!("Dispatched: {res}"));
            }
            Err(_) => {
                self.daemon_connected = false;
                self.panes[0].1 = "LOCAL".into();
                self.push_log(
                    "MISSION",
                    ACCENT_AMBER,
                    format!("Queued locally: \"{prompt}\""),
                );
            }
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(
            Block::default().style(Style::default().bg(BG_PRIMARY)),
            area,
        );

        if self.screen == ScreenMode::StartupAnimation {
            self.render_startup_animation(frame, area);
            return;
        }

        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        let daemon_badge = if self.daemon_connected {
            Span::styled(" ● DAEMON CONNECTED ", Style::default().fg(ACCENT_GREEN))
        } else {
            Span::styled(" ○ STANDALONE ", Style::default().fg(ACCENT_AMBER))
        };
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " AVOS-LITE ",
                Style::default()
                    .fg(BG_WELL)
                    .bg(FG_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" // LOCAL_LITE ", Style::default().fg(FG_MUTED)),
            daemon_badge,
            Span::styled(
                format!(" │ AUTONOMY L{} ", self.autonomy_level),
                Style::default()
                    .fg(ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_HAIRLINE))
                .style(Style::default().bg(BG_WELL)),
        );
        frame.render_widget(header, root[0]);

        if self.show_sidebar && area.width >= 70 {
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
                .split(root[1]);
            self.render_main_pane(frame, body[0]);
            self.render_sidebar(frame, body[1]);
        } else {
            self.render_main_pane(frame, root[1]);
        }

        let cursor_pos = self.cursor.min(self.input.len());
        let (before, after) = self.input.split_at(cursor_pos);
        let prompt = Paragraph::new(Line::from(vec![
            Span::styled(
                " ❯ ",
                Style::default()
                    .fg(ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(before.to_string(), Style::default().fg(FG_PRIMARY)),
            Span::styled("█", Style::default().fg(ACCENT_CYAN)),
            Span::styled(after.to_string(), Style::default().fg(FG_PRIMARY)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT_GREEN))
                .title(Span::styled(
                    " COMMAND / MISSION PROMPT ",
                    Style::default().fg(FG_MUTED),
                ))
                .style(Style::default().bg(BG_WELL)),
        );
        frame.render_widget(prompt, root[2]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" Enter", Style::default().fg(FG_PRIMARY)),
            Span::styled(": Run  ", Style::default().fg(FG_MUTED)),
            Span::styled("F2", Style::default().fg(FG_PRIMARY)),
            Span::styled(": Sidebar  ", Style::default().fg(FG_MUTED)),
            Span::styled("↑/↓", Style::default().fg(FG_PRIMARY)),
            Span::styled(": History  ", Style::default().fg(FG_MUTED)),
            Span::styled("Ctrl+L", Style::default().fg(FG_PRIMARY)),
            Span::styled(": Clear  ", Style::default().fg(FG_MUTED)),
            Span::styled("Esc/Ctrl+C", Style::default().fg(FG_PRIMARY)),
            Span::styled(": Quit", Style::default().fg(FG_MUTED)),
        ]))
        .style(Style::default().bg(BG_PRIMARY));
        frame.render_widget(footer, root[3]);
    }

    fn render_startup_animation(&self, frame: &mut Frame, area: Rect) {
        let outer = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_HAIRLINE))
            .style(Style::default().bg(BG_WELL));
        let inner = outer.inner(area);
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(2),
                Constraint::Length(8),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(2),
            ])
            .split(inner);

        let visible_lines = (self.tick / 2 + 1).min(AVOS_BANNER.len());
        let banner_lines: Vec<Line> = AVOS_BANNER
            .iter()
            .take(visible_lines)
            .map(|l| {
                Line::from(Span::styled(
                    *l,
                    Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                ))
            })
            .collect();
        frame.render_widget(
            Paragraph::new(banner_lines).alignment(Alignment::Center),
            chunks[1],
        );

        let ratio = ((self.boot_step + 1).min(3) as f64) / 3.0;
        let spin = SPINNER[self.tick % SPINNER.len()];
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDER_HAIRLINE))
                    .title(Span::styled(
                        format!(" {spin} BOOTING AVOS-LITE "),
                        Style::default().fg(ACCENT_CYAN),
                    )),
            )
            .gauge_style(Style::default().fg(ACCENT_GREEN).bg(BG_PRIMARY))
            .ratio(ratio);
        frame.render_widget(gauge, centered_rect(60, 3, chunks[2]));

        let steps = [
            "Ed25519 Offline License Verifier",
            "avos-daemon Local Socket (/tmp/avos.sock)",
            "OSC 7501 Multi-Pane Multiplexer (ac)",
        ];
        let step_lines: Vec<Line> = steps
            .iter()
            .enumerate()
            .map(|(idx, label)| {
                let (mark, color) = if idx < self.boot_step {
                    ("✓", ACCENT_GREEN)
                } else if idx == self.boot_step {
                    (spin, ACCENT_CYAN)
                } else {
                    ("·", FG_MUTED)
                };
                Line::from(vec![
                    Span::styled(format!("  [{mark}] "), Style::default().fg(color)),
                    Span::styled(*label, Style::default().fg(FG_PRIMARY)),
                ])
            })
            .collect();
        frame.render_widget(Paragraph::new(step_lines), centered_rect(60, 5, chunks[3]));
    }

    fn render_main_pane(&self, frame: &mut Frame, area: Rect) {
        if self.screen == ScreenMode::Welcome && self.logs.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_HAIRLINE))
                .title(Span::styled(" WELCOME ", Style::default().fg(FG_PRIMARY)))
                .style(Style::default().bg(BG_WELL));
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let mut lines: Vec<Line> = AVOS_BANNER
                .iter()
                .map(|l| {
                    Line::from(Span::styled(
                        *l,
                        Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
                    ))
                })
                .collect();
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "AVOS-LITE // OPEN-CORE DROID TUI (OSC 7501)",
                Style::default()
                    .fg(ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Type a mission prompt or /help to get started.",
                Style::default().fg(FG_MUTED),
            )));
            frame.render_widget(
                Paragraph::new(lines).alignment(Alignment::Center),
                centered_rect(90, 12, inner),
            );
            return;
        }

        let items: Vec<Line> = self
            .logs
            .iter()
            .map(|entry| {
                Line::from(vec![
                    Span::styled(
                        format!(" [{}] ", entry.tag),
                        Style::default()
                            .fg(entry.color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(entry.message.clone(), Style::default().fg(FG_PRIMARY)),
                ])
            })
            .collect();
        let para = Paragraph::new(items).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_HAIRLINE))
                .title(Span::styled(
                    " MISSION STREAM ",
                    Style::default().fg(FG_PRIMARY),
                ))
                .style(Style::default().bg(BG_WELL)),
        );
        frame.render_widget(para, area);
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .panes
            .iter()
            .map(|(name, status)| {
                let color = match status.as_str() {
                    "WORKING" => ACCENT_GREEN,
                    "ARMED" => ACCENT_CYAN,
                    "LOCAL" => ACCENT_AMBER,
                    _ => FG_MUTED,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {name} "), Style::default().fg(FG_PRIMARY)),
                    Span::styled(status.clone(), Style::default().fg(color)),
                ]))
            })
            .collect();
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_HAIRLINE))
                .title(Span::styled(
                    " AGENT PANES ",
                    Style::default().fg(FG_PRIMARY),
                ))
                .style(Style::default().bg(BG_WELL)),
        );
        frame.render_widget(list, area);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}

pub fn run_tui(socket_path: &Path) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_tui_loop(&mut terminal, socket_path);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

fn run_tui_loop<B: Backend>(terminal: &mut Terminal<B>, socket_path: &Path) -> io::Result<()> {
    let mut app = LiteTuiApp::new(socket_path.to_path_buf());
    let boot_start = Instant::now();

    while !app.should_quit {
        if app.screen == ScreenMode::StartupAnimation {
            app.boot_step = (boot_start.elapsed().as_millis() as usize) / 300;
            if app.boot_step >= 3 {
                app.screen = ScreenMode::Welcome;
            }
        }
        terminal.draw(|f| app.render(f))?;

        if event::poll(Duration::from_millis(60))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if app.screen == ScreenMode::StartupAnimation {
                app.screen = ScreenMode::Welcome;
                continue;
            }
            if key.code == KeyCode::Esc
                || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                app.should_quit = true;
                continue;
            }
            if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.logs.clear();
                continue;
            }
            match key.code {
                KeyCode::F(2) => app.show_sidebar = !app.show_sidebar,
                KeyCode::Enter => app.submit_input(),
                KeyCode::PageUp => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(3);
                }
                KeyCode::PageDown => {
                    app.scroll_offset = app.scroll_offset.saturating_add(3);
                }
                KeyCode::Left => {
                    if app.cursor > 0 {
                        let mut pos = app.cursor - 1;
                        while pos > 0 && !app.input.is_char_boundary(pos) {
                            pos -= 1;
                        }
                        app.cursor = pos;
                    }
                }
                KeyCode::Right => {
                    if app.cursor < app.input.len() {
                        let mut pos = app.cursor + 1;
                        while pos < app.input.len() && !app.input.is_char_boundary(pos) {
                            pos += 1;
                        }
                        app.cursor = pos;
                    }
                }
                KeyCode::Home => app.cursor = 0,
                KeyCode::End => app.cursor = app.input.len(),
                KeyCode::Backspace => {
                    if app.cursor > 0 {
                        let mut prev = app.cursor - 1;
                        while prev > 0 && !app.input.is_char_boundary(prev) {
                            prev -= 1;
                        }
                        app.input.remove(prev);
                        app.cursor = prev;
                    }
                }
                KeyCode::Up => {
                    if !app.history.is_empty() {
                        let next = match app.history_idx {
                            None => app.history.len() - 1,
                            Some(i) => i.saturating_sub(1),
                        };
                        app.history_idx = Some(next);
                        app.input = app.history[next].clone();
                        app.cursor = app.input.len();
                    }
                }
                KeyCode::Down => {
                    if let Some(i) = app.history_idx {
                        if i + 1 < app.history.len() {
                            app.history_idx = Some(i + 1);
                            app.input = app.history[i + 1].clone();
                            app.cursor = app.input.len();
                        } else {
                            app.history_idx = None;
                            app.input.clear();
                            app.cursor = 0;
                        }
                    }
                }
                KeyCode::Char(c) => {
                    app.input.insert(app.cursor, c);
                    app.cursor += c.len_utf8();
                }
                _ => {}
            }
        }
        app.tick = app.tick.wrapping_add(1);
        if app.tick.is_multiple_of(16) && app.screen != ScreenMode::StartupAnimation {
            app.refresh_from_daemon();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn lite_tui_renders_startup_welcome_and_workspace() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = LiteTuiApp::new(PathBuf::from("/tmp/avos_lite_nonexistent.sock"));

        terminal.draw(|f| app.render(f)).unwrap();
        app.screen = ScreenMode::Welcome;
        terminal.draw(|f| app.render(f)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("WELCOME"));

        app.input = "/autonomy 2".into();
        app.submit_input();
        assert_eq!(app.autonomy_level, 2);
        assert_eq!(app.screen, ScreenMode::Workspace);

        app.input = "/panes".into();
        app.submit_input();
        assert!(app.logs.iter().any(|l| l.tag == "PANES"));
    }
}
