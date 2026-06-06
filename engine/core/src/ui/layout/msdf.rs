use ttf_parser::{Face, GlyphId, OutlineBuilder};

pub struct MsdfGenerator {
    pub size: u32,
    pub spread: f32,
}

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

struct DistanceField {
    data: Vec<f32>,
    width: u32,
    height: u32,
}

impl DistanceField {
    fn new(width: u32, height: u32) -> Self {
        DistanceField {
            data: vec![f32::MAX; (width * height) as usize],
            width,
            height,
        }
    }
    
    fn set(&mut self, x: u32, y: u32, value: f32) {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] = self.data[(y * self.width + x) as usize].min(value);
        }
    }
    
    fn get(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            f32::MAX
        } else {
            self.data[(y as u32 * self.width + x as u32) as usize]
        }
    }
}

fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    
    if len2 < 0.0001 {
        let dx = px - ax;
        let dy = py - ay;
        return (dx * dx + dy * dy).sqrt();
    }
    
    let t = ((px - ax) * dx + (py - ay) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    
    let near_x = ax + t * dx;
    let near_y = ay + t * dy;
    let dx = px - near_x;
    let dy = py - near_y;
    (dx * dx + dy * dy).sqrt()
}

struct ContourCollector {
    contours: Vec<Vec<Point>>,
    current: Vec<Point>,
}

impl ContourCollector {
    fn new() -> Self {
        ContourCollector {
            contours: Vec::new(),
            current: Vec::new(),
        }
    }
}

impl OutlineBuilder for ContourCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
        self.current.push(Point { x, y });
    }
    
    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push(Point { x, y });
    }
    
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        if let Some(last) = self.current.last().copied() {
            let steps = 4;
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let u = 1.0 - t;
                let px = u * u * last.x + 2.0 * u * t * x1 + t * t * x;
                let py = u * u * last.y + 2.0 * u * t * y1 + t * t * y;
                self.current.push(Point { x: px, y: py });
            }
        }
    }
    
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // 三次贝塞尔简化为二次
        self.quad_to(x1, y1, x, y);
    }
    
    fn close(&mut self) {
        if let Some(first) = self.current.first().copied() {
            self.current.push(first);
        }
    }
}

impl MsdfGenerator {
    pub fn new(size: u32, spread: f32) -> Self {
        MsdfGenerator { size, spread }
    }
    
    pub fn generate(&self, font_data: &[u8], glyph_id: u16, px_size: f32) -> Vec<u8> {
        let face = Face::parse(font_data, 0).unwrap_or_else(|e| {
            panic!("Failed to parse font: {:?}", e);
        });
        let glyph_id = GlyphId(glyph_id);
        let units_per_em = face.units_per_em() as f32;
        let scale = px_size / units_per_em;
        
        let mut collector = ContourCollector::new();
        face.outline_glyph(glyph_id, &mut collector);
        
        if !collector.current.is_empty() {
            collector.contours.push(std::mem::take(&mut collector.current));
        }
        
        if collector.contours.is_empty() {
            return vec![0u8; (self.size * self.size * 4) as usize];
        }
        
        let padding = self.spread;
        let total_size = self.size as f32;
        let scaled_size = total_size - 2.0 * padding;
        
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        
        for contour in &collector.contours {
            for p in contour {
                let px = p.x * scale;
                let py = p.y * scale;
                min_x = min_x.min(px);
                min_y = min_y.min(py);
                max_x = max_x.max(px);
                max_y = max_y.max(py);
            }
        }
        
        let glyph_w = max_x - min_x;
        let glyph_h = max_y - min_y;
        
        if glyph_w < 0.001 || glyph_h < 0.001 {
            return vec![0u8; (self.size * self.size * 4) as usize];
        }
        
        let scale_to_fit = scaled_size / glyph_w.max(glyph_h);
        
        let mut df = DistanceField::new(self.size, self.size);
        
        for contour in &collector.contours {
            let points: Vec<Point> = contour.iter().map(|p| Point {
                x: (p.x * scale - min_x) * scale_to_fit + padding,
                y: (p.y * scale - min_y) * scale_to_fit + padding,
            }).collect();
            
            for y in 0..self.size as i32 {
                for x in 0..self.size as i32 {
                    let mut min_dist = f32::MAX;
                    
                    for i in 0..points.len() - 1 {
                        let d = dist_to_segment(
                            x as f32, y as f32,
                            points[i].x, points[i].y,
                            points[i + 1].x, points[i + 1].y,
                        );
                        min_dist = min_dist.min(d);
                    }
                    
                    let inside = self.is_inside(x as f32, y as f32, &points);
                    let signed_dist = if inside { -min_dist } else { min_dist };
                    
                    df.set(x as u32, y as u32, signed_dist);
                }
            }
        }
        
        let mut msdf = vec![0u8; (self.size * self.size * 4) as usize];
        for y in 0..self.size {
            for x in 0..self.size {
                let d = df.get(x as i32, y as i32);
                let normalized = (d / self.spread + 1.0) * 0.5;
                let value = (normalized.clamp(0.0, 1.0) * 255.0) as u8;
                let idx = ((y * self.size + x) * 4) as usize;
                msdf[idx] = 255;
                msdf[idx + 1] = 255;
                msdf[idx + 2] = 255;
                msdf[idx + 3] = value;
            }
        }
        
        msdf
    }
    
    fn is_inside(&self, px: f32, py: f32, polygon: &[Point]) -> bool {
        let mut inside = false;
        let n = polygon.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let yi = polygon[i].y;
            let yj = polygon[j].y;
            if (yi > py) != (yj > py) {
                let x = polygon[j].x + (py - yj) / (yi - yj) * (polygon[i].x - polygon[j].x);
                if px < x {
                    inside = !inside;
                }
            }
        }
        inside
    }
}