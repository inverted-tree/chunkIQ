#![allow(non_snake_case)] // deal with it

mod chunker;
mod parse;
mod trace;
mod util;

use crate::util::arguments::{Cli, Commands};
use clap::Parser;

fn main() {
    let cli: Cli = Cli::parse();

    match cli.command {
        Commands::Parse(mut args) => {
            if let Err(e) = args.validate() {
                eprintln!("[Error] {}", e);
                return;
            }

            parse::parser::run(&args);
        }

        Commands::Trace(mut args) => {
            if let Err(e) = args.validate() {
                eprintln!("[Error] {}", e);
                return;
            }

            if let Err(e) = trace::tracer::run(&args) {
                eprintln!("[Error] {}", e);
                return;
            }
        }
    }
}
