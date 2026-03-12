use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use dashmap::DashMap;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    DefaultTerminal, Frame, TerminalOptions, Viewport,
};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub enum FileStatus {
    Queued,
    Processing,
    Done,
}

pub struct TraceUiState {
    pub totalTasks: usize,
    pub totalFiles: usize,
    pub completedTasks: Arc<AtomicUsize>,
    pub chunkCount: Arc<AtomicUsize>,
    pub dupCount: Arc<AtomicUsize>,
    pub dupSize: Arc<AtomicUsize>,
    pub fileStats: Arc<DashMap<String, FileStatus>>,
    pub isDone: Arc<AtomicBool>,
    pub chunkerLabel: String,
    pub hasherLabel: String,
    pub numWorkers: usize,
}

pub fn init(numFiles: usize) -> DefaultTerminal {
    let height = (numFiles as u16 + 5).min(20);
    ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(height),
    })
}

pub fn run(mut terminal: DefaultTerminal, state: Arc<TraceUiState>) {
    let mut tick: usize = 0;
    loop {
        let done = state.isDone.load(Ordering::Relaxed);
        terminal.draw(|f| draw(f, &state, tick)).unwrap();
        if done {
            break;
        }
        tick = tick.wrapping_add(1);
        thread::sleep(Duration::from_millis(80));
    }
    ratatui::restore();
    // Move cursor below the inline viewport so subsequent output is not overwritten
    println!();
}

fn fmtSize(bytes: usize) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.1} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.0} KiB", b / KIB)
    } else {
        format!("{} B", bytes)
    }
}

fn draw(frame: &mut Frame, state: &TraceUiState, tick: usize) {
    let area = frame.area();

    let layout = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Min(1),    // file list
        Constraint::Length(1), // stats
        Constraint::Length(1), // gauge
    ])
    .split(area);

    // ── Header ───────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " chunkIQ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(state.chunkerLabel.as_str(), Style::default().fg(Color::Yellow)),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(state.hasherLabel.as_str(), Style::default().fg(Color::Yellow)),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} workers", state.numWorkers),
                Style::default().fg(Color::DarkGray),
            ),
        ])),
        layout[0],
    );

    // ── File List (auto-scrolling) ────────────────────────────────
    {
        let listArea = layout[1];
        // Inner width: subtract 2 borders + 1 left pad + 1 icon + 2 spacing
        let innerWidth = listArea.width.saturating_sub(6) as usize;
        // Rows available inside the block borders
        let maxVisible = listArea.height.saturating_sub(2) as usize;

        let mut entries: Vec<(String, u8)> = state
            .fileStats
            .iter()
            .map(|e| {
                let code = match *e.value() {
                    FileStatus::Done => 2,
                    FileStatus::Processing => 1,
                    FileStatus::Queued => 0,
                };
                (e.key().clone(), code)
            })
            .collect();
        // Done → Processing → Queued; alphabetical within each group
        entries.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        // Auto-scroll: anchor view so the first active file is visible,
        // with up to 2 completed files shown above it as context.
        let firstActive = entries
            .iter()
            .position(|(_, c)| *c < 2)
            .unwrap_or(0);
        let scrollStart = firstActive.saturating_sub(2);

        // Reserve rows for overflow indicators as needed, then compute the window.
        let aboveCount = scrollStart;
        let rowsUsedByAbove = if aboveCount > 0 { 1 } else { 0 };
        let dataRows = maxVisible.saturating_sub(rowsUsedByAbove);
        let tentativeEnd = (scrollStart + dataRows).min(entries.len());
        let belowCount = entries.len().saturating_sub(tentativeEnd);
        let rowsUsedByBelow = if belowCount > 0 { 1 } else { 0 };
        let dataRows = maxVisible.saturating_sub(rowsUsedByAbove + rowsUsedByBelow);
        let endIdx = (scrollStart + dataRows).min(entries.len());
        let belowCount = entries.len().saturating_sub(endIdx);

        let mut items: Vec<ListItem> = Vec::new();

        if aboveCount > 0 {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("   ↑ {} above", aboveCount),
                Style::default().fg(Color::DarkGray),
            ))));
        }

        for (path, code) in &entries[scrollStart..endIdx] {
            let (icon, iconStyle, nameStyle) = match code {
                2 => (
                    "✓",
                    Style::default().fg(Color::Green),
                    Style::default().fg(Color::White),
                ),
                1 => (
                    SPINNER[tick % SPINNER.len()],
                    Style::default().fg(Color::Yellow),
                    Style::default().fg(Color::White),
                ),
                _ => (
                    "○",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                    Style::default().fg(Color::DarkGray),
                ),
            };

            // Truncate to tail — the filename end is most informative
            let display = if path.len() > innerWidth {
                format!("…{}", &path[path.len().saturating_sub(innerWidth - 1)..])
            } else {
                path.clone()
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(icon, iconStyle),
                Span::raw("  "),
                Span::styled(display, nameStyle),
            ])));
        }

        if belowCount > 0 {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("   ↓ {} below", belowCount),
                Style::default().fg(Color::DarkGray),
            ))));
        }

        let block = Block::default()
            .title(Span::styled(
                " Files ",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(List::new(items).block(block), listArea);
    }

    // ── Stats ──────────────────────────────────────────────────────
    {
        let chunkCount = state.chunkCount.load(Ordering::Relaxed);
        let dupCount = state.dupCount.load(Ordering::Relaxed);
        let dupSize = state.dupSize.load(Ordering::Relaxed);
        let dupPct = if chunkCount > 0 {
            dupCount * 100 / chunkCount
        } else {
            0
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  chunks "),
                Span::styled(
                    chunkCount.to_string(),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::raw("duplicates "),
                Span::styled(
                    format!("{} ({}%)", dupCount, dupPct),
                    Style::default().fg(Color::Red),
                ),
                Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                Span::raw("saved "),
                Span::styled(fmtSize(dupSize), Style::default().fg(Color::Green)),
            ])),
            layout[2],
        );
    }

    // ── Progress Gauge ─────────────────────────────────────────────
    {
        let completed = state.completedTasks.load(Ordering::Relaxed);
        let total = state.totalTasks.max(1);
        let ratio = (completed as f64 / total as f64).min(1.0);
        let doneFiles = state
            .fileStats
            .iter()
            .filter(|e| matches!(*e.value(), FileStatus::Done))
            .count();

        frame.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
                .ratio(ratio)
                .label(format!(
                    " {}/{} files  {:.0}%",
                    doneFiles,
                    state.totalFiles,
                    ratio * 100.0
                )),
            layout[3],
        );
    }
}
