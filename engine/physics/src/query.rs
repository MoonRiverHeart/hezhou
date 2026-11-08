//! 物理世界查询：raycast / sphere cast / 胶囊扫掠 / overlap / 穿透解决。
//!
//! 这些接口是 `CharacterMotor`、相机碰撞、命中盒判定与触发器检测的公共基础。
//! 所有查询返回统一的命中结构，包含 hit point、normal、distance、body/collider id。

use crate::collider::ColliderShape;
use crate::intersect::{self, TargetShape};
use crate::raycast::{Ray, RaycastResult};
use crate::world::{ColliderEntry, PhysicsWorld};
use hezhou_core::math::Vec3;

/// 查询过滤器：层掩码 + 是否包含触发器 + 排除自身。
#[derive(Clone, Copy, Debug)]
pub struct QueryFilter {
    layer_mask: u32,
    include_sensors: bool,
    exclude_body: Option<u64>,
    exclude_collider: Option<u64>,
}

impl QueryFilter {
    /// 命中所有普通碰撞体（忽略传感器）。
    pub fn new() -> Self {
        Self {
            layer_mask: u32::MAX,
            include_sensors: false,
            exclude_body: None,
            exclude_collider: None,
        }
    }

    /// 指定层掩码（只命中这些层）。
    pub fn with_layer_mask(mut self, layer_mask: u32) -> Self {
        self.layer_mask = layer_mask;
        self
    }

    /// 是否把传感器（trigger）纳入查询结果。
    pub fn with_sensors(mut self, include_sensors: bool) -> Self {
        self.include_sensors = include_sensors;
        self
    }

    /// 排除某个刚体（通常传角色/相机自己的 body id）。
    pub fn exclude_body(mut self, body_id: u64) -> Self {
        self.exclude_body = Some(body_id);
        self
    }

    /// 排除某个碰撞体。
    pub fn exclude_collider(mut self, collider_id: u64) -> Self {
        self.exclude_collider = Some(collider_id);
        self
    }

    pub fn layer_mask(&self) -> u32 {
        self.layer_mask
    }

    pub fn includes_sensors(&self) -> bool {
        self.include_sensors
    }

    pub fn excluded_body(&self) -> Option<u64> {
        self.exclude_body
    }

    pub fn excluded_collider(&self) -> Option<u64> {
        self.exclude_collider
    }

    /// 该碰撞体是否通过过滤器。
    pub(crate) fn accepts(
        &self,
        layer: u32,
        body_id: u64,
        collider_id: u64,
        is_sensor: bool,
    ) -> bool {
        if is_sensor && !self.include_sensors {
            return false;
        }
        if self.layer_mask & layer == 0 {
            return false;
        }
        if self.exclude_body == Some(body_id) {
            return false;
        }
        if self.exclude_collider == Some(collider_id) {
            return false;
        }
        true
    }
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 统一命中结果。
///
/// 字段语义：
/// - `point`：碰撞点（世界空间）；
/// - `normal`：把查询形状推出去的单位法线（射线命中时朝向射线来向）；
/// - `distance`：沿查询方向的距离（扫掠/射线为实际距离，单位为米）；
/// - `body_id` / `collider_id`：命中的刚体/碰撞体 id；
/// - `is_sensor`：命中的是否为触发器。
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    point: Vec3,
    normal: Vec3,
    distance: f32,
    body_id: Option<u64>,
    collider_id: Option<u64>,
    is_sensor: bool,
}

impl Hit {
    pub(crate) fn new(
        point: Vec3,
        normal: Vec3,
        distance: f32,
        body_id: u64,
        collider_id: u64,
        is_sensor: bool,
    ) -> Self {
        Self {
            point,
            normal,
            distance,
            body_id: Some(body_id),
            collider_id: Some(collider_id),
            is_sensor,
        }
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

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }
}

/// 重叠查询结果。
///
/// `depth` 为穿透深度（正值，表示需要把查询形状沿 `normal` 推出多少米）。
#[derive(Clone, Copy, Debug)]
pub struct OverlapHit {
    point: Vec3,
    normal: Vec3,
    depth: f32,
    body_id: u64,
    collider_id: u64,
    is_sensor: bool,
}

impl OverlapHit {
    pub(crate) fn new(
        point: Vec3,
        normal: Vec3,
        depth: f32,
        body_id: u64,
        collider_id: u64,
        is_sensor: bool,
    ) -> Self {
        Self {
            point,
            normal,
            depth,
            body_id,
            collider_id,
            is_sensor,
        }
    }

    pub fn point(&self) -> &Vec3 {
        &self.point
    }

    pub fn normal(&self) -> &Vec3 {
        &self.normal
    }

