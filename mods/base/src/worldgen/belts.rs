//! The belts up a mountain: forest, meadow, scree and rock, snow and ice. Each starts at a line
//! that is a field of the terrain, a height that drifts and frays, so none runs along a contour.

use weft_sdk::terrain::*;

use super::relief::{HIGHLAND, MOUNTAINS, MOUNTAINS_BIOME, RANGE, RANGE_BIOME, VALLEY};

/// Field: the y where snow starts, a length.
pub const SNOW_LINE: &str = "base:snow_line";
/// Field: the y where a range's meadow gives way to bare rock, a length.
pub const ROCK_LINE: &str = "base:rock_line";
/// Field: noise whose high patches are scree.
pub const SCREE: &str = "base:scree";

/// The snow line in a temperate climate (temperature 0).
const SNOW_AT: f64 = 280.0;
/// Air cools with height: the snow line is this many blocks higher for every step of temperature.
const LAPSE: f64 = 150.0;
/// The snow line of the coldest and of the hottest climates, before it frays.
const SNOW_FLOOR: f64 = 165.0;
const SNOW_TOP: f64 = 520.0;
/// The snow line frays into patches this many blocks across, which move it by up to this many.
const SNOW_PATCH: f64 = 32.0;
const SNOW_FRAY: f64 = 60.0;

/// Bare rock starts at this y in a range and at this y on a massif, where nothing frays it.
const RANGE_ROCK: i32 = 250;
const MASSIF_ROCK: i32 = 130;
/// The rock line frays into patches this many blocks across, which move it by up to this many.
const ROCK_PATCH: f64 = 44.0;
const ROCK_FRAY: f64 = 45.0;
/// A range's forest ends this many blocks under its rock; between them is meadow.
const MEADOW: i32 = 50;

/// Scree lies in patches this many blocks across, from this far under the rock line to this far
/// above it, where `SCREE` is at least `SCREE_FROM` (about a third of the ground).
const SCREE_PATCH: f64 = 26.0;
const SCREE_BELT: (i32, i32) = (-30, 16);
const SCREE_FROM: f64 = 0.12;

/// A glacier fills a valley this close to its line (`VALLEY`), from this far above the snow line.
const GLACIER_WIDTH: f64 = 0.035;
const GLACIER_ABOVE_SNOW: i32 = 10;
const GLACIER_DEPTH: i32 = 5;

/// A noise in patches `size` across with finer ones a quarter of that on it, about -1..1.
fn fray(size: f64) -> Expr {
    Expr::noise(size) + Expr::noise(size / 4.0) * Expr::value(0.5)
}

/// The `SNOW_LINE` field: higher in warm and wet climates, drifting over a mountain's width.
pub fn snow_line() -> Expr {
    let climate = Expr::temperature() + Expr::noise(400.0) * Expr::value(0.2) + Expr::noise(70.0) * Expr::value(0.1)
        - Expr::humidity() * Expr::value(0.1);
    (Expr::blocks(SNOW_AT) + climate * Expr::blocks(LAPSE)).clamp(Expr::blocks(SNOW_FLOOR), Expr::blocks(SNOW_TOP))
        + fray(SNOW_PATCH) * Expr::blocks(SNOW_FRAY)
}

/// The `ROCK_LINE` field.
pub fn rock_line() -> Expr {
    Expr::blocks(f64::from(RANGE_ROCK)) + fray(ROCK_PATCH) * Expr::blocks(ROCK_FRAY)
}

/// The `SCREE` field.
pub fn scree() -> Expr {
    Expr::noise(SCREE_PATCH)
}

/// The rules of everything above a range's forest and a massif's grass, in the order they go
/// into the terrain: ice and snow, then scree, rock and meadow.
pub fn rules() -> Vec<BiomeRule> {
    let range = |name: &str| {
        BiomeRule::new("range_alpine")
            .named(name)
            .field(RANGE, RANGE_BIOME..)
    };
    let massif = |name: &str| {
        BiomeRule::new("mountains")
            .named(name)
            .field(MOUNTAINS, MOUNTAINS_BIOME..)
    };
    let massif_rock = MASSIF_ROCK - RANGE_ROCK;
    let gravel = |rule: BiomeRule| {
        rule.field(SCREE, SCREE_FROM..)
            .surface("gravel", "gravel", 2)
    };
    let stone = |rule: BiomeRule| rule.surface("stone", "stone", 0);
    vec![
        BiomeRule::new("snowy_peaks")
            .named("glacier")
            .field(RANGE, RANGE_BIOME..)
            .field(VALLEY, ..GLACIER_WIDTH)
            .height_from(SNOW_LINE, GLACIER_ABOVE_SNOW..)
            .surface("ice", "ice", GLACIER_DEPTH),
        BiomeRule::new("snowy_peaks")
            .named("snow_line")
            .field(HIGHLAND, MOUNTAINS_BIOME..)
            .height_from(SNOW_LINE, 0..),
        gravel(range("range_scree").height_from(ROCK_LINE, SCREE_BELT.0..=SCREE_BELT.1)),
        stone(range("range_rock").height_from(ROCK_LINE, 0..)),
        range("range_alpine").height_from(ROCK_LINE, -MEADOW..),
        gravel(massif("mountains_scree").height_from(ROCK_LINE, massif_rock + SCREE_BELT.0..=massif_rock + SCREE_BELT.1)),
        stone(massif("mountains_rock").height_from(ROCK_LINE, massif_rock..)),
    ]
}
