//! `ulttt`: command-line interface to the engine and solver.

mod bench;

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
        #[arg(long, default_value = "alphabeta")]
        solver: String,
        /// Give up after this many positions.
        #[arg(long)]
        node_limit: Option<u64>,
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
        #[arg(long, default_value = "search")]
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

/// Solves `pos` and writes the certificates proving its value. A win needs one
/// certificate (for the winner), a draw two (at least a draw for each player).
fn certify(
    variant: Variant,
    solver_name: &str,
    out: &std::path::Path,
    name: &str,
    pos: &Position,
) -> ExitCode {
    use ulttt_game::Player;
    use ulttt_solver::Value;
    let Some(mut solver) = ulttt_solver::by_name(solver_name) else {
        eprintln!(
            "unknown solver {solver_name:?}; known: {:?}",
            ulttt_solver::SOLVERS
        );
        return ExitCode::FAILURE;
    };
    let value = solver.solve(pos, variant).expect("no node limit");
    let claims = match value {
        Value::XWins => vec![(Player::X, 1)],
        Value::OWins => vec![(Player::O, 1)],
        Value::Draw => vec![(Player::X, 0), (Player::O, 0)],
    };
    std::fs::create_dir_all(out).expect("create output directory");
    for (player, at_least) in claims {
        let cert = ulttt_certificate::generate(solver.as_mut(), pos, variant, player, at_least)
            .expect("the solved value holds");
        let path = out.join(format!("{name}-{}.cert", player.symbol()));
        std::fs::write(&path, cert.to_string()).expect("write certificate");
        println!(
            "value {}: wrote {} ({} moves)",
            value.name(),
            path.display(),
            cert.moves.len()
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
