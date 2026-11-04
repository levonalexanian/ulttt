//! `ulttt`: command-line interface to the engine and solver.

use std::time::Instant;

use clap::{Parser, Subcommand};
use ulttt_game::random::Rng;
use ulttt_game::{Position, Variant, perft};
use ulttt_solver::{AlphaBeta, Solver};

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
    /// Print random positions reached after a number of random moves.
    Random {
        #[arg(short, long, default_value = "classic")]
        variant: Variant,
        /// Number of random moves played from the start.
        #[arg(long)]
        plies: usize,
        /// Number of positions.
        #[arg(short = 'n', long, default_value_t = 1)]
        count: usize,
        #[arg(short, long, default_value_t = 1)]
        seed: u64,
    },
    /// Compute the exact value of a position.
    Solve {
        #[arg(short, long, default_value = "classic")]
        variant: Variant,
        /// Position in the text notation; the empty board by default.
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
        Command::Random {
            variant,
            plies,
            count,
            seed,
        } => {
            let mut rng = Rng::new(seed);
            let mut found = 0;
            while found < count {
                if let Some(pos) = rng.position(variant, plies) {
                    println!("{pos}");
                    found += 1;
                }
            }
        }
        Command::Solve { variant, position } => {
            let pos = position.unwrap_or_default();
            let mut solver = AlphaBeta::new();
            let start = Instant::now();
            let value = solver.solve(&pos, variant).expect("no node limit");
            let secs = start.elapsed().as_secs_f64();
            let nodes = solver.stats().nodes;
            println!(
                "value {}  nodes {nodes}  {secs:.3}s  {:.1} Mnodes/s",
                value.name(),
                nodes as f64 / secs.max(1e-9) / 1e6
            );
        }
    }
}
