//! physics ↔ gameplay 集成适配层。
//!
//! gameplay-owner 已在 `hezhou-gameplay` 的 `physics` feature 中实现
//! [`PhysicsQuery`](hezhou_gameplay::PhysicsQuery) / [`PhysicsHitQuery`](hezhou_gameplay::PhysicsHitQuery)，
//! 把 gameplay 的 4 个查询 trait 接到 `hezhou-physics` 定稿 API 上。
//! **本模块不重复实现这些查询 trait**，只做集成层职责：
//!
//! 1. **组合**：持有 `PhysicsWorld` + `FixedTimestep`，提供固定步推进与查询入口；
//! 2. **映射**：[`CombatantBodyMap`] 维护 `CombatantId ↔ body_id`，命中盒结果按映射后的
//!    `CombatantId` 去重（同一个 combatant 挂多个碰撞体时只返回一次）；
//! 3. **过滤透传**：角色/相机/命中查询各自带 `QueryFilter`，支持层掩码、排除自身、
//!    包含触发器；查询时按需把"自身 body"从过滤器中排除；
//! 4. **单一积分来源**：角色移动由 gameplay 的 `CharacterMotor` 积分（它内部只调用
//!    `move_capsule` 查询做扫掠）；physics 的 `step` 只推进**动态刚体**（道具/敌人）。
//!    本适配器**不会**调用 `physics::CharacterMotor::step`，也不会对角色 body 做二次积分；
//!    角色 body 使用 `Kinematic` 类型，位置由 [`PhysicsCombatAdapter::sync_character_transform`]
//!    从 gameplay motor 同步过来。
//!
//! ## 每帧顺序（与 README 一致）
//!
//! ```text
//! let fixed_dt = adapter.timestep().step();
//! let output = gameplay_motor.step(fixed_dt, &input, &adapter.character_query_for(player));
//! adapter.sync_character_transform(player, output.position); // 角色 body 只做同步，不积分
//! let result = adapter.advance(frame_dt);                     // 只推进动态刚体
//! let contacts = hitbox_runtime.detect(&adapter.combat_hit_query_for(player), &hitbox, &pose);
//! // contacts -> DamageEvent::from_contact -> resolve_damage(...)
//! ```
//!
//! ## 已知限制（physics 0.2 轴对齐形状）
//!
//! - 旋转后的 Box/胶囊命中盒会退化为"世界中心 + 轴对齐尺寸"，斜向命中盒偏保守；
//! - 胶囊轴固定世界 Y 轴，适用于直立角色；不支持倾斜胶囊；
//! - 一个刚体最多挂一个碰撞体（physics 0.2 约定），多命中盒需求需拆多个 body；
//! - physics `step` 只做动态刚体积分，不做 CCD/关节/布娃娃（0.3）。

use std::collections::HashMap;

use hezhou_gameplay::{
    capsule_half_height, from_core_vec3, to_core_vec3, CombatantId, HitContact, HitDetectionQuery,
    Hitbox, HitboxPose, PhysicsHitQuery, PhysicsQuery, Vec3,
};
use hezhou_physics::{
    layers, ColliderShape, FixedTimestep, PhysicsWorld, QueryFilter, RigidBodyType,
};

/// 一次固定步推进的结果。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FixedStepResult {
    /// 本次实际执行的固定步数。
    pub steps: u32,
    /// 渲染插值系数（0..=1），由 `FixedTimestep` 的累加余量得到。
    pub alpha: f32,
    /// 本次推进的模拟时间（`steps * fixed_step`）。
    pub simulated_time: f32,
}

impl FixedStepResult {
    /// 构造结果。
    pub fn new(steps: u32, alpha: f32, fixed_step: f32) -> Self {
        Self {
            steps,
            alpha,
            simulated_time: steps as f32 * fixed_step,
        }
    }

    /// 本帧是否没有执行固定步（高刷新率下常见）。
    pub fn is_empty(&self) -> bool {
        self.steps == 0
    }
}

