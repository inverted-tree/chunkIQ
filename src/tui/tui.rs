use std::{
    sync::{atomic::AtomicBool, Arc},
    thread,
    time::Duration,
};

use crossbeam::queue::ArrayQueue;
use ratatui::{
    init_with_options,
    style::{Style, Stylize},
    symbols,
    widgets::LineGauge,
    DefaultTerminal, TerminalOptions, Viewport,
};
use std::sync::atomic::Ordering;

use crate::trace::tracer::ChunkingTask;

pub fn initAndRunParse() {
    todo!("Implement when the parser works.")
}

pub fn initAndRunTrace(
    numTasks: usize,
    queue: Arc<ArrayQueue<ChunkingTask>>,
    isDone: Arc<AtomicBool>,
) {
    let mut terminal = init(1);

    if numTasks == 0 {
        ratatui::restore();
        return;
    }

    loop {
        let remainingTasks = queue.len();

        if remainingTasks == 0 {
            break;
        }

        terminal
            .draw(|frame| {
                let area = frame.area();
                let _percent = 100 - (remainingTasks * 100 / numTasks);

                let gauge = LineGauge::default()
                    .line_set(symbols::line::THICK)
                    .ratio((remainingTasks / numTasks) as f64);

                let gauge = if isDone.load(Ordering::Relaxed) == true {
                    gauge.filled_style(Style::new().green())
                } else {
                    gauge.filled_style(Style::new().yellow())
                };
                frame.render_widget(gauge, area);
            })
            .unwrap();

        thread::sleep(Duration::from_millis(40));
    }

    destroy();
}

fn init(rows: u16) -> DefaultTerminal {
    let opts = TerminalOptions {
        viewport: Viewport::Inline(rows),
    };
    let terminal = init_with_options(opts);

    terminal
}

fn destroy() {
    ratatui::restore();
}
