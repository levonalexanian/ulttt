//! `ulttt`: command-line interface to the engine and solver.

mod bench;
mod play;
mod texel;
mod verify;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use ulttt_db::Db;
use ulttt_game::random::Rng;
use ulttt_game::{Position, Variant, perft};

#[derive(Parser)]
#[command(name = "ulttt", about = "Ultimate Tic-Tac-Toe engine and solver")]
struct Cli {
    /// SQLite database for suites, runs and solved positions.
    #[arg(long, global = true, default_value = "data/ulttt.db")]
    db: PathBuf,
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
        /// Solver to use.
        #[arg(long, default_value = "dfpn")]
        solver: String,
        /// Give up after this many positions.
        #[arg(long)]
        node_limit: Option<u64>,
        /// Also print a line of optimal play.
        #[arg(long)]
        pv: bool,
        /// Position in the text notation; the empty board by default.
        position: Option<Position>,
    },
    /// Create a benchmark suite of random positions.
    SuiteCreate {
        name: String,
        #[arg(short, long, default_value = "classic")]
        variant: Variant,
        /// Number of random moves played from the start.
        #[arg(long)]
        plies: usize,
        /// Number of positions.
        #[arg(short = 'n', long, default_value_t = 20)]
        count: usize,
        #[arg(short, long, default_value_t = 1)]
        seed: u64,
    },
    /// List benchmark suites.
    Suites,
    /// Run a solver over a suite and record the results.
    Bench(bench::BenchArgs),
    /// Certify every position of a suite and check the certificates with Lean,
    /// recording verified values in the database.
    Verify(verify::VerifyArgs),
    /// Play a match between two playing-engine configurations and record it.
    Match(play::MatchArgs),
    /// Time the playing engine on fixed positions: speed, and a checksum of its
    /// moves that changes only if its choices do.
    EngineBench {
        /// Engine settings (see `match`).
        #[arg(short, long, default_value = "")]
        engine: String,
        #[arg(short, long, default_value = "classic")]
        variant: Variant,
        #[arg(long, default_value_t = 200_000)]
        nodes: u64,
        /// Positions per game phase (after 6, 20 and 34 random moves).
        #[arg(short = 'n', long, default_value_t = 10)]
        count: usize,
    },
    /// Tune the evaluation weights on the recorded games of matches.
    Texel(texel::TexelArgs),
    /// Show recent engine matches.
    Matches {
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },
    /// Compare two benchmark runs of the same suite, position by position.
    Compare {
        /// The baseline run.
        a: i64,
        /// The run to compare against it.
        b: i64,
    },
    /// Show what the database holds.
    Summary,
    /// Show recent benchmark runs.
    Runs {
        /// Only runs of this suite.
        suite: Option<String>,
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
    },
    /// Solve a position and write certificates of its value, for `ulttt-check`.
    Certify {
        #[arg(short, long, default_value = "classic")]
        variant: Variant,
        /// Solver used to pick the strategy's moves.
        #[arg(long, default_value = "dfpn")]
        solver: String,
        /// Directory to write the certificates to.
        #[arg(short, long, default_value = "data/certificates")]
        out: PathBuf,
        /// File name prefix for the certificates.
        #[arg(long, default_value = "cert")]
        name: String,
        /// Position in the text notation.
        position: Position,
    },
}

fn open_db(path: &PathBuf) -> Db {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).expect("create database directory");
    }
    Db::open(path).expect("open database")
}

