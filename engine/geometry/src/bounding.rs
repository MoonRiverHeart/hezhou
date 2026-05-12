use nalgebra::{Vector3, Point3};

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

impl BoundingBox {
    pub fn new(min: Vector3<f32>, max: Vector3<f32>) -> Self {
        Self { min, max }
    }

    pub fn empty() -> Self {
        Self {
            min: Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }

    pub fn from_points(points: &[Vector3<f32>]) -> Self {
        if points.is_empty() {
            return Self::empty();
        }

        let mut min = points[0];
        let mut max = points[0];

        for point in &points[1..] {
            min = min.inf(point);
            max = max.sup(point);
        }

        Self { min, max }
    }

    pub fn center(&self) -> Vector3<f32> {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vector3<f32> {
        self.max - self.min
    }

    pub fn extents(&self) -> Vector3<f32> {
        self.size() * 0.5
    }

    pub fn volume(&self) -> f32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    pub fn contains(&self, point: &Vector3<f32>) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
            && point.z >= self.min.z && point.z <= self.max.z
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x
            && self.min.y <= other.max.y && self.max.y >= other.min.y
            && self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    pub fn merge(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox::new(self.min.inf(&other.min), self.max.sup(&other.max))
    }

    pub fn expand(&self, point: &Vector3<f32>) -> BoundingBox {
        BoundingBox::new(self.min.inf(point), self.max.sup(point))
    }

    pub fn transform(&self, matrix: &nalgebra::Matrix4<f32>) -> BoundingBox {
        let corners = self.corners();
        let transformed: Vec<Vector3<f32>> = corners
            .iter()
            .map(|p| {
                let p4 = matrix * nalgebra::Vector4::new(p.x, p.y, p.z, 1.0);
                Vector3::new(p4.x, p4.y, p4.z)
            })
            .collect();
        Self::from_points(&transformed)
    }

    pub fn corners(&self) -> [Vector3<f32>; 8] {
        [
            Vector3::new(self.min.x, self.min.y, self.min.z),
            Vector3::new(self.max.x, self.min.y, self.min.z),
            Vector3::new(self.min.x, self.max.y, self.min.z),
            Vector3::new(self.max.x, self.max.y, self.min.z),
            Vector3::new(self.min.x, self.min.y, self.max.z),
            Vector3::new(self.max.x, self.min.y, self.max.z),
            Vector3::new(self.min.x, self.max.y, self.max.z),
            Vector3::new(self.max.x, self.max.y, self.max.z),
        ]
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingSphere {
    pub center: Vector3<f32>,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(center: Vector3<f32>, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn empty() -> Self {
        Self {
            center: Vector3::zeros(),
            radius: 0.0,
        }
    }

    pub fn from_bounding_box(bbox: &BoundingBox) -> Self {
        let center = bbox.center();
        let radius = (bbox.max - center).magnitude();
        Self { center, radius }
    }

    pub fn contains(&self, point: &Vector3<f32>) -> bool {
        (self.center - point).magnitude_squared() <= self.radius * self.radius
    }

    pub fn intersects(&self, other: &BoundingSphere) -> bool {
        let distance_sq = (self.center - other.center).magnitude_squared();
        let radius_sum = self.radius + other.radius;
        distance_sq <= radius_sum * radius_sum
    }

    pub fn intersects_bbox(&self, bbox: &BoundingBox) -> bool {
        let closest = Vector3::new(
            self.center.x.clamp(bbox.min.x, bbox.max.x),
            self.center.y.clamp(bbox.min.y, bbox.max.y),
            self.center.z.clamp(bbox.min.z, bbox.max.z),
        );
        let distance_sq = (self.center - closest).magnitude_squared();
        distance_sq <= self.radius * self.radius
    }

    pub fn merge(&self, other: &BoundingSphere) -> BoundingSphere {
        let center = (self.center + other.center) * 0.5;
        let distance = (self.center - other.center).magnitude();
        let radius = (self.radius.max(other.radius) + distance) * 0.5;
        Self { center, radius }
    }
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self::empty()
    }
}