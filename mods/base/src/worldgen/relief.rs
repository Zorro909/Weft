//! The overworld's relief as parts of its terrain: hills that differ by region, mountains in
//! massifs, and rare mountain ranges with foothills, a crest, valleys and passes.
//!
//! Nothing here may reach the overworld's sky at y 640 (`sky_above(40)` in `lib.rs`): each
//! height says how it stays under it.

use weft_sdk::terrain::*;
use weft_sdk::*;

use super::{GRASS_COVER, TREE_KINDS, TREES};

/// Field: 0 outside a mountain range to 1 on its spine.
pub const RANGE: &str = "base:range";
/// Field: 0 in plains to 1 in uplands and at the foot of a range.
pub const RELIEF: &str = "base:relief";
/// Field: how mountainous a place is outside the ranges.
pub const MOUNTAINS: &str = "base:mountains";
/// Field: `MOUNTAINS`, or `RANGE` scaled so that both biomes start at `MOUNTAINS_BIOME`.
pub const HIGHLAND: &str = "base:highland";

/// How much of `RANGE` a column needs to belong to a range's biomes.
pub const RANGE_BIOME: f64 = 0.25;
/// How much of `MOUNTAINS` a column needs to belong to the mountains biome.
pub const MOUNTAINS_BIOME: f64 = 0.45;
/// A range's forest starts at this y; below it the lowland's biomes go on.
pub const RANGE_FOREST_FROM: i32 = 110;
/// No trees from this y up.
pub const TREE_LINE: i32 = 200;
/// Bare stone above this y.
const ROCK_FROM: i32 = 250;
/// Ground rising more than this many blocks per block is a cliff: bare stone in the mountains.
pub const CLIFF: f64 = 1.5;

/// The size in blocks of the noise whose zero lines the spines of ranges follow.
const SPINE_SIZE: f64 = 12000.0;
/// How far that noise is from zero at the edge of a range: bigger is wider.
const SPINE_REACH: f64 = 0.0884;
/// The spine is bent by up to this many blocks, in bends about `SPINE_WARP_SIZE` long.
const SPINE_WARP: f64 = 500.0;
const SPINE_WARP_SIZE: f64 = 2400.0;
/// Ranges stand where a noise of this size is above `GATE.0`, at full height from `GATE.1`:
/// raising both makes ranges rarer.
const GATE_SIZE: f64 = 14000.0;
const GATE: (f64, f64) = (0.12, 0.26);
/// Below this much of `RANGE` are the foothills.
const FOOTHILLS: f64 = 0.3;

/// What a range would add if all its parts peaked at once, in blocks.
const RANGE_HEIGHT: f64 = 600.0;
/// The most a range adds: the lowland under it reaches y 85 (sea 63, continents 18, dunes 4)
/// and has no hills where a range can get this high.
const RANGE_CAP: f64 = 545.0;
/// The share of a range's height its foothills reach.
const FOOTHILL_SHARE: f64 = 0.1;
/// About this many blocks lie between the peaks along a crest, and the saddles between them
/// are lower by up to twice this share.
const PEAK_SIZE: f64 = 420.0;
const SADDLE_DROP: f64 = 0.28;
/// The sizes in blocks of the noises a range's largest ridges and the valleys through it follow.
const RIDGE_SIZE: f64 = 800.0;
const VALLEY_SIZE: f64 = 1300.0;
/// The share of the height a valley floor keeps.
const VALLEY_FLOOR: f64 = 0.5;
/// Ridges and valleys are bent by up to this many blocks, in bends about `BEND_SIZE` long.
const BEND: f64 = 70.0;
const BEND_SIZE: f64 = 300.0;

/// What a massif adds along its ridges, in blocks. Where every octave of ridges peaks at once
/// it is 1.17 times this, which with the lowland (y 85), the highest hills and the roughness
/// stays under y 230.
const MASSIF_HEIGHT: f64 = 90.0;
const MASSIF_RIDGE_SIZE: f64 = 300.0;
const MASSIF_ROUGHNESS: f64 = 5.0;

/// Hills are this high in plains and this high in uplands, in blocks above and below.
const PLAIN_HILLS: f64 = 4.0;
const UPLAND_HILLS: f64 = 34.0;
const HILL_SIZE: f64 = 170.0;
const RELIEF_SIZE: f64 = 1600.0;