/// `CombatantId ↔ 物理 body_id` 双向映射表。
///
/// - gameplay 侧用 [`CombatantId`] 引用战斗目标（可对应 ECS Entity / 逻辑 id）；
/// - physics 侧用 `u64` body_id；
/// - 一个 combatant 可以注册**多个** body（多部位命中盒 / 武器 body），
///   因此命中盒结果按映射后的 `CombatantId` 去重后同一目标只出现一次；
/// - 未注册的 body 在 [`CombatantBodyMap::resolve`] 中回退为 `CombatantId::from(body_id)`，
///   与 gameplay `PhysicsHitQuery` 的默认行为保持一致（便于混合使用）。
#[derive(Clone, Debug, Default)]
pub struct CombatantBodyMap {
    body_to_combatant: HashMap<u64, CombatantId>,
    combatant_to_bodies: HashMap<CombatantId, Vec<u64>>,
}

impl CombatantBodyMap {
    /// 空映射表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册映射（一个 combatant 可注册多个 body）。
    ///
    /// 返回 `true` 表示 `body_id` 之前未注册、本次新增；重复注册返回 `false`。
    /// 同一个 body 只能属于一个 combatant（会从旧 combatant 的列表移除）。
    pub fn register(&mut self, combatant: CombatantId, body_id: u64) -> bool {
        // body 之前属于别的 combatant：从旧列表移除。
        if let Some(previous) = self.body_to_combatant.insert(body_id, combatant) {
            if previous != combatant {
                self.detach_body(previous, body_id);
            }
        }
        let bodies = self.combatant_to_bodies.entry(combatant).or_default();
        if bodies.contains(&body_id) {
            return false;
        }
        bodies.push(body_id);
        true
    }

    /// 解除 combatant 的全部映射，返回被移除的 body 列表。
    pub fn unregister_combatant(&mut self, combatant: CombatantId) -> Vec<u64> {
        let bodies = self
            .combatant_to_bodies
            .remove(&combatant)
            .unwrap_or_default();
        for body_id in &bodies {
            if self.body_to_combatant.get(body_id) == Some(&combatant) {
                self.body_to_combatant.remove(body_id);
            }
        }
        bodies
    }

    /// 解除单个 body 的映射，返回其 combatant。
    pub fn unregister_body(&mut self, body_id: u64) -> Option<CombatantId> {
        let combatant = self.body_to_combatant.remove(&body_id)?;
        self.detach_body(combatant, body_id);
        Some(combatant)
    }

    /// body → combatant（仅已注册时返回 `Some`）。
    pub fn combatant_of_body(&self, body_id: u64) -> Option<CombatantId> {
        self.body_to_combatant.get(&body_id).copied()
    }

    /// combatant 的**第一个** body（便捷方法；多部位用 [`Self::bodies_of_combatant`]）。
    pub fn body_of_combatant(&self, combatant: CombatantId) -> Option<u64> {
        self.combatant_to_bodies
            .get(&combatant)
            .and_then(|bodies| bodies.first().copied())
    }

