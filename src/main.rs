#![allow(non_snake_case)]

mod chunker;
mod parse;
mod trace;
mod tui;
mod util;

use crate::util::arguments::{Cli, Commands};
use clap::Parser;
use parse::parser;
use trace::tracer;

fn main() {
    let cli: Cli = Cli::parse();

    match cli.command {
        Commands::Parse(mut args) => {
            if let Err(e) = args.validate() {
                eprintln!("[Error] {}", e);
                return;
            }

            parser::run(&args);
        }

        Commands::Trace(mut args) => {
            if let Err(e) = args.validate() {
                eprintln!("[Error] {}", e);
                return;
            }

            if let Err(e) = tracer::run(&args) {
                eprintln!("[Error] {}", e);
                return;
            }
            // let (numTasks, receiver, isDone, file_stats_map) = tracer::run(&args).unwrap();
            // initAndRunTrace(numTasks, receiver, isDone, file_stats_map);
        }
    }
}
