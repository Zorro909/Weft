//! Erosion: gullies cut into a terrain's slopes once its landforms are added up
//! (`weft_noise::gullies`). A column's slope is how much the ground before erosion rises over
//! `gullies::SPAN` blocks to the east and to the south, so no column needs its neighbours' cuts.

use weft_noise::gullies::{SPAN, gullies};

use super::*;

/// The deepest gullies, Q4.
const DEEPEST: i32 = 1 << 14;

/// Gullies `size` apart and about `depth` deep (Q4), at the strength of `mask` (0 to `ONE`;
/// `None`: at full strength everywhere).
pub struct Erosion {
    pub size: i32,
    pub depth: i32,
    pub mask: Option<Expr>,
}

impl Erosion {
    /// What `described` asks for; `field` finds the fields its mask may use by name.
    pub(super) fn resolve(described: &abi::TerrainErosion, field: &dyn Fn(&str) -> Option<usize>) -> Result<Erosion, String> {
        let mask = (!described.mask.is_empty())
            .then(|| Expr::resolve(&described.mask, field))
            .transpose()?;
        Ok(Erosion { size: q4(described.size), depth: q4(described.depth).clamp(0, DEEPEST), mask })
    }

    pub(super) fn uses(&self, field: usize) -> bool {
        self.mask.as_ref().is_some_and(|mask| mask.uses(field))
    }
}

impl Terrain {
    /// The fields at (`x`, `z`) (Q4) and the height (Q4) the landforms add up to there.
    pub(super) fn ground(&self, seed: u64, x: i32, z: i32) -> ([i32; limit::FIELDS as usize], i32) {
        let mut fields = [0i32; limit::FIELDS as usize];
        for (i, field) in self.fields.iter().enumerate() {
            fields[i] = field.expr.eval(&fields, seed, x, z);
        }
        let mut height = self.base;
        for feature in &self.features {
            let strength = feature
                .mask
                .as_ref()
                .map_or(ONE, |mask| mask.eval(&fields, seed, x, z).min(ONE));
            if strength > 0 {
                let added = feature
                    .height
                    .eval(&fields, seed, x, z)
                    .clamp(-HEIGHT_LIMIT, HEIGHT_LIMIT);
                height = height.wrapping_add((strength * added) >> 12);
            }
        }
        (fields, height)
    }

    /// How far (Q4) erosion cuts into the ground at (`x`, `z`) (Q4), which is `height` high (Q4)
    /// before it and where the fields are `fields`.
    pub(super) fn cut(&self, seed: u64, x: i32, z: i32, fields: &[i32], height: i32) -> i32 {
        let Some(erosion) = &self.erosion else { return 0 };
        let strength = erosion
            .mask
            .as_ref()
            .map_or(ONE, |mask| mask.eval(fields, seed, x, z).min(ONE));
        if strength <= 0 {
            return 0;
        }
        let rise = |x: i32, z: i32| self.ground(seed, x, z).1.wrapping_sub(height);
        let (east, south) = (rise(x + SPAN * 16, z), rise(x, z + SPAN * 16));
        (gullies(seed32(seed, salt::EROSION), x, z, east, south, erosion.size, erosion.depth) * strength) >> 12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: u32 = 1;

    /// Ground that rises `rise` blocks per block toward +x, with gullies 40 blocks apart and 16 deep.
    fn slope(rise: f64) -> Terrain {
        let step = |kind, a| Step { kind, salt: 0, a, b: 0, c: 0 };
        let height = Expr(vec![step(ek::X, 0), step(ek::CONST, q12(rise)), step(ek::MUL, 0)]);
        let feature = Feature { name: String::new(), height, mask: None };
        let mut terrain = Terrain::new(STONE, q4(64.0), None, Vec::new(), vec![feature], Vec::new(), None, Vec::new());
        terrain.erosion = Some(Erosion { size: q4(40.0), depth: q4(16.0), mask: None });
        terrain
    }

    #[test]
    fn slopes_get_gullies_and_flats_none() {
        let heights = |terrain: &Terrain| -> Vec<i32> { (0..200).map(|z| terrain.column(1, 40, z).height).collect() };
        let (steep, flat) = (heights(&slope(1.5)), heights(&slope(0.0)));
        assert!(flat.iter().all(|&height| height == 64), "flat ground is cut");
        let (lowest, highest) = (steep.iter().min().unwrap(), steep.iter().max().unwrap());
        assert!(*highest <= 64 + 60, "erosion raised the ground to {highest}");
        assert!(highest - lowest >= 8, "gullies only {} deep", highest - lowest);
    }

    #[test]
    fn a_mask_holds_erosion_off() {
        let mut terrain = slope(1.5);
        terrain.erosion.as_mut().unwrap().mask = Some(Expr(vec![Step { kind: ek::CONST, salt: 0, a: 0, b: 0, c: 0 }]));
        assert!((0..200).all(|z| terrain.column(1, 40, z).height == 64 + 60), "a mask of 0 still cuts");
    }

    /// A cube's columns and a single column are worked out the same.
    #[test]
    fn cubes_and_columns_agree() {
        let terrain = slope(1.2);
        let columns = terrain.columns(5, 2, -1);
        for i in 0..256 {
            let (x, z) = (32 + (i % 16) as i32, -16 + (i / 16) as i32);
            assert_eq!(columns.cols[i].height, terrain.compute(5, x, z).0.height, "at {x} {z}");
        }
    }
}