    /// combatant 注册的全部 body。
    pub fn bodies_of_combatant(&self, combatant: CombatantId) -> &[u64] {
        self.combatant_to_bodies
            .get(&combatant)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// 解析 body 对应的 combatant；未注册时回退为 `CombatantId::from(body_id)`。
    pub fn resolve(&self, body_id: u64) -> CombatantId {
        self.combatant_of_body(body_id)
            .unwrap_or_else(|| CombatantId::from(body_id))
    }

    /// 已注册的 body 数量。
    pub fn len(&self) -> usize {
        self.body_to_combatant.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.body_to_combatant.is_empty()
    }

    /// 已注册的 combatant 数量。
    pub fn combatant_count(&self) -> usize {
        self.combatant_to_bodies.len()
    }

    /// 从某 combatant 的 body 列表移除一个 body（列表为空时删除该 combatant）。
    fn detach_body(&mut self, combatant: CombatantId, body_id: u64) {
        if let Some(bodies) = self.combatant_to_bodies.get_mut(&combatant) {
            bodies.retain(|registered| *registered != body_id);
            if bodies.is_empty() {
                self.combatant_to_bodies.remove(&combatant);
            }
        }
    }
}

/// 角色 body 的附加信息（用于把 gameplay motor 的脚底位置同步到 physics）。
#[derive(Clone, Copy, Debug)]
struct CharacterBody {
    body_id: u64,
    radius: f32,
    height: f32,
}

/// physics ↔ gameplay 组合适配器。
///
/// 持有 `PhysicsWorld` + `FixedTimestep` + [`CombatantBodyMap`]，提供：
/// - 角色/相机/命中盒查询入口（复用 gameplay `physics` feature 的适配器）；
/// - 固定步推进（只推进动态刚体）；
/// - 角色 kinematic body 与 gameplay motor 的位置同步。
///
/// **注意**：本类型不包含 gameplay 的 `CharacterMotor` 状态机；调用方持有它，
/// 用 [`Self::character_query_for`] 注入物理查询，再把输出位置交给
/// [`Self::sync_character_transform`]。这样保证只有 gameplay motor 一个积分来源。
pub struct PhysicsCombatAdapter {
    world: PhysicsWorld,
    timestep: FixedTimestep,
    bodies: CombatantBodyMap,
    characters: HashMap<CombatantId, CharacterBody>,
    character_filter: QueryFilter,
    camera_filter: QueryFilter,
    hit_filter: QueryFilter,
}

impl Default for PhysicsCombatAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsCombatAdapter {
    /// 以默认 60Hz 固定步创建空物理世界。
    pub fn new() -> Self {
        Self::with_fps(FixedTimestep::DEFAULT_FPS)
    }

    /// 以指定固定步频率（Hz）创建空物理世界。
    pub fn with_fps(fps: f32) -> Self {
        Self {
            world: PhysicsWorld::new(),
            timestep: FixedTimestep::new(fps),
            bodies: CombatantBodyMap::new(),
            characters: HashMap::new(),
            character_filter: QueryFilter::new(),
            camera_filter: QueryFilter::new(),
            hit_filter: QueryFilter::new(),
        }
    }

    /// 使用已有物理世界创建（固定步默认 60Hz）。
    pub fn from_world(world: PhysicsWorld) -> Self {
        let mut adapter = Self::new();
        adapter.world = world;
        adapter
    }

    /// 物理世界（只读）。
    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    /// 物理世界（可变）：用于创建/销毁刚体、碰撞体、设置重力等。
    pub fn world_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.world
    }

    /// 固定步累加器（只读）。
    pub fn timestep(&self) -> &FixedTimestep {
        &self.timestep
    }

    /// 固定步累加器（可变）：设置最大补跑步数/最大帧时间等。
    pub fn timestep_mut(&mut self) -> &mut FixedTimestep {
        &mut self.timestep
    }

    /// `CombatantId ↔ body_id` 映射表（只读）。
    pub fn bodies(&self) -> &CombatantBodyMap {
        &self.bodies
    }

    /// `CombatantId ↔ body_id` 映射表（可变）。
    pub fn bodies_mut(&mut self) -> &mut CombatantBodyMap {
        &mut self.bodies
    }

    /// 注册 combatant 与 body 的映射（一个 combatant 可注册多个 body）。
    ///
    /// 返回 `true` 表示新增映射。
    pub fn register_combatant(&mut self, combatant: CombatantId, body_id: u64) -> bool {
        self.bodies.register(combatant, body_id)
    }

    /// 解除 combatant 的全部 body 映射，返回被移除的 body 列表。
    pub fn unregister_combatant(&mut self, combatant: CombatantId) -> Vec<u64> {
        self.bodies.unregister_combatant(combatant)
    }

    /// combatant 的第一个 body（多部位用 [`Self::bodies_of_combatant`]）。
    pub fn body_of_combatant(&self, combatant: CombatantId) -> Option<u64> {
        self.bodies.body_of_combatant(combatant)
    }

    /// combatant 注册的全部 body。
    pub fn bodies_of_combatant(&self, combatant: CombatantId) -> &[u64] {
        self.bodies.bodies_of_combatant(combatant)
    }

