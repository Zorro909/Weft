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
/// The first y of the overworld's sky (`sky_above(40)`).
const SKY: i32 = 640;
/// Ground that rises this much over 4 blocks is steeper than base's cliffs start (1.5 per block).
const CLIFF_RISE: i32 = 8;

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

/// A cliff in a mountain range's forest is bare stone, and the GPU makes it as the CPU does.
pub async fn cliffs(t: T) -> R {
    static CHECK: Config<i32> = Config::named("core:terrain_check");
    let forest = |x: i32, z: i32| name_of(ek::BIOME, OVERWORLD.column(SEED, x, z).biome) == "base:range_forest";
    let rise = |x: i32, z: i32| (OVERWORLD.column(SEED, x + 4, z).height - OVERWORLD.column(SEED, x, z).height).abs();
    // Columns on the terrain's 4 block grid, where its steepness is the rise to the next one.
    let cliff = (-REACH..REACH)
        .flat_map(|z| (-REACH..REACH).map(move |x| (x * STEP, z * STEP)))
        .filter(|&(x, z)| forest(x, z))
        .flat_map(|(x, z)| (0..32).flat_map(move |dz| (0..32).map(move |dx| (x + dx * 4, z + dz * 4))))
        .find(|&(x, z)| forest(x, z) && rise(x, z) >= CLIFF_RISE && !OVERWORLD.surface_carved(SEED, x, z));
    let (x, z) = cliff.ok_or("no cliff in a range's forest")?;
    let height = OVERWORLD.column(SEED, x, z).height;
    t.note(format!("a cliff at {x} {height} {z}, rising {} over 4 blocks", rise(x, z)));
    let before = CHECK.get();
    CHECK.set(1);
    let out = async {
        t.world(SEED).await?;
        t.op("mode spectator").await?;
        let start = t.state();
        t.cmd(&format!("/tp {x} {} {z}", height + 20)).await?;
        t.secs(0.5).await?;
        t.loaded().await?;
        t.idle().await?;
        t.check_eq(t.block([x, height, z]).await?, "base:stone".to_string(), "the cliff's surface");
        let state = t.state();
        t.check(state.gpu_cubes > start.gpu_cubes + 1000, "the GPU made the terrain around the cliff");
        t.check_eq(state.gpu_mismatches, start.gpu_mismatches, "cubes the GPU made differently from the CPU");
        Ok(())
    }
    .await;
    CHECK.set(before);
    out
}
