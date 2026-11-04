//! Certifying solved positions and checking the certificates with Lean.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use clap::Args;
use ulttt_db::Db;
use ulttt_game::{Player, Position, Variant};
use ulttt_solver::{Solver, Value};

/// Solves `pos` and writes the certificates proving its value to `out`. A win
/// needs one certificate (for the winner), a draw two (at least a draw for
/// each player). Returns the value and each file with its number of moves.
pub fn write_certificates(
    solver: &mut dyn Solver,
    pos: &Position,
    variant: Variant,
    out: &Path,
    name: &str,
) -> (Value, Vec<(PathBuf, usize)>) {
    let value = solver.solve(pos, variant).expect("no node limit");
    let claims = match value {
        Value::XWins => vec![(Player::X, 1)],
        Value::OWins => vec![(Player::O, 1)],
        Value::Draw => vec![(Player::X, 0), (Player::O, 0)],
    };
    std::fs::create_dir_all(out).expect("create output directory");
    let files = claims
        .into_iter()
        .map(|(player, at_least)| {
            let cert = ulttt_certificate::generate(solver, pos, variant, player, at_least)
                .expect("the solved value holds");
            let path = out.join(format!("{name}-{}.cert", player.symbol()));
            std::fs::write(&path, cert.to_string()).expect("write certificate");
            (path, cert.moves.len())
        })
        .collect();
    (value, files)
}

#[derive(Args)]
pub struct VerifyArgs {
    /// Suite whose positions to verify.
    suite: String,
    /// Solver used to solve positions and pick the strategies' moves.
    #[arg(long, default_value = "dfpn")]
    solver: String,
    /// The Lean certificate checker.
    #[arg(long, default_value = "proofs/.lake/build/bin/ulttt-check")]
    checker: PathBuf,
    /// Positions certified and checked in parallel.
    #[arg(short, long, default_value_t = 1)]
    jobs: usize,
}

/// Solves every position of a suite, certifies its value, checks the
/// certificates with the Lean checker, and records accepted values in the
/// `verified` table.
pub fn verify_suite(db: &Db, args: &VerifyArgs) -> ExitCode {
    let Some(suite) = db.suite(&args.suite).expect("read suite") else {
        eprintln!("no suite named {:?}", args.suite);
        return ExitCode::FAILURE;
    };
    if ulttt_solver::by_name(&args.solver).is_none() {
        eprintln!("unknown solver {:?}", args.solver);
        return ExitCode::FAILURE;
    }
    let dir = std::env::temp_dir().join(format!("ulttt-verify-{}", std::process::id()));
    let mut failures = 0;
    // Jobs take positions in turn; results come back to this thread, which
    // owns the database.
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel();
    let (positions, variant, dir) = (&suite.positions, suite.variant, &dir);
    std::thread::scope(|scope| {
        for _ in 0..args.jobs.max(1) {
            let (tx, next) = (tx.clone(), &next);
            scope.spawn(move || {
                let mut solver = ulttt_solver::by_name(&args.solver).expect("checked above");
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(pos) = positions.get(i) else { break };
                    solver.reset();
                    let (value, files) =
                        write_certificates(solver.as_mut(), pos, variant, dir, &format!("{i}"));
                    let output = Command::new(&args.checker)
                        .args(files.iter().map(|(p, _)| p))
                        .output()
                        .expect("run the Lean checker");
                    for (p, _) in &files {
                        let _ = std::fs::remove_file(p);
                    }
                    let moves: usize = files.iter().map(|(_, n)| n).sum();
                    let _ = tx.send((i, value, moves, output));
                }
            });
        }
        drop(tx);
        for (i, value, moves, output) in rx {
            if output.status.success() {
                db.store_verified(variant, &positions[i], value, moves as u64, &args.solver)
                    .expect("store verified");
                println!(
                    "{i:>3}  {:<5} verified ({moves} certificate moves)",
                    value.name()
                );
            } else if output.status.code() == Some(1) {
                // `ulttt-check` exits with 1 exactly when it rejects a certificate.
                failures += 1;
                println!("{i:>3}  {:<5} REJECTED by the checker", value.name());
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                // Killed or crashed, e.g. out of memory on a huge certificate:
                // nothing was decided either way.
                failures += 1;
                println!(
                    "{i:>3}  {:<5} not checked: the checker failed ({}, {moves} moves)",
                    value.name(),
                    output.status
                );
            }
        }
    });
    let _ = std::fs::remove_dir(dir);
    for (variant, n) in db.verified_counts().expect("count verified") {
        println!("verified positions ({variant}): {n}");
    }
    if failures > 0 {
        eprintln!("{failures} positions failed verification");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