    /// 物理 body 对应的 combatant（未注册时回退为 `CombatantId::from(body_id)`）。
    pub fn combatant_of_body(&self, body_id: u64) -> CombatantId {
        self.bodies.resolve(body_id)
    }

    // ---- 查询过滤器（层掩码/触发器/排除自身） -------------------------

    /// 设置角色查询过滤器。
    pub fn set_character_filter(&mut self, filter: QueryFilter) {
        self.character_filter = filter;
    }

    /// 角色查询过滤器。
    pub fn character_filter(&self) -> QueryFilter {
        self.character_filter
    }

    /// 设置相机/视线查询过滤器。
    pub fn set_camera_filter(&mut self, filter: QueryFilter) {
        self.camera_filter = filter;
    }

    /// 相机/视线查询过滤器。
    pub fn camera_filter(&self) -> QueryFilter {
        self.camera_filter
    }

    /// 设置命中盒查询过滤器。
    pub fn set_hit_filter(&mut self, filter: QueryFilter) {
        self.hit_filter = filter;
    }

    /// 命中盒查询过滤器。
    pub fn hit_filter(&self) -> QueryFilter {
        self.hit_filter
    }

    /// 角色查询只命中这些层。
    pub fn set_character_layer_mask(&mut self, layer_mask: u32) {
        self.character_filter = self.character_filter.with_layer_mask(layer_mask);
    }

    /// 相机查询只命中这些层。
    pub fn set_camera_layer_mask(&mut self, layer_mask: u32) {
        self.camera_filter = self.camera_filter.with_layer_mask(layer_mask);
    }

    /// 命中盒查询只命中这些层。
    pub fn set_hit_layer_mask(&mut self, layer_mask: u32) {
        self.hit_filter = self.hit_filter.with_layer_mask(layer_mask);
    }

    // ---- 查询入口（复用 gameplay physics 适配层） ---------------------

