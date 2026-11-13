//! The overworld's relief, sampled with seed 5 without loading anything.

use base_api::OVERWORLD;
use weft_sdk::abi::entry_kind as ek;
use weft_sdk::*;

use crate::runner::{R, T};

const SEED: u64 = 5;
/// Blocks between sampled columns, and how many are sampled each way from the origin.
const STEP: i32 = 192;
const REACH: i32 = 200;
const SEA: i32 = 63;
/// The first y of the overworld's sky (`sky_above(16)`).
const SKY: i32 = 256;

/// Mountain ranges are rare, rise far above the land around them, and nothing reaches the sky.
pub async fn mountain_ranges(t: T) -> R {
    let (mut land, mut range, mut mountains, mut top, mut range_top) = (0u32, 0u32, 0u32, i32::MIN, i32::MIN);
    for z in -REACH..REACH {
        for x in -REACH..REACH {
            let column = OVERWORLD.column(SEED, x * STEP, z * STEP);
            top = top.max(column.height);
            if column.height < SEA {
                continue;
            }
            land += 1;
            match name_of(ek::BIOME, column.biome) {
                "base:range_forest" | "base:range_alpine" => {
                    range += 1;
                    range_top = range_top.max(column.height);
                }
                "base:mountains" => mountains += 1,
                _ => {}
            }
        }
    }
    let share = |columns: u32| 100.0 * columns as f32 / land.max(1) as f32;
    t.note(format!("highest column {top}, highest below a range's snow {range_top}"));
    t.check(top < SKY, format!("a column reaches the sky: {top}"));
    t.check(range_top >= 180, format!("ranges stay low: {range_top}"));
    t.probe("percent of the land in a range", share(range), (0.5..8.0).contains(&share(range)));
    t.probe("percent of the land in mountains", share(mountains), (0.5..8.0).contains(&share(mountains)));
    Ok(())
}
