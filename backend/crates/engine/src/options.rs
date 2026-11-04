//! Engine settings, parseable from a short text spec so matches can compare
//! configurations without recompiling.

use ulttt_game::Variant;

/// Weights of the heuristic evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    /// Global lines: the sum over lines of the product of board chances.
    pub global: f32,
    /// Board chances, weighted by how many global lines each board is on.
    pub local: f32,
    /// `majority` only: the sum of board chances.
    pub majority: f32,
    /// Bonus for the player to move having a free choice of board.
    pub tempo: f32,
    /// Weight of an open local line holding one or two of the player's marks
    /// (an empty open line weighs 1).
    pub line1: f32,
    pub line2: f32,
    /// Nonzero: combine global lines as the chance that at least one
    /// completes, instead of their sum.
    pub union: f32,
    /// Bonus for the player to move being sent to a board they can take with
    /// their move.
    pub sent: f32,
    /// Weights of local boards by position, for the `local` term: the center
    /// is on four global lines, corners on three, edges on two.
    pub center: f32,
    pub corner: f32,
    pub edge: f32,
    /// An unowned board's chances: `(base + fill * filled) * potential /
    /// (potential of both + prior)`, with `filled` the share of occupied cells.
    pub base: f32,
    pub fill: f32,
    pub prior: f32,
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            global: 6000.0,
            local: 400.0,
            majority: 1500.0,
            tempo: 300.0,
            line1: 3.0,
            line2: 10.0,
            union: 0.0,
            sent: 0.0,
            center: 4.0,
            corner: 3.0,
            edge: 2.0,
            base: 0.5,
            fill: 0.4,
            prior: 4.0,
        }
    }
}

impl Weights {
    /// The default weights for variant `v`, tuned on self-play games with
    /// `ulttt texel` and kept only where a match confirmed them.
    pub fn for_variant(v: Variant) -> Weights {
        let base = Weights::default();
        match v {
            // +37 ±18 Elo against the untuned weights (800 games, 20k nodes).
            Variant::Classic => Weights {
                global: 10_133.0,
                local: 70.9,
                tempo: 1511.0,
                line1: 2.131,
                line2: 8.213,
                ..base
            },
            // +38 ±22 Elo against the untuned weights (800 games, 20k nodes).
            Variant::Majority => Weights {
                global: 7376.0,
                local: 246.5,
                majority: 1630.0,
                tempo: 2043.0,
                line1: 2.356,
                line2: 12.92,
                ..base
            },
            // Tuned weights were not significantly better (+10 ±22).
            Variant::Open => base,
        }
    }
}

/// Engine settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// Heuristic transposition table size as a power of two.
    pub tt_bits: u32,
    /// Exact solver table size as a power of two.
    pub exact_tt_bits: u32,
    /// Try the exact solver in positions with at most this many empty cells
    /// (`0` never tries it).
    pub exact_max_empties: u32,
    /// The exact solver gets `1 / exact_share` of the node budget.
    pub exact_share: u64,
    /// At the search horizon, recognize a win in one move before evaluating.
    pub leaf_wins: bool,
    /// Principal variation search: null-window searches after the first move.
    pub pvs: bool,
    /// Board-winning moves followed past the horizon (`0` turns it off).
    pub quiescence: u32,
    /// Try two recent cutoff moves per ply early.
    pub killers: bool,
    /// Two table entries per bucket, keeping the deeper current one.
    pub tt_buckets: bool,
    /// Search moves from this index on (in move order) one ply shallower
    /// first, at depth 3 or more (`0` turns it off).
    pub lmr: u32,
    /// Half-width of the root's aspiration window from depth 4 (`0` turns
    /// it off).
    pub aspiration: u32,
    /// Quiescence: lower the stand-pat score by this much when every move
    /// gives the opponent a board to take or a free choice (`0` turns it off).
    pub unsafe_penalty: u32,
    pub weights: Weights,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            tt_bits: 18,
            exact_tt_bits: 18,
            exact_max_empties: 55,
            exact_share: 4,
            leaf_wins: true,
            pvs: true,
            quiescence: 6,
            killers: true,
            tt_buckets: false,
            lmr: 0,
            aspiration: 0,
            unsafe_penalty: 0,
            weights: Weights::default(),
        }
    }
}

impl Options {
    /// Parses comma-separated `key=value` settings on top of the defaults. The
    /// keys are in the `match` below; flags take `0` or `1`.
    pub fn parse(spec: &str) -> Result<Options, String> {
        Options::parse_onto(Options::default(), spec)
    }

    /// The default settings for variant `v` (see [`Weights::for_variant`]).
    pub fn for_variant(v: Variant) -> Options {
        Options {
            weights: Weights::for_variant(v),
            ..Options::default()
        }
    }

    /// Like [`Options::parse`], on top of the defaults for variant `v`.
    pub fn parse_for(v: Variant, spec: &str) -> Result<Options, String> {
        Options::parse_onto(Options::for_variant(v), spec)
    }

    fn parse_onto(mut o: Options, spec: &str) -> Result<Options, String> {
        for part in spec.split(',').filter(|p| !p.is_empty()) {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, got {part:?}"))?;
            let int = || {
                value
                    .parse::<u32>()
                    .map_err(|_| format!("bad value in {part:?}"))
            };
            let float = || {
                value
                    .parse::<f32>()
                    .map_err(|_| format!("bad value in {part:?}"))
            };
            match key {
                "tt" => o.tt_bits = int()?.clamp(4, 30),
                "xtt" => o.exact_tt_bits = int()?.clamp(4, 30),
                "xmax" => o.exact_max_empties = int()?,
                "qsafe" => o.unsafe_penalty = int()?,
                "asp" => o.aspiration = int()?,
                "lmr" => o.lmr = int()?,
                "tt2" => o.tt_buckets = int()? != 0,
                "killers" => o.killers = int()? != 0,
                "qs" => o.quiescence = int()?,
                "pvs" => o.pvs = int()? != 0,
                "leafwin" => o.leaf_wins = int()? != 0,
                "xshare" => o.exact_share = u64::from(int()?.max(1)),
                "global" => o.weights.global = float()?,
                "local" => o.weights.local = float()?,
                "majority" => o.weights.majority = float()?,
                "tempo" => o.weights.tempo = float()?,
                "line1" => o.weights.line1 = float()?,
                "line2" => o.weights.line2 = float()?,
                "union" => o.weights.union = float()?,
                "sent" => o.weights.sent = float()?,
                "center" => o.weights.center = float()?,
                "corner" => o.weights.corner = float()?,
                "edge" => o.weights.edge = float()?,
                "base" => o.weights.base = float()?,
                "fill" => o.weights.fill = float()?,
                "prior" => o.weights.prior = float()?,
                _ => return Err(format!("unknown engine option {key:?}")),
            }
        }
        Ok(o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_options() {
        assert_eq!(Options::parse(""), Ok(Options::default()));
        let o = Options::parse("tt=12,xmax=0,tempo=0").unwrap();
        assert_eq!(
            (o.tt_bits, o.exact_max_empties, o.weights.tempo),
            (12, 0, 0.0)
        );
        assert!(Options::parse("nope=1").is_err());
        assert!(Options::parse("tt").is_err());
    }
}
