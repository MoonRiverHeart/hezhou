pub struct MsdfConfig {
    pub padding: u32,
    pub range: f32,
    pub scale: f32,
}

impl Default for MsdfConfig {
    fn default() -> Self {
        Self {
            padding: 4,
            range: 4.0,
            scale: 1.0,
        }
    }
}

pub fn generate_sdf(bitmap: &[u8], width: u32, height: u32, config: &MsdfConfig) -> Vec<u8> {
    let padding = config.padding;
    let range = config.range;

    let padded_width = width + 2 * padding;
    let padded_height = height + 2 * padding;

    let mut sdf = vec![0u8; (padded_width * padded_height) as usize];

    let threshold = 128u8;

    for py in 0..padded_height {
        for px in 0..padded_width {
            let bx = px.saturating_sub(padding);
            let by = py.saturating_sub(padding);

            let bx = if bx >= width { width - 1 } else { bx };
            let by = if by >= height { height - 1 } else { by };

            let center_val = bitmap
                .get(by as usize * width as usize + bx as usize)
                .copied()
                .unwrap_or(0);
            let is_inside = center_val >= threshold;

            let mut min_dist = f32::MAX;

            let search_range = (range * 2.0) as i32;

            for dy in -search_range..=search_range {
                for dx in -search_range..=search_range {
                    let nx = bx as i32 + dx;
                    let ny = by as i32 + dy;

                    if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                        continue;
                    }

                    let neighbor_val = bitmap
                        .get(ny as usize * width as usize + nx as usize)
                        .copied()
                        .unwrap_or(0);
                    let neighbor_inside = neighbor_val >= threshold;

                    if neighbor_inside != is_inside {
                        let dist = ((dx * dx + dy * dy) as f32).sqrt();
                        if dist < min_dist {
                            min_dist = dist;
                        }
                    }
                }
            }

            let normalized_dist = if is_inside {
                -min_dist / range
            } else {
                min_dist / range
            };

            let sdf_value = ((normalized_dist * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

            let idx = py as usize * padded_width as usize + px as usize;
            sdf[idx] = sdf_value;
        }
    }

    sdf
}

pub fn generate_msdf(bitmap: &[u8], width: u32, height: u32, config: &MsdfConfig) -> Vec<u8> {
    let padding = config.padding;
    let range = config.range;

    let padded_width = width + 2 * padding;
    let padded_height = height + 2 * padding;

    let mut msdf = vec![0u8; (padded_width * padded_height * 3) as usize];

    let threshold = 128u8;

    for py in 0..padded_height {
        for px in 0..padded_width {
            let bx = px.saturating_sub(padding);
            let by = py.saturating_sub(padding);

            let bx = if bx >= width { width - 1 } else { bx };
            let by = if by >= height { height - 1 } else { by };

            let center_val = bitmap
                .get(by as usize * width as usize + bx as usize)
                .copied()
                .unwrap_or(0);
            let is_inside = center_val >= threshold;

            let mut distances = [f32::MAX, f32::MAX, f32::MAX];

            let projections = [(1.0, 0.0), (0.5, 0.866), (-0.5, 0.866)];

            let search_range = (range * 2.0) as i32;

            for (channel_idx, (proj_x, proj_y)) in projections.iter().enumerate() {
                let mut min_dist = f32::MAX;

                for dy in -search_range..=search_range {
                    for dx in -search_range..=search_range {
                        let nx = bx as i32 + dx;
                        let ny = by as i32 + dy;

                        if nx < 0 || nx >= width as i32 || ny < 0 || ny >= height as i32 {
                            continue;
                        }

                        let neighbor_val = bitmap
                            .get(ny as usize * width as usize + nx as usize)
                            .copied()
                            .unwrap_or(0);
                        let neighbor_inside = neighbor_val >= threshold;

                        if neighbor_inside != is_inside {
                            let dist_x = dx as f32;
                            let dist_y = dy as f32;

                            let projected_dist = dist_x * proj_x + dist_y * proj_y;
                            let ortho_dist = (dist_x * dist_x + dist_y * dist_y).sqrt();

                            let dist = ortho_dist.min(projected_dist.abs() * 2.0);

                            if dist < min_dist {
                                min_dist = dist;
                            }
                        }
                    }
                }

                distances[channel_idx] = min_dist;
            }

            let idx = py as usize * padded_width as usize + px as usize;

            for (channel_idx, min_dist) in distances.iter().enumerate() {
                let normalized_dist = if is_inside {
                    -*min_dist / range
                } else {
                    *min_dist / range
                };

                let sdf_value = ((normalized_dist * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                msdf[idx * 3 + channel_idx] = sdf_value;
            }
        }
    }

    msdf
}

pub fn median_of_three(r: f32, g: f32, b: f32) -> f32 {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    r + g + b - max - min
}

pub fn pixel_to_signed_distance(value: u8) -> f32 {
    (value as f32 / 255.0 - 0.5) * 2.0
}
