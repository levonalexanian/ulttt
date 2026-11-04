//! Benchmarking solvers over suites, with results stored in SQLite.

use std::collections::HashMap;
use std::process::ExitCode;
use std::time::Instant;

use clap::Args;
use ulttt_db::{Db, NewRun};
use ulttt_solver::Value;

#[derive(Args)]
pub struct BenchArgs {
    /// Suite to run.
    suite: String,
    /// Solver to benchmark.
    #[arg(long, default_value = "alphabeta")]
    solver: String,
    /// Give up on a position after this many nodes.
    #[arg(long)]
    node_limit: Option<u64>,
    /// Git revision of the code being benchmarked, recorded with the run.
    #[arg(long, default_value = "")]
    rev: String,
    /// Free-form notes recorded with the run.
    #[arg(long, default_value = "")]
    notes: String,
}

/// Runs a solver over every position of a suite, records each result, and
/// checks the values against earlier runs of the same suite.
pub fn run(db: &mut Db, args: &BenchArgs) -> ExitCode {
    let Some(suite) = db.suite(&args.suite).expect("read suite") else {
        eprintln!("no suite named {:?}", args.suite);
        return ExitCode::FAILURE;
    };
    if ulttt_solver::by_name(&args.solver).is_none() {
        eprintln!(
            "unknown solver {:?}; known: {:?}",
            args.solver,
            ulttt_solver::SOLVERS
        );
        return ExitCode::FAILURE;
    }
    let known = known_values(db, &suite.name);
    let run_id = db
        .start_run(&NewRun {
            suite_id: suite.id,
            solver: &args.solver,
            git_rev: &args.rev,
            node_limit: args.node_limit,
            notes: &args.notes,
        })
        .expect("start run");

    let mut mismatches = 0;
    let (mut total_nodes, mut total_micros, mut solved) = (0u64, 0u64, 0usize);
    for (i, pos) in suite.positions.iter().enumerate() {
        let mut solver = ulttt_solver::by_name(&args.solver).expect("checked above");
        solver.set_node_limit(args.node_limit);
        let start = Instant::now();
        let value = solver.solve(pos, suite.variant);
        let micros = start.elapsed().as_micros() as u64;
        let nodes = solver.stats().nodes;
        db.record_result(run_id, i, value, nodes, micros)
            .expect("record result");
        total_nodes += nodes;
        total_micros += micros;
        let shown = value.map_or("?", Value::name);
        let mut note = String::new();
        if let Some(v) = value {
            solved += 1;
            db.store_solved(suite.variant, pos, v, nodes, &args.solver)
                .expect("store solved");
            if let Some(&expected) = known.get(&i)
                && expected != v
            {
                mismatches += 1;
                note = format!("  MISMATCH: earlier runs found {}", expected.name());
            }
        }
        println!(
            "{i:>3}  {shown:<5} {nodes:>14} nodes {:>10.3}s{note}",
            micros as f64 / 1e6
        );
    }
    db.finish_run(run_id).expect("finish run");
    println!(
        "run {run_id}: {} solved {solved}/{}  nodes {total_nodes}  time {:.3}s  {:.1} Mnodes/s",
        args.solver,
        suite.positions.len(),
        total_micros as f64 / 1e6,
        total_nodes as f64 / (total_micros.max(1) as f64)
    );
    if mismatches > 0 {
        eprintln!("{mismatches} values disagree with earlier runs");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Values found for each position by earlier runs of a suite.
fn known_values(db: &Db, suite: &str) -> HashMap<usize, Value> {
    let mut known = HashMap::new();
    for run in db.run_summaries(Some(suite), usize::MAX).expect("runs") {
        for (i, v) in db.run_values(run.id).expect("run values") {
            if let Some(v) = v {
                known.insert(i, v);
            }
        }
    }
    known
}

/// Prints a table of recent runs.
pub fn print_runs(db: &Db, suite: Option<&str>, limit: usize) {
    println!(
        "{:>5}  {:<20} {:<24} {:<9} {:>9} {:>16} {:>10} {:>9}",
        "run", "suite", "solver", "rev", "solved", "nodes", "seconds", "Mnodes/s"
    );
    for r in db.run_summaries(suite, limit).expect("runs") {
        println!(
            "{:>5}  {:<20} {:<24} {:<9} {:>4}/{:<4} {:>16} {:>10.3} {:>9.1}",
            r.id,
            r.suite,
            r.solver,
            r.git_rev,
            r.solved,
            r.positions,
            r.nodes,
            r.micros as f64 / 1e6,
            r.nodes as f64 / r.micros.max(1) as f64
        );
    }
}