    /// 角色查询（地面探测 + 胶囊扫掠），使用通用角色过滤器。
    pub fn character_query(&self) -> PhysicsQuery<'_> {
        PhysicsQuery::with_filter(&self.world, self.character_filter)
    }

    /// 角色查询，并自动排除该 combatant 自己的 body（避免自碰撞）。
    pub fn character_query_for(&self, combatant: CombatantId) -> PhysicsQuery<'_> {
        PhysicsQuery::with_filter(
            &self.world,
            self.excluding(self.character_filter, combatant),
        )
    }

    /// 相机/视线查询（球体扫掠 + raycast），使用通用相机过滤器。
    pub fn camera_query(&self) -> PhysicsQuery<'_> {
        PhysicsQuery::with_filter(&self.world, self.camera_filter)
    }

    /// 相机/视线查询，并自动排除该 combatant 自己的 body。
    pub fn camera_query_for(&self, combatant: CombatantId) -> PhysicsQuery<'_> {
        PhysicsQuery::with_filter(&self.world, self.excluding(self.camera_filter, combatant))
    }

    /// 命中盒查询，使用通用命中过滤器（`body_id` 尚未映射）。
    pub fn hit_query(&self) -> PhysicsHitQuery<'_> {
        PhysicsHitQuery::with_filter(&self.world, self.hit_filter)
    }

    /// 命中盒查询，并自动排除该 combatant 自己的 body。
    pub fn hit_query_for(&self, combatant: CombatantId) -> PhysicsHitQuery<'_> {
        PhysicsHitQuery::with_filter(&self.world, self.excluding(self.hit_filter, combatant))
    }

    /// 命中盒查询（带 `CombatantId` 映射与去重），可直接交给 gameplay 的
    /// `HitboxRuntime::detect`。
    pub fn combat_hit_query(&self) -> CombatHitQuery<'_> {
        CombatHitQuery::new(self.hit_query(), &self.bodies)
    }

    /// 命中盒查询（带映射/去重），并排除攻击者自己的 body。
    pub fn combat_hit_query_for(&self, combatant: CombatantId) -> CombatHitQuery<'_> {
        CombatHitQuery::new(self.hit_query_for(combatant), &self.bodies)
    }

    /// 在过滤器基础上排除 combatant 自身 body。
    ///
    /// `QueryFilter` 只支持排除单个 body，因此多部位角色只排除**第一个** body；
    /// 需要排除多个部位时请通过层/掩码或自定义过滤器处理（physics 0.2 限制）。
    fn excluding(&self, filter: QueryFilter, combatant: CombatantId) -> QueryFilter {
        match self.bodies.body_of_combatant(combatant) {
            Some(body_id) => filter.exclude_body(body_id),
            None => filter,
        }
    }

    // ---- 固定步推进 ---------------------------------------------------

    /// 用真实帧时间 `frame_dt` 推进物理世界的固定步。
    ///
    /// 只推进**动态刚体**；角色是 `Kinematic` body，不走这里的积分，
    /// 位置由 [`Self::sync_character_transform`] 从 gameplay motor 同步。
    pub fn advance(&mut self, frame_dt: f32) -> FixedStepResult {
        let steps = self.world.advance_fixed(&mut self.timestep, frame_dt);
        FixedStepResult::new(steps, self.timestep.alpha(), self.timestep.step())
    }

    /// 强制执行一次固定步（调试/确定性模拟用；跳过累加器）。
    pub fn fixed_step(&mut self) {
        let step = self.timestep.step();
        self.world.step(step);
    }

    /// 清空固定步累加器（传送/切场景后调用）。
    pub fn reset_accumulator(&mut self) {
        self.timestep.reset();
    }

    // ---- 角色 body 同步 -----------------------------------------------

    /// 创建角色 body（Kinematic 胶囊）+ 注册映射，返回 body_id。
    ///
    /// `feet` 为 gameplay 约定的脚底位置；physics 的胶囊中心在 `feet + up * height/2`。
    /// 角色 body 使用 `PLAYER` 层、与所有层交互，可按需再用 `world_mut()` 调整层/掩码。
    pub fn spawn_character(
        &mut self,
        combatant: CombatantId,
        feet: Vec3,
        radius: f32,
        height: f32,
    ) -> u64 {
        let radius = radius.max(0.01);
        let height = height.max(radius * 2.0);
        let half_height = capsule_half_height(height, radius);
        let body = self.world.create_body_with_type(RigidBodyType::Kinematic);
        self.write_character_body_position(body, feet, height);
        let collider = self
            .world
            .create_collider(ColliderShape::capsule(half_height, radius), body);
        if let Some(collider) = self.world.get_collider_mut(collider) {
            collider.set_layer(layers::PLAYER);
            collider.set_mask(layers::ALL);
        }
        self.bodies.register(combatant, body);
        self.characters.insert(
            combatant,
            CharacterBody {
                body_id: body,
                radius,
                height,
            },
        );
        body
    }

    /// 角色 body 的尺寸 `(radius, height)`。
    pub fn character_dims(&self, combatant: CombatantId) -> Option<(f32, f32)> {
        self.characters
            .get(&combatant)
            .map(|body| (body.radius, body.height))
    }

    /// 从 physics 读取角色脚底位置（需要先用 [`Self::spawn_character`] 创建）。
    pub fn character_feet(&self, combatant: CombatantId) -> Option<Vec3> {
        let body = self.characters.get(&combatant)?;
        let position = *self.world.get_body(body.body_id)?.position();
        Some(from_core_vec3(position) - Vec3::UP * (body.height * 0.5))
    }

    /// 把 gameplay motor 的脚底位置同步到角色 kinematic body。
    ///
    /// 这是**唯一**允许写角色 body 位置的入口之一（另一个是直接 `world_mut()`）；
    /// 它不做任何积分/扫掠，只同步 transform。返回 `false` 表示该 combatant
    /// 不是由 [`Self::spawn_character`] 创建的角色。
    pub fn sync_character_transform(&mut self, combatant: CombatantId, feet: Vec3) -> bool {
        let Some(body) = self.characters.get(&combatant).copied() else {
            return false;
        };
        self.write_character_body_position(body.body_id, feet, body.height);
        if let Some(body) = self.world.get_body_mut(body.body_id) {
            body.wake_up();
        }
        true
    }

    fn write_character_body_position(&mut self, body_id: u64, feet: Vec3, height: f32) {
        if let Some(body) = self.world.get_body_mut(body_id) {
            body.set_position(to_core_vec3(feet + Vec3::UP * (height * 0.5)));
        }
    }

    // ---- 常用场景搭建辅助 ---------------------------------------------

    /// 创建静态轴对齐盒（默认层、与所有层交互），返回 body_id。
    pub fn spawn_static_box(&mut self, center: Vec3, size: Vec3) -> u64 {
        self.spawn_static_box_with_layers(center, size, layers::DEFAULT, layers::ALL)
    }

    /// 创建指定层/掩码的静态轴对齐盒，返回 body_id。
    ///
    /// physics 0.2 没有 body→collider 的公开访问器，因此层/掩码在创建时由本方法设置。
    pub fn spawn_static_box_with_layers(
        &mut self,
        center: Vec3,
        size: Vec3,
        layer: u32,
        mask: u32,
    ) -> u64 {
        let body = self.world.create_body_with_type(RigidBodyType::Static);
        if let Some(body_ref) = self.world.get_body_mut(body) {
            body_ref.set_position(to_core_vec3(center));
        }
        let collider = self
            .world
            .create_collider(ColliderShape::box_size(to_core_vec3(size)), body);
        apply_collider_layers(&mut self.world, collider, layer, mask);
        body
    }

    /// 创建动态轴对齐盒（参与 `advance` 的固定步积分），返回 body_id。
    pub fn spawn_dynamic_box(&mut self, center: Vec3, size: Vec3) -> u64 {
        self.spawn_dynamic_box_with_layers(center, size, layers::DEFAULT, layers::ALL)
    }

    /// 创建指定层/掩码的动态轴对齐盒，返回 body_id。
    pub fn spawn_dynamic_box_with_layers(
        &mut self,
        center: Vec3,
        size: Vec3,
        layer: u32,
        mask: u32,
    ) -> u64 {
        let body = self.world.create_body_with_type(RigidBodyType::Dynamic);
        if let Some(body_ref) = self.world.get_body_mut(body) {
            body_ref.set_position(to_core_vec3(center));
        }
        let collider = self
            .world
            .create_collider(ColliderShape::box_size(to_core_vec3(size)), body);
        apply_collider_layers(&mut self.world, collider, layer, mask);
        body
    }

    /// 创建传感器球（trigger），返回 body_id；默认 `TRIGGER` 层。
    pub fn spawn_sensor_sphere(&mut self, center: Vec3, radius: f32) -> u64 {
        let body = self.world.create_body_with_type(RigidBodyType::Static);
        if let Some(body_ref) = self.world.get_body_mut(body) {
            body_ref.set_position(to_core_vec3(center));
        }
        let collider = self
            .world
            .create_collider(ColliderShape::sphere(radius), body);
        if let Some(collider) = self.world.get_collider_mut(collider) {
            collider.set_sensor(true);
            collider.set_layer(layers::TRIGGER);
        }
        body
    }
}

