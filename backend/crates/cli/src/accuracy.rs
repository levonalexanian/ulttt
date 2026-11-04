//! How often the playing engine keeps a position's value: on suite positions
//! whose exact value is known, does the engine's move preserve it?

use std::process::ExitCode;

use clap::Args;
use ulttt_db::Db;
use ulttt_game::Player;
use ulttt_solver::Value;

#[derive(Args)]
pub struct AccuracyArgs {
    /// Suite whose positions to test; their values come from the database.
    suite: String,
    /// Engine settings (see `match`). `xmax=0` keeps the exact solver out, to
    /// test the heuristic search alone.
    #[arg(short, long, default_value = "xmax=0")]
    engine: String,
    #[arg(long, default_value_t = 20_000)]
    nodes: u64,
    /// Solver for the positions after the engine's moves.
    #[arg(long, default_value = "dfpn")]
    solver: String,
}

/// A value from player `p`'s point of view: 1 win, 0 draw, -1 loss.
fn for_player(v: Value, p: Player) -> i8 {
    let x = v as i8;
    if p == Player::X { x } else { -x }
}

pub fn run(db: &Db, args: &AccuracyArgs) -> ExitCode {
    let Some(suite) = db.suite(&args.suite).expect("read suite") else {
        eprintln!("no suite named {:?}", args.suite);
        return ExitCode::FAILURE;
    };
    let v = suite.variant;
    let Some(mut solver) = ulttt_solver::by_name(&args.solver) else {
        eprintln!("unknown solver {:?}", args.solver);
        return ExitCode::FAILURE;
    };
    if let Err(e) = ulttt_engine::agent(v, &args.engine) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let (mut kept, mut tested, mut wins_kept, mut wins) = (0, 0, 0, 0);
    for (i, pos) in suite.positions.iter().enumerate() {
        let known = db
            .lookup_verified(v, pos)
            .expect("read")
            .or(db.lookup_solved(v, pos).expect("read"));
        let Some(value) = known else {
            println!("{i:>3}  no known value, skipped");
            continue;
        };
        let mut engine = ulttt_engine::agent(v, &args.engine).expect("checked");
        let Some(choice) = engine.best_move(pos, args.nodes) else {
            continue;
        };
        solver.reset();
        let after = solver
            .solve(&pos.after(choice.mv), v)
            .expect("no node limit");
        let me = pos.to_move();
        let ok = for_player(after, me) == for_player(value, me);
        tested += 1;
        kept += usize::from(ok);
        if for_player(value, me) == 1 {
            wins += 1;
            wins_kept += usize::from(ok);
        }
        println!(
            "{i:>3}  {:<5} -> {:<5} {}",
            value.name(),
            after.name(),
            if ok { "kept" } else { "LOST" }
        );
    }
    println!("kept the value in {kept}/{tested} positions ({wins_kept}/{wins} wins)");
    ExitCode::SUCCESS
}