fn certify(
    variant: Variant,
    solver_name: &str,
    out: &std::path::Path,
    name: &str,
    pos: &Position,
) -> ExitCode {
    let Some(mut solver) = ulttt_solver::by_name(solver_name) else {
        eprintln!(
            "unknown solver {solver_name:?}; known: {:?}",
            ulttt_solver::SOLVERS
        );
        return ExitCode::FAILURE;
    };
    let (value, files) = verify::write_certificates(solver.as_mut(), pos, variant, out, name);
    for (path, moves) in files {
        println!(
            "value {}: wrote {} ({moves} moves)",
            value.name(),
            path.display()
        );
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Certify {
            variant,
            solver,
            out,
            name,
            position,
        } => return certify(variant, &solver, &out, &name, &position),
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
            for pos in random_positions(&mut rng, variant, plies, count) {
                println!("{pos}");
            }
        }
        Command::Solve {
            variant,
            solver,
            node_limit,
            pv,
            position,
        } => {
            let pos = position.unwrap_or_default();
            let Some(mut solver) = ulttt_solver::by_name(&solver) else {
                eprintln!(
                    "unknown solver {solver:?}; known: {:?}",
                    ulttt_solver::SOLVERS
                );
                return ExitCode::FAILURE;
            };
            solver.set_node_limit(node_limit);
            let start = Instant::now();
            let value = solver.solve(&pos, variant);
            let secs = start.elapsed().as_secs_f64();
            let nodes = solver.stats().nodes;
            println!(
                "value {}  nodes {nodes}  {secs:.3}s  {:.1} Mnodes/s",
                value.map_or("unknown", |v| v.name()),
                nodes as f64 / secs.max(1e-9) / 1e6
            );
            if pv && value.is_some() {
                let line = ulttt_solver::principal_variation(solver.as_mut(), &pos, variant)
                    .expect("solved once already");
                let shown: Vec<String> = line
                    .iter()
                    .map(|&m| {
                        format!(
                            "{}.{}",
                            ulttt_game::board_of(m) + 1,
                            ulttt_game::cell_of(m) + 1
                        )
                    })
                    .collect();
                println!("pv ({} moves, board.cell): {}", line.len(), shown.join(" "));
            }
        }
        Command::SuiteCreate {
            name,
            variant,
            plies,
            count,
            seed,
        } => {
            let mut db = open_db(&cli.db);
            let mut rng = Rng::new(seed);
            let positions = random_positions(&mut rng, variant, plies, count);
            let description = format!("{count} positions after {plies} random moves, seed {seed}");
            db.create_suite(&name, variant, &description, &positions)
                .expect("create suite");
            println!("created suite {name}: {description}");
        }
        Command::Suites => {
            let db = open_db(&cli.db);
            for name in db.suite_names().expect("list suites") {
                let suite = db.suite(&name).expect("read suite").expect("suite exists");
                println!("{name:<24} {:<9} {}", suite.variant, suite.description);
            }
        }
        Command::Bench(args) => return bench::run(&mut open_db(&cli.db), &args),
        Command::Verify(args) => return verify::verify_suite(&open_db(&cli.db), &args),
        Command::Match(args) => return play::run(&open_db(&cli.db), &args),
        Command::EngineBench {
            engine,
            variant,
            nodes,
            count,
        } => return play::bench(&engine, variant, nodes, count),
        Command::Texel(args) => return texel::run(&open_db(&cli.db), &args),
        Command::Matches { limit } => return play::list(&open_db(&cli.db), limit),
        Command::Compare { a, b } => return bench::print_compare(&open_db(&cli.db), a, b),
        Command::Summary => {
            let s = open_db(&cli.db).summary().expect("read summary");
            println!("suites {}  runs {}", s.suites, s.runs);
            for (v, n) in &s.solved {
                println!("solved {v:<9} {n}");
            }
            for (v, n) in &s.verified {
                println!("verified {v:<9} {n}  (certificates accepted by Lean)");
            }
        }
        Command::Runs { suite, limit } => {
            let db = open_db(&cli.db);
            bench::print_runs(&db, suite.as_deref(), limit);
        }
    }
    ExitCode::SUCCESS
}

/// `count` random non-terminal positions after `plies` random moves.
fn random_positions(rng: &mut Rng, v: Variant, plies: usize, count: usize) -> Vec<Position> {
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        if let Some(pos) = rng.position(v, plies) {
            out.push(pos);
        }
    }
    out
}