/// 设置碰撞体层/掩码（内部辅助）。
fn apply_collider_layers(world: &mut PhysicsWorld, collider_id: u64, layer: u32, mask: u32) {
    if let Some(collider) = world.get_collider_mut(collider_id) {
        collider.set_layer(layer);
        collider.set_mask(mask);
    }
}

/// 带 `CombatantId` 映射与去重的命中盒查询。
///
/// 包装 gameplay 的 `PhysicsHitQuery`（它按 body 去重并返回 `CombatantId::from(body_id)`），
/// 再按 [`CombatantBodyMap`] 把 body 重映射为业务 `CombatantId`，并保证同一目标只出现一次。
/// 实现 gameplay 的 [`HitDetectionQuery`]，可直接用于 `HitboxRuntime::detect`。
pub struct CombatHitQuery<'a> {
    physics: PhysicsHitQuery<'a>,
    bodies: &'a CombatantBodyMap,
}

impl<'a> CombatHitQuery<'a> {
    /// 由 physics 查询 + 映射表构造。
    pub fn new(physics: PhysicsHitQuery<'a>, bodies: &'a CombatantBodyMap) -> Self {
        Self { physics, bodies }
    }

    /// 底层 physics 命中查询（未映射的 `body_id` 结果）。
    pub fn physics(&self) -> PhysicsHitQuery<'a> {
        self.physics
    }
}