/// How far a ridge noise is from zero where its ridge meets the flat floor: smaller ridges
/// are narrower.
const RIDGE_WIDTH: f64 = 0.35;
/// How much each finer octave of ridges adds to the one before it.
const RIDGE_DETAIL: f64 = 0.6;

/// What the noises here are salted with, one name each.
mod salt {
    pub const SPINE: u64 = 0xA003;
    pub const SPINE_WARP_X: u64 = 0xA001;
    pub const SPINE_WARP_Z: u64 = 0xA002;
    pub const GATE: u64 = 0xA004;
    pub const PEAKS: u64 = 0xA005;
    pub const BEND_X: u64 = 0xA006;
    pub const BEND_Z: u64 = 0xA007;
    /// Octave `k` of a range's ridges uses `RIDGES + k`.
    pub const RIDGES: u64 = 0xA010;
    pub const VALLEYS: u64 = 0xA020;
    pub const RELIEF: u64 = 0xA030;
    /// Octave `k` of a massif's ridges uses `MASSIF + k`.
    pub const MASSIF: u64 = 0xA040;
    pub const ROUGHNESS: u64 = 0xA050;
    pub const MOUNTAINS: u64 = 0x44;
    pub const HILLS: u64 = weft_sdk::noise::fixed::salt::HILLS;
}

pub static RANGE_FOREST: Biome = Biome::named("range_forest");
pub static RANGE_ALPINE: Biome = Biome::named("range_alpine");

/// The biomes of a mountain range below its snow: forest, then meadow and bare rock.
pub fn biomes(r: &mut Registry) {
    let (grass, foliage) = (&tints::GRASS, &tints::FOLIAGE);
    r.biome(&RANGE_FOREST)
        .surface("grass", "dirt", 3)
        .tint(grass, 0x86b783)
        .tint(foliage, 0x68a464)
        .tag("mountain")
        .tag("mountain_range")
        .tag("forest")
        .tag("animals")
        .bare_steeper(CLIFF)
        .set(&TREES, 0.45)
        .set(&GRASS_COVER, 0.08)
        .set_text(&TREE_KINDS, "base:spruce_log");
    r.biome(&RANGE_ALPINE)
        .surface("grass", "dirt", 3)
        .bare_above(ROCK_FROM)
        .bare_steeper(CLIFF)
        .tint(grass, 0x8ab689)
        .tint(foliage, 0x6da36b)
        .tag("mountain")
        .tag("mountain_range")
        .tag("animals")
        .set(&GRASS_COVER, 0.2);
}

fn range() -> Expr {
    Expr::field(RANGE)
}

fn foothills() -> Expr {
    range().ramp(0.0, FOOTHILLS)
}

/// The `RANGE` field. A range follows a zero line of a large noise, which makes it long and
/// winding, and stands only where a second noise lets it, which makes it rare and lets it fade
/// out at its ends.
pub fn range_field() -> Expr {
    let warp = |axis_salt| Expr::fbm(axis_salt, SPINE_WARP_SIZE, 2) * Expr::blocks(SPINE_WARP);
    let spine = Expr::fbm(salt::SPINE, SPINE_SIZE, 2).at(Expr::x() + warp(salt::SPINE_WARP_X), Expr::z() + warp(salt::SPINE_WARP_Z));
    let gate = Expr::fbm(salt::GATE, GATE_SIZE, 2).ramp(GATE.0, GATE.1);
    spine.abs().ramp(SPINE_REACH, 0.0).and(gate)
}

/// The `RELIEF` field: plains, rolling hills or uplands by region, and always hilly at the foot
/// of a range.
pub fn relief_field() -> Expr {
    Expr::fbm(salt::RELIEF, RELIEF_SIZE, 2)
        .ramp(-0.3, 0.45)
        .max(foothills())
}

/// The `MOUNTAINS` field: patches of the land's inside.
pub fn mountains_field() -> Expr {
    Expr::fbm(salt::MOUNTAINS, 800.0, 3)
        .ramp(0.0, 0.35)
        .and(Expr::continent().ramp(0.05, 0.25))
}

