//! Plays random games with the Rust engine and checks that the Lean reference
//! executable (`proofs/Main.lean`) agrees on every position: the legal moves, the
//! board owners and the result.
//!
//! Usage: `ulttt-difftest <path to ulttt-ref> [games] [seed]`

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, ExitCode, Stdio};
use std::thread;

use ulttt_game::{Move, Moves, Outcome, Player, Position, Variant};

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

/// A random game played to the end, with the description after each move.
fn random_game(v: Variant, rng: &mut Rng) -> (Vec<Move>, Vec<String>) {
    let mut pos = Position::new();
    let mut moves = Vec::new();
    let mut expected = vec![describe(v, &pos)];
    loop {
        let legal: Vec<Move> = Moves(pos.legal_moves(v)).collect();
        if legal.is_empty() {
            return (moves, expected);
        }
        let m = legal[rng.below(legal.len())];
        pos.play(m);
        moves.push(m);
        expected.push(describe(v, &pos));
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(reference) = args.get(1) else {
        eprintln!("usage: ulttt-difftest <path to ulttt-ref> [games] [seed]");
        return ExitCode::FAILURE;
    };
    let games: usize = args.get(2).map_or(3000, |s| s.parse().expect("games"));
    let seed: u64 = args.get(3).map_or(1, |s| s.parse().expect("seed"));

    let mut rng = Rng(seed);
    let cases: Vec<(Variant, Vec<Move>, Vec<String>)> = (0..games)
        .map(|i| {
            let v = Variant::ALL[i % Variant::ALL.len()];
            let (moves, expected) = random_game(v, &mut rng);
            (v, moves, expected)
        })
        .collect();

    let mut child = Command::new(reference)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start the reference executable");
    let mut stdin = child.stdin.take().expect("stdin");
    let input: Vec<String> = cases
        .iter()
        .map(|(v, moves, _)| {
            let ms: Vec<String> = moves.iter().map(|m| m.to_string()).collect();
            format!("{v} {}\n", ms.join(" "))
        })
        .collect();
    let writer = thread::spawn(move || {
        for line in input {
            stdin
                .write_all(line.as_bytes())
                .expect("write to reference");
        }
    });

    let mut lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
    let mut positions = 0usize;
    for (i, (v, moves, expected)) in cases.iter().enumerate() {
        for (k, want) in expected.iter().enumerate() {
            let got = lines
                .next()
                .expect("reference ended early")
                .expect("read from reference");
            if &got != want {
                eprintln!(
                    "MISMATCH in game {i} ({v}) after {k} moves: {:?}",
                    &moves[..k]
                );
                eprintln!("  rust: {want}");
                eprintln!("  lean: {got}");
                let _ = child.kill();
                let _ = child.wait();
                return ExitCode::FAILURE;
            }
            positions += 1;
        }
    }
    writer.join().expect("writer thread");
    child.wait().expect("reference exit");
    println!("ok: {games} games, {positions} positions agree (seed {seed})");
    ExitCode::SUCCESS
}
