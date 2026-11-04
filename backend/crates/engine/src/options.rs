//! Engine settings, parseable from a short text spec so matches can compare
//! configurations without recompiling.

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
            weights: Weights::default(),
        }
    }
}

impl Options {
    /// Parses comma-separated `key=value` settings on top of the defaults:
    /// `tt`, `xtt`, `xmax`, `xshare`, `leafwin`, `pvs` (0 or 1), and the weights `global`, `local`,
    /// `majority`, `tempo`, `line1`, `line2`.
    pub fn parse(spec: &str) -> Result<Options, String> {
        let mut o = Options::default();
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
                "pvs" => o.pvs = int()? != 0,
                "leafwin" => o.leaf_wins = int()? != 0,
                "xshare" => o.exact_share = u64::from(int()?.max(1)),
                "global" => o.weights.global = float()?,
                "local" => o.weights.local = float()?,
                "majority" => o.weights.majority = float()?,
                "tempo" => o.weights.tempo = float()?,
                "line1" => o.weights.line1 = float()?,
                "line2" => o.weights.line2 = float()?,
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
