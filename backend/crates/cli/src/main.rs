//! `ulttt`: command-line interface to the engine and solver.

use std::time::Instant;

use clap::{Parser, Subcommand};
use ulttt_game::{Position, Variant, perft};

#[derive(Parser)]
#[command(name = "ulttt", about = "Ultimate Tic-Tac-Toe engine and solver")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Count move sequences of each length up to `depth`.
    Perft {
        /// Rule variant: classic, open or majority.
        #[arg(short, long, default_value = "classic")]
        variant: Variant,
        /// Maximum number of moves.
        #[arg(short, long, default_value_t = 5)]
        depth: u32,
        /// Starting position in the text notation; the empty board by default.
        #[arg(short, long)]
        position: Option<Position>,
    },
}

fn main() {
    match Cli::parse().command {
        Command::Perft {
            variant,
            depth,
            position,
        } => {
            let pos = position.unwrap_or_default();
            for d in 1..=depth {
                let start = Instant::now();
                let n = perft(&pos, variant, d);
                let secs = start.elapsed().as_secs_f64();
                println!(
                    "depth {d:>2}  {n:>16}  {secs:>8.3}s  {:>8.1} Mnodes/s",
                    n as f64 / secs.max(1e-9) / 1e6
                );
            }
        }
    }
}
