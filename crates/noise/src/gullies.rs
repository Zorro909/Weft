//! Gullies that run down a slope and branch, for eroded mountainsides: stripes across the
//! slope, laid out per point from its own slope, so no column needs its neighbours. After the
//! erosion filter of Rune Skovbo Johansen (2026), which builds on Clay John's and Fewes's.
//!
//! Integer-only, like `fixed`: `terrain.wgsl` copies it line by line.

use crate::fixed::{ONE, div_floor, hash, ramp};

/// Layers of gullies, each half as wide and half as deep as the one before.
pub const OCTAVES: u32 = 3;
/// A slope is the rise over this many blocks, in Q4. Heights are only as fine as their
/// formulas' arithmetic, a fifth of a block in a high mountain, and over a short span that
/// would turn the slope's direction, and the gullies with it, from one column to the next.
pub const SPAN: i32 = 32;
/// The slope of 45 degrees.
const DIAGONAL: i32 = 16 * SPAN;
/// The slopes gullies start at and are fully cut at.
const ONSET: (i32, i32) = (DIAGONAL * 5 / 8, DIAGONAL * 3 / 2);
/// The steepest slope taken into account.
const STEEPEST: i32 = 1 << 14;
/// Each cell has a pivot at least this far from its edges (Q12 of a cell), and a pivot's stripes
/// reach this far: just short of the nearest pivot outside the 3 x 3 cells around a point.
const MARGIN: i32 = ONE / 4;
const REACH: i32 = ONE + MARGIN;
/// Stripes that agree less than one in this many are scaled up to full ones.
const SHARPEN: i32 = 2;
/// The slope of a gully's side for a depth equal to its width: pi times 45 degrees.
const SIDE: i32 = DIAGONAL * 355 / 113;
/// What the octaves' seeds differ by, as `fixed::fbm`'s do.
const OCTAVE_SEED: u32 = 7919;