    pub fn depth(&self) -> f32 {
        self.depth
    }

    pub fn body_id(&self) -> u64 {
        self.body_id
    }

    pub fn collider_id(&self) -> u64 {
        self.collider_id
    }

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
    }
}

fn target_of(entry: &ColliderEntry<'_>) -> Option<TargetShape> {
    match entry.collider.shape() {
        // 旋转盒：查询用保守 AABB（与接触检测一致，宁可多报不漏报）。
        ColliderShape::Box { half_extents } => Some(TargetShape::Box {
            center: entry.position,
            half_extents: crate::intersect::rotated_aabb_half_extents(
                entry.body.rotation(),
                *half_extents,
            ),
        }),
        shape => TargetShape::from_collider(shape, entry.position, entry.plane_normal),
    }
}

impl PhysicsWorld {
    /// 最近命中的射线查询（忽略传感器）。
    pub fn raycast(&self, ray: &Ray) -> Option<RaycastResult> {
        self.raycast_filtered(ray, QueryFilter::default())
    }

    /// 带过滤器的最近命中射线查询。
    pub fn raycast_filtered(&self, ray: &Ray, filter: QueryFilter) -> Option<RaycastResult> {
        let mut best: Option<(f32, RaycastResult)> = None;
        self.each_collider(|entry| {
            if !filter.accepts(
                entry.collider.layer(),
                entry.body_id,
                entry.collider_id,
                entry.collider.is_sensor(),
            ) {
                return;
            }
            let Some(target) = target_of(&entry) else {
                return;
            };
            let Some(hit) =
                intersect::ray_target(*ray.origin(), *ray.direction(), ray.max_distance(), &target)
            else {
                return;
            };
            if best.as_ref().is_none_or(|(d, _)| hit.t < *d) {
                let result = RaycastResult::hit(hit.point, hit.normal, hit.t)
                    .with_body(entry.body_id)
                    .with_collider(entry.collider_id);
                best = Some((hit.t, result));
            }
        });
        best.map(|(_, result)| result)
    }

    /// 所有命中的射线查询，按距离升序。
    pub fn raycast_all(&self, ray: &Ray, filter: QueryFilter) -> Vec<RaycastResult> {
        let mut out: Vec<(f32, RaycastResult)> = Vec::new();
        self.each_collider(|entry| {
            if !filter.accepts(
                entry.collider.layer(),
                entry.body_id,
                entry.collider_id,
                entry.collider.is_sensor(),
            ) {
                return;
            }
            let Some(target) = target_of(&entry) else {
                return;
            };
            let Some(hit) =
                intersect::ray_target(*ray.origin(), *ray.direction(), ray.max_distance(), &target)
            else {
                return;
            };
            let result = RaycastResult::hit(hit.point, hit.normal, hit.t)
                .with_body(entry.body_id)
                .with_collider(entry.collider_id);
            out.push((hit.t, result));
        });
        out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        out.into_iter().map(|(_, result)| result).collect()
    }

    /// 胶囊扫掠：从 `center` 沿 `displacement` 移动，返回最早命中。
    ///
    /// `center` 为胶囊中点（圆柱段中点），`half_height` 为圆柱段半高。
    /// 返回的 `distance` 为沿位移方向的实际距离。
    pub fn sweep_capsule(
        &self,
        center: Vec3,
        displacement: Vec3,
        radius: f32,
        half_height: f32,
        filter: QueryFilter,
    ) -> Option<Hit> {
        let a = center - Vec3::up() * half_height;
        let b = center + Vec3::up() * half_height;
        let len = displacement.magnitude();
        let mut best: Option<(f32, Hit)> = None;
        self.each_collider(|entry| {
            if !filter.accepts(
                entry.collider.layer(),
                entry.body_id,
                entry.collider_id,
                entry.collider.is_sensor(),
            ) {
                return;
            }
            let Some(target) = target_of(&entry) else {
                return;
            };
            let Some(hit) = intersect::sweep_segment_shape(a, b, radius, displacement, &target)
            else {
                return;
            };
            if best.as_ref().is_none_or(|(t, _)| hit.t < *t) {
                let distance = hit.t * len;
                let result = Hit::new(
                    hit.point,
                    hit.normal,
                    distance,
                    entry.body_id,
                    entry.collider_id,
                    entry.collider.is_sensor(),
                );
                best = Some((hit.t, result));
            }
        });
        best.map(|(_, hit)| hit)
    }