/// The `HIGHLAND` field: snow lies on high mountains and high ranges alike.
pub fn highland_field() -> Expr {
    Expr::field(MOUNTAINS).max(range() * Expr::value(MOUNTAINS_BIOME / RANGE_BIOME))
}

/// Hills as high as `RELIEF` says. Biomes shape the land through their climate, not their
/// borders, so nothing seams: the cold flattens hills into tundra.
pub fn hills() -> Landform {
    let height = Expr::blocks(PLAIN_HILLS) + Expr::field(RELIEF) * Expr::blocks(UPLAND_HILLS - PLAIN_HILLS);
    let warm = Expr::temperature().ramp(-0.35, -0.6).not();
    // The higher part of a range has its own shape.
    let below_ranges = range().ramp(0.55, FOOTHILLS);
    Landform::new(Expr::fbm(salt::HILLS, HILL_SIZE, 4) * height).mask(warm.and(below_ranges))
}

/// Ridges that branch, 0 to about 2.3: each octave is a ridge along its zero lines over a flat
/// floor and counts only where the octaves before it stand, so crests carry the detail and
/// valley floors stay smooth. `at` moves where an octave looks.
fn ridges(salt: u64, size: f64, octaves: u32, at: impl Fn(Expr) -> Expr) -> Expr {
    let ridge = |octave: u32| {
        at(Expr::fbm(salt + u64::from(octave), size / f64::from(1u32 << octave), 1))
            .abs()
            .ramp(RIDGE_WIDTH, 0.0)
    };
    let mut detail = ridge(octaves - 1);
    for octave in (0..octaves - 1).rev() {
        detail = (detail * Expr::value(RIDGE_DETAIL) + Expr::value(1.0)) * ridge(octave);
    }
    detail
}

/// Mountains in massifs, where `MOUNTAINS` says and no range stands.
pub fn mountains() -> Landform {
    // A sixth of the height everywhere, the rest on the ridges (which reach 2.2).
    let shape = Expr::value(0.15) + ridges(salt::MASSIF, MASSIF_RIDGE_SIZE, 4, |noise| noise) * Expr::value(0.4675);
    let roughness = Expr::fbm(salt::ROUGHNESS, 48.0, 3) * Expr::blocks(MASSIF_ROUGHNESS);
    Landform::new(shape * Expr::blocks(MASSIF_HEIGHT) + roughness)
        .mask(Expr::field(MOUNTAINS).and(range().ramp(FOOTHILLS, 0.0)))
        .named("mountains")
}

/// A mountain range: across it foothills, then a rise that steepens toward the spine; along it
/// peaks and saddles; on it branching ridges, and valleys that cut through.
pub fn ranges() -> Landform {
    let core = || range().ramp(0.15, 0.85);
    let across = foothills() * Expr::value(FOOTHILL_SHARE)
        + core() * (Expr::value(0.35) + core() * Expr::value(0.65)) * Expr::value(1.0 - FOOTHILL_SHARE);
    let along = Expr::value(1.0 - SADDLE_DROP) + Expr::fbm(salt::PEAKS, PEAK_SIZE, 2) * Expr::value(SADDLE_DROP);
    let bend = |axis_salt| Expr::fbm(axis_salt, BEND_SIZE, 1) * Expr::blocks(BEND);
    let bent = |noise: Expr| noise.at(Expr::x() + bend(salt::BEND_X), Expr::z() + bend(salt::BEND_Z));
    // Under a third of the height everywhere, the rest on the ridges (which reach 2.3).
    let ridged = Expr::value(0.3) + ridges(salt::RIDGES, RIDGE_SIZE, 5, bent) * Expr::value(0.385);
    // A valley runs along a zero line of its noise with a flat floor; where it crosses the
    // spine it leaves a pass.
    let valleys = bent(Expr::fbm(salt::VALLEYS, VALLEY_SIZE, 3))
        .abs()
        .ramp(0.012, 0.16);
    let cut = Expr::value(VALLEY_FLOOR) + valleys * Expr::value(1.0 - VALLEY_FLOOR);
    let height = (across * along * ridged * cut * Expr::blocks(RANGE_HEIGHT)).min(Expr::blocks(RANGE_CAP));
    // The height is only worked out where there is a range.
    Landform::new(height)
        .mask(range().ramp(0.0, 0.001))
        .named("range")
}
