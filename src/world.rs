use glam;

pub const GRID_SPACING: f32 = 12.0;
pub const BUILDING_WIDTH: f32 = 5.0;
pub const GROUND_LEVEL: f32 = -10.0;
pub const MAX_BUILDING_HEIGHT: f32 = 60.0;

// Deterministic random number generator
pub fn hash(x: i32, z: i32) -> u64 {
    let mut h = (x as u64).wrapping_mul(374761393);
    h = h.wrapping_add((z as u64).wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

pub fn get_building_info(x: i32, z: i32) -> Option<(f32, glam::Vec3)> {
    let block_size = 6;
    let road_width = 1;

    let mx = x.rem_euclid(block_size);
    let mz = z.rem_euclid(block_size);

    if mx < road_width || mz < road_width {
        return None; 
    }

    let h_val = hash(x, z);
    
    if h_val % 100 < 20 {
        return None;
    }

    let height = ((h_val % 45) + 10) as f32; 

    // Pastel
    let r = 0.5 + ((h_val & 0xFF) as f32 / 255.0) * 0.5;
    let g = 0.5 + (((h_val >> 8) & 0xFF) as f32 / 255.0) * 0.5;
    let b = 0.5 + (((h_val >> 16) & 0xFF) as f32 / 255.0) * 0.5;

    Some((height, glam::Vec3::new(r, g, b)))
}

pub fn check_collision(pos: glam::Vec3) -> bool {
    if pos.y < GROUND_LEVEL + 1.0 { return true; }

    let grid_x = (pos.x / GRID_SPACING).round() as i32;
    let grid_z = (pos.z / GRID_SPACING).round() as i32;

    if let Some((height, _)) = get_building_info(grid_x, grid_z) {
        let building_x = grid_x as f32 * GRID_SPACING;
        let building_z = grid_z as f32 * GRID_SPACING;
        
        let dx = (pos.x - building_x).abs();
        let dz = (pos.z - building_z).abs();
        let half_width = BUILDING_WIDTH / 2.0;
        let top_y = height + GROUND_LEVEL;

        if dx < half_width && dz < half_width && pos.y < top_y {
            return true;
        }
    }
    false
}