impl HitDetectionQuery for CombatHitQuery<'_> {
    fn query_hitbox(&self, hitbox: &Hitbox, pose: &HitboxPose) -> Vec<HitContact> {
        let mut contacts = self.physics.query_hitbox(hitbox, pose);
        // physics 适配层返回的 target 是 CombatantId::from(body_id)；这里换成业务映射。
        for contact in contacts.iter_mut() {
            contact.target = self.bodies.resolve(contact.target.raw());
        }
        // 同一个 combatant 可能挂多个 body（多部位）：按映射后的目标去重。
        let mut seen: Vec<CombatantId> = Vec::with_capacity(contacts.len());
        contacts.retain(|contact| {
            if seen.contains(&contact.target) {
                false
            } else {
                seen.push(contact.target);
                true
            }
        });
        contacts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combatant_body_map_supports_multiple_bodies() {
        let mut map = CombatantBodyMap::new();
        assert!(map.is_empty());
        let first = CombatantId::new(1);
        assert!(map.register(first, 10));
        assert!(!map.register(first, 10), "重复注册同一 body 返回 false");
        assert!(map.register(first, 11), "同一 combatant 可注册多个 body");
        assert_eq!(map.combatant_of_body(10), Some(first));
        assert_eq!(map.combatant_of_body(11), Some(first));
        assert_eq!(map.body_of_combatant(first), Some(10));
        assert_eq!(map.bodies_of_combatant(first), &[10, 11]);
        assert_eq!(map.len(), 2);
        assert_eq!(map.combatant_count(), 1);

        // body 换 combatant：从旧 combatant 的列表移除。
        let second = CombatantId::new(2);
        assert!(map.register(second, 11));
        assert_eq!(map.bodies_of_combatant(first), &[10]);
        assert_eq!(map.bodies_of_combatant(second), &[11]);

        // 未注册 body 回退为同值 CombatantId。
        assert_eq!(map.resolve(99), CombatantId::from(99));

        // 解除单个 body / 整个 combatant。
        assert_eq!(map.unregister_body(10), Some(first));
        assert!(map.bodies_of_combatant(first).is_empty());
        assert_eq!(map.unregister_combatant(second), vec![11]);
        assert!(map.is_empty());
    }

    #[test]
    fn adapter_advance_reports_fixed_steps_and_alpha() {
        let mut adapter = PhysicsCombatAdapter::with_fps(60.0);
        let result = adapter.advance(1.0 / 60.0);
        assert_eq!(result.steps, 1);
        assert!((result.simulated_time - 1.0 / 60.0).abs() < 1e-6);
        assert!((0.0..=1.0).contains(&result.alpha));

        // 不足一步的帧不推进。
        let result = adapter.advance(1.0 / 240.0);
        assert!(result.is_empty());
    }

    #[test]
    fn spawned_character_position_roundtrip() {
        let mut adapter = PhysicsCombatAdapter::new();
        let player = CombatantId::new(7);
        adapter.spawn_character(player, Vec3::new(0.0, 0.0, 0.0), 0.35, 1.8);
        let feet = adapter.character_feet(player).expect("角色应有 body");
        assert!((feet.y).abs() < 1e-5, "脚底应在 0，实际 {feet:?}");

        assert!(adapter.sync_character_transform(player, Vec3::new(1.0, 2.0, 3.0)));
        let feet = adapter.character_feet(player).unwrap();
        assert!((feet - Vec3::new(1.0, 2.0, 3.0)).magnitude() < 1e-5);
        assert_eq!(adapter.character_dims(player), Some((0.35, 1.8)));
        assert!(!adapter.sync_character_transform(CombatantId::new(404), Vec3::ZERO));
    }
}
