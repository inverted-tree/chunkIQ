use crossterm::tty::IsTty;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    collections::BTreeMap,
    io::{stdout, Stdout},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crossbeam_channel::Receiver;
use dashmap::DashMap;
use ratatui::{
    prelude::*,
    symbols,
    widgets::{Block, Borders, LineGauge, Paragraph},
};

use crate::trace::tracer::ChunkingTask;

pub fn initAndRunParse() {
    todo!("Implement when the parser works.")
}

pub fn initAndRunTrace(
    numTasks: usize,
    receiver: Receiver<ChunkingTask>,
    isDone: Arc<AtomicBool>,
    fileStats: Arc<DashMap<String, bool>>,
) {
    if !stdout().is_tty() {
        while !isDone.load(Ordering::Relaxed) || !receiver.is_empty() {
            thread::sleep(Duration::from_millis(200));
        }
        return;
    }

    let mut terminal = init().unwrap();

    if numTasks == 0 {
        destroy().unwrap();
        return;
    }

    while !isDone.load(Ordering::Relaxed) && !receiver.is_empty() {
        let mut fileList: BTreeMap<String, bool> = BTreeMap::new();
        for entry in fileStats.iter() {
            fileList.insert(entry.key().clone(), *entry.value());
        }

        let remainingTasks = receiver.len();
        let progress = 1.0 - (remainingTasks as f64 / numTasks as f64);

        terminal
            .draw(|frame| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
                    .split(frame.area());

                let gauge = LineGauge::default()
                    .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT))
                    .line_set(symbols::line::THICK)
                    .ratio(progress)
                    .label(format!("{:.0}%", progress * 100.0));

                let gauge = if isDone.load(Ordering::Relaxed) && receiver.is_empty() {
                    gauge.filled_style(Style::new().green())
                } else {
                    gauge.filled_style(Style::new().yellow())
                };
                frame.render_widget(gauge, chunks[0]);

                // let mut file_list_text = String::new();
                let mut fileListText = String::new();
                for (filename, done) in &fileList {
                    let statusSymbol = if !done { "[...]" } else { "[ ✓ ]" };
                    fileListText.push_str(&format!("{} {}\n", statusSymbol, filename));
                }

                let file_list = Paragraph::new(fileListText).block(
                    Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT),
                );
                frame.render_widget(file_list, chunks[1]);
            })
            .unwrap();

        thread::sleep(Duration::from_millis(100));
    }

    // Final draw to show 100%
    terminal
        .draw(|frame| {
            let area = frame.area();
            let gauge = LineGauge::default()
                .line_set(symbols::line::THICK)
                .ratio(1.0)
                .label("100%")
                .filled_style(Style::new().green());
            frame.render_widget(gauge, area);
        })
        .unwrap();

    destroy().unwrap();
}

type CrosstermTerminal = Terminal<CrosstermBackend<Stdout>>;

fn init() -> Result<CrosstermTerminal, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}

fn destroy() -> Result<(), Box<dyn std::error::Error>> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
