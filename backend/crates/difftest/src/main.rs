//! Differential testing against the Lean reference executable (`proofs/Main.lean`).
//!
//! 1. Rules: plays random games with the Rust engine and checks Lean agrees on
//!    every position: the legal moves, the board owners and the result.
//! 2. Values: picks random endgame positions and checks the Rust solver's value
//!    equals `Ulttt.value`, the minimax value defined in Lean.
//!
//! Usage: `ulttt-difftest <path to ulttt-ref> [games] [seed]`

use std::io::{BufRead, BufReader, Lines, Write};
use std::process::{Child, ChildStdout, Command, ExitCode, Stdio};
use std::thread;

use ulttt_game::{Move, Moves, Outcome, Player, Position, Variant};
use ulttt_solver::{AlphaBeta, Solver};

/// SplitMix64: a small, fast, seedable generator; good enough for random games.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// The description of a position in the reference executable's output format.
fn describe(v: Variant, pos: &Position) -> String {
    let legal: Vec<String> = Moves(pos.legal_moves(v)).map(|m| m.to_string()).collect();
    let owners: String = (0..9)
        .map(|b| pos.owner(b).map_or('.', Player::symbol))
        .collect();
    let result = match pos.result(v) {
        None => "-",
        Some(Outcome::Win(Player::X)) => "x",
        Some(Outcome::Win(Player::O)) => "o",
        Some(Outcome::Draw) => "d",
    };
    format!("{}|{owners}|{result}", legal.join(","))
}

/// A random game played to the end.
fn random_game(v: Variant, rng: &mut Rng) -> Vec<Move> {
    let mut pos = Position::new();
    let mut moves = Vec::new();
    loop {
        let legal: Vec<Move> = Moves(pos.legal_moves(v)).collect();
        if legal.is_empty() {
            return moves;
        }
        let m = legal[rng.below(legal.len())];
        pos.play(m);
        moves.push(m);
    }
}

fn play_all(moves: &[Move]) -> Position {
    let mut pos = Position::new();
    for &m in moves {
        pos.play(m);
    }
    pos
}

/// The number of positions in the full game tree below `pos`, or `None` if it
/// exceeds `limit`.
fn tree_size(pos: &Position, v: Variant, limit: u64) -> Option<u64> {
    let mut total = 1;
    for m in Moves(pos.legal_moves(v)) {
        total += tree_size(&pos.after(m), v, limit - total)?;
        if total > limit {
            return None;
        }
    }
    Some(total)
}

/// The reference executable, fed by a writer thread so pipes cannot deadlock.
struct Reference {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
    writer: Option<thread::JoinHandle<()>>,
}

impl Reference {
    fn start(path: &str, input: Vec<String>) -> Reference {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to start the reference executable");
        let mut stdin = child.stdin.take().expect("stdin");
        let writer = thread::spawn(move || {
            for line in input {
                stdin
                    .write_all(line.as_bytes())
                    .expect("write to reference");
            }
        });
        let lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
        Reference {
            child,
            lines,
            writer: Some(writer),
        }
    }

    fn next_line(&mut self) -> String {
        self.lines
            .next()
            .expect("reference ended early")
            .expect("read from reference")
    }

    fn finish(mut self) {
        if let Some(w) = self.writer.take() {
            w.join().expect("writer thread");
        }
        self.child.wait().expect("reference exit");
    }
}

impl Drop for Reference {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn move_list(moves: &[Move]) -> String {
    moves
        .iter()
        .map(|m| m.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn check_rules(reference: &str, games: usize, rng: &mut Rng) -> bool {
    let cases: Vec<(Variant, Vec<Move>)> = (0..games)
        .map(|i| {
            let v = Variant::ALL[i % Variant::ALL.len()];
            (v, random_game(v, rng))
        })
        .collect();
    let input = cases
        .iter()
        .map(|(v, moves)| format!("{v} {}\n", move_list(moves)))
        .collect();
    let mut lean = Reference::start(reference, input);
    let mut positions = 0usize;
    for (i, (v, moves)) in cases.iter().enumerate() {
        let mut pos = Position::new();
        for k in 0..=moves.len() {
            if k > 0 {
                pos.play(moves[k - 1]);
            }
            let want = describe(*v, &pos);
            let got = lean.next_line();
            if got != want {
                eprintln!(
                    "MISMATCH in game {i} ({v}) after {k} moves: {:?}",
                    &moves[..k]
                );
                eprintln!("  rust: {want}");
                eprintln!("  lean: {got}");
                return false;
            }
            positions += 1;
        }
    }
    lean.finish();
    println!("rules ok: {games} games, {positions} positions agree");
    true
}

fn check_values(reference: &str, count: usize, rng: &mut Rng) -> bool {
    // Endgame positions whose full game tree is small enough for Lean's minimax.
    let mut cases: Vec<(Variant, Vec<Move>)> = Vec::new();
    while cases.len() < count {
        let v = Variant::ALL[cases.len() % Variant::ALL.len()];
        let game = random_game(v, rng);
        let back = 1 + rng.below(12.min(game.len()));
        let moves = game[..game.len() - back].to_vec();
        if tree_size(&play_all(&moves), v, 3_000).is_some() {
            cases.push((v, moves));
        }
    }
    let input = cases
        .iter()
        .map(|(v, moves)| format!("value {v} {}\n", move_list(moves)))
        .collect();
    let mut lean = Reference::start(reference, input);
    for (v, moves) in &cases {
        let want = AlphaBeta::new().solve(&play_all(moves), *v) as i8;
        let got = lean.next_line();
        if got != want.to_string() {
            eprintln!("VALUE MISMATCH ({v}) after {:?}", moves);
            eprintln!("  rust: {want}");
            eprintln!("  lean: {got}");
            return false;
        }
    }
    lean.finish();
    println!("values ok: {count} endgames agree");
    true
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(reference) = args.get(1) else {
        eprintln!("usage: ulttt-difftest <path to ulttt-ref> [games] [seed]");
        return ExitCode::FAILURE;
    };
    let games: usize = args.get(2).map_or(3000, |s| s.parse().expect("games"));
    let seed: u64 = args.get(3).map_or(1, |s| s.parse().expect("seed"));
    println!("seed {seed}");

    let mut rng = Rng(seed);
    if check_rules(reference, games, &mut rng) && check_values(reference, games / 30, &mut rng) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