    /// 球体扫掠（sphere cast）：相机防穿墙最常用。
    ///
    /// `direction` 会被归一化，`max_distance` 为最大检测距离。
    pub fn sphere_cast(
        &self,
        origin: Vec3,
        radius: f32,
        direction: Vec3,
        max_distance: f32,
        filter: QueryFilter,
    ) -> Option<Hit> {
        let dir = direction.normalized();
        if dir.magnitude() <= f32::EPSILON || max_distance <= 0.0 {
            return None;
        }
        self.sweep_capsule(origin, dir * max_distance, radius, 0.0, filter)
    }

    /// 任意查询形状的重叠查询（命中盒判定入口）。
    ///
    /// 查询形状支持 `Sphere` / `Box` / `Capsule`；`Plane` 只作为静态目标。
    pub fn overlap_shape(
        &self,
        center: Vec3,
        shape: &ColliderShape,
        filter: QueryFilter,
    ) -> Vec<OverlapHit> {
        let mut out = Vec::new();
        self.each_collider(|entry| {
            if !filter.accepts(
                entry.collider.layer(),
                entry.body_id,
                entry.collider_id,
                entry.collider.is_sensor(),
            ) {
                return;
            }
            let Some(target) = target_of(&entry) else {
                return;
            };
            let Some(contact) = intersect::shape_overlap(shape, center, &target) else {
                return;
            };
            out.push(OverlapHit::new(
                contact.query_point,
                contact.normal,
                (-contact.distance).max(0.0),
                entry.body_id,
                entry.collider_id,
                entry.collider.is_sensor(),
            ));
        });
        out
    }

    /// 球体重叠查询。
    pub fn overlap_sphere(
        &self,
        center: Vec3,
        radius: f32,
        filter: QueryFilter,
    ) -> Vec<OverlapHit> {
        self.overlap_shape(center, &ColliderShape::sphere(radius), filter)
    }

    /// 胶囊重叠查询（胶囊中点 + 圆柱段半高 + 半径）。
    pub fn overlap_capsule(
        &self,
        center: Vec3,
        radius: f32,
        half_height: f32,
        filter: QueryFilter,
    ) -> Vec<OverlapHit> {
        self.overlap_shape(
            center,
            &ColliderShape::Capsule {
                half_height,
                radius,
            },
            filter,
        )
    }

    /// 轴对齐盒重叠查询。
    pub fn overlap_box(
        &self,
        center: Vec3,
        half_extents: Vec3,
        filter: QueryFilter,
    ) -> Vec<OverlapHit> {
        self.overlap_shape(center, &ColliderShape::Box { half_extents }, filter)
    }

    /// 脚底地面探测（gameplay 适配层入口）。
    ///
    /// 从“脚底 + 胶囊高度”构成的胶囊向下扫掠 `max_distance`，返回最近命中；
    /// 吸附后脚底位置 = `feet - up * hit.distance()`，`hit.normal()` 为地面法线。
    pub fn ground_probe(
        &self,
        feet: Vec3,
        radius: f32,
        half_height: f32,
        max_distance: f32,
        filter: QueryFilter,
    ) -> Option<Hit> {
        if !(max_distance > 0.0) {
            return None;
        }
        let center = feet + Vec3::up() * (half_height + radius);
        let hit = self.sweep_capsule(
            center,
            Vec3::down() * max_distance,
            radius,
            half_height,
            filter,
        )?;
        if hit.distance() > max_distance {
            return None;
        }
        Some(hit)
    }

    /// 胶囊穿透解决：返回穿透最深的一个接触（含推出法线与深度）。
    ///
    /// 用于角色生成/传送后纠正穿透，`CharacterMotor` 在移动前会先调用它。
    pub fn resolve_capsule_penetration(
        &self,
        center: Vec3,
        radius: f32,
        half_height: f32,
        filter: QueryFilter,
    ) -> Option<OverlapHit> {
        let a = center - Vec3::up() * half_height;
        let b = center + Vec3::up() * half_height;
        let mut best: Option<(f32, OverlapHit)> = None;
        self.each_collider(|entry| {
            if !filter.accepts(
                entry.collider.layer(),
                entry.body_id,
                entry.collider_id,
                entry.collider.is_sensor(),
            ) {
                return;
            }
            let Some(target) = target_of(&entry) else {
                return;
            };
            let contact = intersect::segment_shape_contact(a, b, radius, &target);
            if contact.distance >= 0.0 {
                return;
            }
            let depth = -contact.distance;
            if best.as_ref().is_none_or(|(d, _)| depth > *d) {
                best = Some((
                    depth,
                    OverlapHit::new(
                        contact.query_point,
                        contact.normal,
                        depth,
                        entry.body_id,
                        entry.collider_id,
                        entry.collider.is_sensor(),
                    ),
                ));
            }
        });
        best.map(|(_, hit)| hit)
    }
}