/// The square root, rounded down.
pub fn isqrt(v: u32) -> u32 {
    let (mut rest, mut root, mut bit) = (v, 0u32, 1u32 << 30);
    while bit > rest {
        bit >>= 2;
    }
    while bit != 0 {
        if rest >= root + bit {
            rest -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// The sine of `turns` (Q12 of a full turn) in Q12, a parabola for each half turn.
fn sine(turns: i32) -> i32 {
    let half = turns & (ONE / 2 - 1);
    let arc = (half * (ONE / 2 - half)) >> 8;
    if turns & (ONE / 2) == 0 { arc } else { -arc }
}

/// `share` (0 to `ONE`) rising fast at first and levelling off at `ONE`.
fn eased(share: i32) -> i32 {
    let rest = ONE - share.clamp(0, ONE);
    ONE - ((rest * rest) >> 12)
}

/// The length of (`x`, `z`).
fn length(x: i32, z: i32) -> i32 {
    isqrt((x * x + z * z) as u32) as i32
}

/// Stripes one cell of `size` (Q4) wide running along (`along_x`, `along_z`) (a direction of
/// length `ONE`) at (`x`, `z`) (Q4): the cosine and sine of where the point is across them (Q12),
/// 1 and 0 on a crest. Every cell turns its stripes about its own pivot and the cells around the
/// point are blended, so the direction may change from place to place.
fn stripes(seed: u32, x: i32, z: i32, size: i32, along_x: i32, along_z: i32) -> (i32, i32) {
    let (cell_x, cell_z) = (div_floor(x, size).0, div_floor(z, size).0);
    let (mut cos, mut sin, mut weight) = (0, 0, 0);
    for dz in -1..=1 {
        for dx in -1..=1 {
            let h = hash(seed, cell_x + dx, 0, cell_z + dz);
            let pivot = |cell: i32, roll: u32| cell * size + (((MARGIN + (roll & 4095) as i32 / 2) * size) >> 12);
            // From the pivot to the point, in cells (Q12).
            let off_x = (x - pivot(cell_x + dx, h)) * ONE / size;
            let off_z = (z - pivot(cell_z + dz, h >> 12)) * ONE / size;
            let near = (((REACH * REACH) >> 12) - ((off_x * off_x + off_z * off_z) >> 12)).max(0);
            let w = (near * near) >> 12;
            let across = (off_z * along_x - off_x * along_z) >> 12;
            cos += w * sine(across + ONE / 4);
            sin += w * sine(across);
            weight += w;
        }
    }
    if weight == 0 {
        return (0, 0);
    }
    let (cos, sin) = (cos / weight, sin / weight);
    // Cells that disagree blend into fainter stripes: bring those back up.
    let strength = length(cos, sin);
    if strength == 0 {
        (0, 0)
    } else if strength * SHARPEN >= ONE {
        (cos * ONE / strength, sin * ONE / strength)
    } else {
        (cos * SHARPEN, sin * SHARPEN)
    }
}

/// How far gullies cut into the ground at (`x`, `z`) (Q4), 0 to about twice `depth` (Q4):
/// gullies `size` (Q4) apart run down the slope (`slope_x`, `slope_z`) (the rise over `SPAN`
/// blocks east and south, Q4), with finer ones branching into them. Crests keep their height;
/// flat ground has none.
pub fn gullies(seed: u32, x: i32, z: i32, slope_x: i32, slope_z: i32, size: i32, depth: i32) -> i32 {
    let size = size.clamp(64, 4096);
    let (mut slope_x, mut slope_z) = (slope_x.clamp(-STEEPEST, STEEPEST), slope_z.clamp(-STEEPEST, STEEPEST));
    // How much of its depth a layer cuts here: nothing on gentle ground.
    let mut share = eased(ramp(length(slope_x, slope_z), ONSET.0, ONSET.1));
    if share == 0 {
        return 0;
    }
    let side = (SIDE * depth.clamp(0, 1 << 14) / size).min(STEEPEST);
    let mut cut = 0;
    for octave in 0..OCTAVES {
        let steepness = length(slope_x, slope_z);
        if steepness == 0 {
            break;
        }
        let (down_x, down_z) = (slope_x * ONE / steepness, slope_z * ONE / steepness);
        let (cos, sin) = stripes(seed.wrapping_add(octave * OCTAVE_SEED), x, z, size >> octave, down_x, down_z);
        cut += ((((depth >> octave) * (ONE - cos)) >> 13) * share) >> 12;
        // The side of the gully the point is on turns the next, finer gullies toward its floor.
        // They meet at an angle there and on the crest, so those two stay clear of them.
        let lean = if sin > 0 { -side } else { side };
        slope_x = (slope_x - ((down_z * lean) >> 12)).clamp(-STEEPEST, STEEPEST);
        slope_z = (slope_z + ((down_x * lean) >> 12)).clamp(-STEEPEST, STEEPEST);
        share = (share * eased(sin.abs())) >> 12;
    }
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_roots_round_down() {
        for (v, root) in [(0, 0), (1, 1), (3, 1), (4, 2), (99, 9), (1 << 30, 1 << 15), (u32::MAX, 65535)] {
            assert_eq!(isqrt(v), root, "the root of {v}");
        }
    }

    #[test]
    fn the_sine_is_a_wave() {
        assert_eq!(sine(0), 0);
        assert_eq!(sine(ONE / 4), ONE);
        assert_eq!(sine(ONE / 2), 0);
        assert_eq!(sine(3 * ONE / 4), -ONE);
        assert_eq!(sine(-ONE / 4), -ONE, "below zero it goes on");
    }

    #[test]
    fn flat_ground_keeps_its_height() {
        for x in 0..64 {
            assert_eq!(gullies(7, x * 16, 300, 0, 0, 640, 160), 0, "at {x}");
            assert_eq!(gullies(7, x * 16, 300, 48, -48, 640, 160), 0, "a gentle slope at {x}");
        }
    }

    /// Gullies run down the slope: the cut changes across it far more than along it.
    #[test]
    fn gullies_run_down_the_slope() {
        let cut = |x: i32, z: i32| gullies(7, x * 16, z * 16, 768, 0, 640, 80);
        let (mut along, mut across) = (0, 0);
        for z in 0..200 {
            for x in 0..200 {
                along += (cut(x + 1, z) - cut(x, z)).abs();
                across += (cut(x, z + 1) - cut(x, z)).abs();
            }
        }
        assert!(across > 3 * along, "across {across}, along {along}");
    }

    #[test]
    fn cuts_stay_within_reach() {
        let mut deepest = 0;
        for z in 0..150 {
            for x in 0..150 {
                let cut = gullies(3, x * 16, z * 16, 1600, -1200, 640, 320);
                assert!(cut >= 0, "raised the ground at {x} {z}");
                deepest = deepest.max(cut);
            }
        }
        assert!(deepest > 320 && deepest <= 2 * 320, "the deepest cut is {deepest}");
    }

    /// A step to the next block never jumps: stripes blend from cell to cell.
    #[test]
    fn neighbours_differ_by_little() {
        let cut = |x: i32, z: i32| gullies(11, x * 16, z * 16, 960, 480, 640, 320);
        let mut steepest = 0;
        for z in 0..300 {
            for x in 0..300 {
                steepest = steepest.max(
                    (cut(x + 1, z) - cut(x, z))
                        .abs()
                        .max((cut(x, z + 1) - cut(x, z)).abs()),
                );
            }
        }
        // Gullies 40 blocks apart and 20 deep have sides of 1.6 blocks per block.
        assert!(steepest < 96, "a jump of {steepest} between neighbours");
    }
}
