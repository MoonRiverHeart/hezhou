use hezhou_core::math::Vec3;

#[derive(Clone, Debug)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
            max_distance: f32::INFINITY,
        }
    }
    
    pub fn with_max_distance(origin: Vec3, direction: Vec3, max_distance: f32) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
            max_distance,
        }
    }
    
    pub fn origin(&self) -> &Vec3 {
        &self.origin
    }
    
    pub fn direction(&self) -> &Vec3 {
        &self.direction
    }
    
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }
    
    pub fn point_at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }
}

impl Default for Ray {
    fn default() -> Self {
        Self::new(Vec3::zero(), Vec3::forward())
    }
}

#[derive(Clone, Debug)]
pub struct RaycastResult {
    hit: bool,
    point: Vec3,
    normal: Vec3,
    distance: f32,
    body_id: Option<u64>,
    collider_id: Option<u64>,
}

impl RaycastResult {
    pub fn miss() -> Self {
        Self {
            hit: false,
            point: Vec3::zero(),
            normal: Vec3::zero(),
            distance: f32::INFINITY,
            body_id: None,
            collider_id: None,
        }
    }
    
    pub fn hit(point: Vec3, normal: Vec3, distance: f32) -> Self {
        Self {
            hit: true,
            point,
            normal,
            distance,
            body_id: None,
            collider_id: None,
        }
    }
    
    pub fn with_body(mut self, body_id: u64) -> Self {
        self.body_id = Some(body_id);
        self
    }
    
    pub fn with_collider(mut self, collider_id: u64) -> Self {
        self.collider_id = Some(collider_id);
        self
    }
    
    pub fn is_hit(&self) -> bool {
        self.hit
    }
    
    pub fn point(&self) -> &Vec3 {
        &self.point
    }
    
    pub fn normal(&self) -> &Vec3 {
        &self.normal
    }
    
    pub fn distance(&self) -> f32 {
        self.distance
    }
    
    pub fn body_id(&self) -> Option<u64> {
        self.body_id
    }
    
    pub fn collider_id(&self) -> Option<u64> {
        self.collider_id
    }
}

impl Default for RaycastResult {
    fn default() -> Self {
        Self::miss()
    }
}