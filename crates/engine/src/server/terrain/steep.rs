//! Which columns are too steep to keep their surface: bare stone on cliffs
//! (`BiomeRule::bare_steeper`). Steepness is read off the same corner heights caves measure depth
//! from, one every 4 blocks, so a single step is not a cliff. `shaders/terrain.wgsl` has the
//! same integer math.

use super::*;

/// The steepest slope told apart, in `steepness`'s unit (1024 blocks up per block along): the
/// squares of two of these fit 32 bits.
pub const STEEPEST: i32 = 1 << 14;

/// How steep the surface is at (`x`, `z`) (0..4) of a 4 x 4 cell with corner heights `corners`
/// (as `smooth4` takes them), squared. Its unit is 1/16 block up per block along, so 16 * 16
/// is a slope of 45 degrees.
pub fn steepness(corners: [i32; 4], x: i32, z: i32) -> i32 {
    let [near_west, near_east, far_west, far_east] = corners;
    let along = |rise_here: i32, rise_across: i32, across: i32| {
        rise_here
            .wrapping_mul(4 - across)
            .wrapping_add(rise_across.wrapping_mul(across))
            .clamp(-STEEPEST, STEEPEST)
    };
    let along_x = along(near_east.wrapping_sub(near_west), far_east.wrapping_sub(far_west), z);
    let along_z = along(far_west.wrapping_sub(near_west), far_east.wrapping_sub(near_east), x);
    along_x * along_x + along_z * along_z
}

/// `steepness` at column (`x`, `z`) of a cube, from the cube's corner heights.
pub fn in_cube(corners: &[i32; 25], x: usize, z: usize) -> i32 {
    let (cell_x, cell_z) = (x / 4, z / 4);
    let corner = |dx: usize, dz: usize| corners[(cell_z + dz) * 5 + cell_x + dx];
    steepness([corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1)], (x % 4) as i32, (z % 4) as i32)
}

impl Terrain {
    /// Column heights at the corners of the 4 x 4 cell whose lowest corner is (`x`, `z`).
    pub(super) fn cell(&self, seed: u64, x: i32, z: i32) -> [i32; 4] {
        [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dz)| self.column(seed, x + dx * 4, z + dz * 4).height)
    }

    /// `steepness` at column (`x`, `z`), as cube generation has it.
    pub(super) fn steepness_at(&self, seed: u64, x: i32, z: i32) -> i32 {
        let (cell_x, cell_z) = (x.div_euclid(4) * 4, z.div_euclid(4) * 4);
        steepness(self.cell(seed, cell_x, cell_z), x - cell_x, z - cell_z)
    }

    /// Whether rule `rule` leaves a column `height` high as plain stone. `steepness` is only
    /// asked where the rule cares about it.
    pub(super) fn bare(&self, rule: u32, height: i32, steepness: impl FnOnce() -> i32) -> bool {
        let Some(rule) = self.rules.get(rule as usize) else { return false };
        rule.bare_above.is_some_and(|above| height > above)
            || rule
                .bare_steeper
                .is_some_and(|steeper| steepness() > steeper * steeper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: u32 = 1;
    const GRASS: u32 = 2;

    /// Ground that rises `rise` blocks per block toward +x, grass unless steeper than 1.
    fn slope(rise: f64) -> Terrain {
        let step = |kind, a| Step { kind, salt: 0, a, b: 0, c: 0 };
        let height = Expr(vec![step(ek::X, 0), step(ek::CONST, q12(rise)), step(ek::MUL, 0)]);
        let rule = Rule {
            biome: 0,
            ranges: Vec::new(),
            height: (i32::MIN, i32::MAX),
            height_from: None,
            top: Some(GRASS),
            filler: None,
            depth: 0,
            under: None,
            deep_top: None,
            bare_above: None,
            bare_steeper: Some(q4(1.0)),
            ice: None,
        };
        let feature = Feature { name: String::new(), height, mask: None };
        Terrain::new(STONE, q4(64.0), None, Vec::new(), vec![feature], vec![rule], None, Vec::new())
    }

    fn surface(terrain: &Terrain, x: i32) -> u32 {
        terrain.surface_block(0, x, 5)
    }

    #[test]
    fn steepness_is_the_rise_per_block() {
        assert_eq!(steepness([0, 4, 0, 4], 1, 2), 16 * 16, "one block up per block along x");
        assert_eq!(steepness([0, 0, 8, 8], 3, 0), 32 * 32, "two blocks up per block along z");
        assert_eq!(steepness([7, 7, 7, 7], 2, 2), 0, "flat ground");
        assert_eq!(steepness([0, 4, 4, 8], 2, 2), 2 * 16 * 16, "a diagonal slope counts both ways");
    }

    #[test]
    fn huge_steps_stay_in_range() {
        let wall = 1 << 20;
        assert_eq!(steepness([0, wall, 0, wall], 0, 0), STEEPEST * STEEPEST, "held to the steepest");
    }

    #[test]
    fn cliffs_are_bare() {
        assert_eq!(surface(&slope(0.5), 40), GRASS, "a gentle slope keeps its surface");
        assert_eq!(surface(&slope(2.0), 40), STONE, "a cliff is plain stone");
    }

    #[test]
    fn cubes_and_reads_agree_on_cliffs() {
        let terrain = slope(2.0);
        let (x, z) = (40, 5);
        let height = terrain.column(0, x, z).height;
        let mut blocks = vec![0u32; 4096];
        terrain.generate(0, [x >> 4, height >> 4, z >> 4], &mut blocks, &mut [0; 256]);
        let in_cube = |v: i32| (v & 15) as usize;
        let generated = blocks[(in_cube(height) * 16 + in_cube(z)) * 16 + in_cube(x)];
        assert_eq!(generated, STONE, "the cube has the cliff's stone");
        assert_eq!(terrain.read(0, x, height, z), generated, "a read sees what the cube has");
    }
}
