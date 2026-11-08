//! Player 角色控制运行时（task-36）：`hezhou-physics` 地面 Plane + 每实体 `CharacterMotor`。
//!
//! - 世界启动时创建一个静态 `Plane` 碰撞体（y=0，法线 +Y）作为地面；
//! - `character_create(entity, radius, total_height)` 从 Scene transform（脚底位置）
//!   创建胶囊角色并注册到 entity id；
//! - `character_step(entity, input_x, input_z, jump, dt)` 调用 `CharacterMotor::step`
//!   （水平输入为期望速度 m/s，y 由重力/跳跃接管），把结果脚底位置写回 Scene；
//! - `grounded/position/remove` 查询/移除角色。
//!
//! 所有方法对非法实体/参数返回可读错误（C ABI 层再包 catch_unwind）；场景指针必须
//! 在 runtime 生命周期内有效（host 帧循环内）。
//!
//! 限制（0.2）：单无限地面 Plane、胶囊直立、无坡/台阶之外的复杂地形、
//! 输入为直接速度（无加速度/相机方向/动画混合）。

use std::collections::BTreeMap;

use hezhou_core::ecs::{Entity, Scene};
use hezhou_core::math::Vec3;
use hezhou_physics::{CharacterMotor, ColliderShape, PhysicsWorld, RigidBodyType};

/// 默认角色半径（米）。
pub const DEFAULT_CHARACTER_RADIUS: f32 = 0.3;
/// 默认角色总高（米）。
pub const DEFAULT_CHARACTER_HEIGHT: f32 = 1.8;

/// 角色控制运行时错误。
#[derive(Clone, Debug, PartialEq)]
pub enum GameplayRuntimeError {
    /// Scene 指针为空。
    NoScene,
    /// 实体不存在。
    EntityNotFound { entity: u64 },
    /// 尚未创建角色。
    CharacterNotFound { entity: u64 },
    /// 非法半径/高度/dt/输入。
    InvalidParameter { name: &'static str, value: f32 },
    /// 场景/物理操作 panic（被捕获）。
    Panicked { operation: &'static str },
}

impl std::fmt::Display for GameplayRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoScene => write!(f, "Scene 指针为空"),
            Self::EntityNotFound { entity } => write!(f, "Scene 实体 {entity} 不存在"),
            Self::CharacterNotFound { entity } => write!(f, "实体 {entity} 未创建角色"),
            Self::InvalidParameter { name, value } => {
                write!(f, "非法参数 {name}={value}")
            }
            Self::Panicked { operation } => {
                write!(f, "角色控制 {operation} 发生 panic（已捕获）")
            }
        }
    }
}

impl std::error::Error for GameplayRuntimeError {}

/// 单个角色的运行时状态（诊断/日志）。
#[derive(Clone, Debug)]
pub struct PlayerCharacterState {
    /// 脚底位置。
    pub position: Vec3,
    /// 是否着地。
    pub grounded: bool,
    /// 半径。
    pub radius: f32,
    /// 总高。
    pub total_height: f32,
}

/// Player 角色控制运行时（窗口 host 持有）。
pub struct PlayerGameplayRuntime {
    world: PhysicsWorld,
    scene: *mut Scene,
    characters: BTreeMap<u64, CharacterMotor>,
    ground_body: u64,
}

impl PlayerGameplayRuntime {
    /// 创建运行时并为场景建立 y=0 地面 Plane。
    pub fn new(scene: *mut Scene) -> Self {
        let mut world = PhysicsWorld::new();
        let ground_body = world.create_body_with_type(RigidBodyType::Static);
        world.create_collider(ColliderShape::Plane, ground_body);
        Self {
            world,
            scene,
            characters: BTreeMap::new(),
            ground_body,
        }
    }

    /// 地面刚体 id（诊断）。
    pub fn ground_body(&self) -> u64 {
        self.ground_body
    }

    /// 已注册角色数量。
    pub fn character_count(&self) -> usize {
        self.characters.len()
    }

    /// 是否已为 entity 创建角色。
    pub fn has_character(&self, entity_id: u64) -> bool {
        self.characters.contains_key(&entity_id)
    }

    /// 查询角色状态（诊断/日志）。
    pub fn character_state(&self, entity_id: u64) -> Option<PlayerCharacterState> {
        self.characters.get(&entity_id).map(|motor| PlayerCharacterState {
            position: motor.position,
            grounded: motor.is_grounded(),
            radius: motor.radius,
            total_height: motor.total_height(),
        })
    }

    /// 为 entity 创建胶囊角色；脚底位置来自 Scene transform。
    pub fn create(
        &mut self,
        entity_id: u64,
        radius: f32,
        total_height: f32,
    ) -> Result<(), GameplayRuntimeError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(GameplayRuntimeError::InvalidParameter {
                name: "radius",
                value: radius,
            });
        }
        if !total_height.is_finite() || total_height <= 0.0 {
            return Err(GameplayRuntimeError::InvalidParameter {
                name: "total_height",
                value: total_height,
            });
        }
        let scene = self.scene_mut().ok_or(GameplayRuntimeError::NoScene)?;
        let entity = find_entity(scene, entity_id)
            .ok_or(GameplayRuntimeError::EntityNotFound { entity: entity_id })?;
        let feet = scene
            .get_entity_position(entity)
            .ok_or(GameplayRuntimeError::EntityNotFound { entity: entity_id })?;
        let motor = CharacterMotor::new(feet.x, feet.y, feet.z)
            .with_radius(radius)
            .with_total_height(total_height);
        self.characters.insert(entity_id, motor);
        Ok(())
    }

    /// 步进角色：input_x/input_z 为水平期望速度（m/s），jump 仅着地时生效；
    /// 结果脚底位置写回 Scene transform。
    pub fn step(
        &mut self,
        entity_id: u64,
        input_x: f32,
        input_z: f32,
        jump: bool,
        dt: f32,
    ) -> Result<PlayerCharacterState, GameplayRuntimeError> {
        if !input_x.is_finite() || !input_z.is_finite() {
            return Err(GameplayRuntimeError::InvalidParameter {
                name: "input",
                value: input_x,
            });
        }
        if !dt.is_finite() || dt <= 0.0 {
            return Err(GameplayRuntimeError::InvalidParameter {
                name: "dt",
                value: dt,
            });
        }
        let Self {
            world,
            scene,
            characters,
            ..
        } = self;
        let motor = characters.get_mut(&entity_id).ok_or(
            GameplayRuntimeError::CharacterNotFound { entity: entity_id },
        )?;
        let result = motor.step(world, dt, Vec3::new(input_x, 0.0, input_z), jump);
        let feet = result.position;
        let scene = unsafe { scene.as_mut() }.ok_or(GameplayRuntimeError::NoScene)?;
        if let Some(entity) = find_entity(scene, entity_id) {
            scene.set_entity_position(entity, feet);
        }
        Ok(PlayerCharacterState {
            position: motor.position,
            grounded: motor.is_grounded(),
            radius: motor.radius,
            total_height: motor.total_height(),
        })
    }

    /// 是否着地（未创建角色 → 0/false）。
    pub fn grounded(&self, entity_id: u64) -> bool {
        self.characters
            .get(&entity_id)
            .map(|motor| motor.is_grounded())
            .unwrap_or(false)
    }

    /// 角色脚底位置（未创建 → None）。
    pub fn position(&self, entity_id: u64) -> Option<Vec3> {
        self.characters
            .get(&entity_id)
            .map(|motor| motor.position)
    }

    /// 移除角色（不存在返回 false）。
    pub fn remove(&mut self, entity_id: u64) -> bool {
        self.characters.remove(&entity_id).is_some()
    }

    /// 每帧推进（当前无独立物理世界步进；CharacterMotor 在 step 中查询世界）。
    pub fn update(&mut self, _dt: f32) {
        // 0.2 角色物理为查询驱动；保留给后续刚体/触发器等世界步进。
    }

    /// 单行摘要（启动/结束日志）。
    pub fn summary(&self) -> String {
        format!(
            "角色运行时：{} 个角色，地面 body={}",
            self.characters.len(),
            self.ground_body
        )
    }

    fn scene_mut(&mut self) -> Option<&mut Scene> {
        if self.scene.is_null() {
            None
        } else {
            Some(unsafe { &mut *self.scene })
        }
    }
}

impl std::fmt::Debug for PlayerGameplayRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerGameplayRuntime")
            .field("characters", &self.characters.len())
            .field("ground_body", &self.ground_body)
            .finish()
    }
}

fn find_entity(scene: &Scene, entity_id: u64) -> Option<Entity> {
    scene
        .get_all_entities()
        .into_iter()
        .find(|entity| entity.id == entity_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_with_cube(position: Vec3) -> (Scene, u64) {
        let mut scene = Scene::new();
        let cube = scene.create_cube();
        scene.set_entity_position(cube, position);
        (scene, cube.id)
    }

    #[test]
    fn character_falls_to_ground_and_grounded() {
        let (mut scene, entity) = scene_with_cube(Vec3::new(0.0, 0.5, 0.0));
        let mut runtime = PlayerGameplayRuntime::new(&mut scene as *mut Scene);
        runtime
            .create(entity, DEFAULT_CHARACTER_RADIUS, DEFAULT_CHARACTER_HEIGHT)
            .expect("create");
        assert_eq!(runtime.character_count(), 1);
        assert!(runtime.has_character(entity));
        // 下落 0.5m 到地面，需要若干帧。
        let mut state = None;
        for _ in 0..120 {
            state = Some(runtime.step(entity, 0.0, 0.0, false, 1.0 / 60.0).expect("step"));
        }
        let state = state.unwrap();
        assert!(state.position.y.abs() < 0.05, "y={}", state.position.y);
        assert!(state.grounded, "应落地 grounded");
        assert!(runtime.grounded(entity));
        let scene_entity = find_entity(&scene, entity).unwrap();
        let scene_pos = scene.get_entity_position(scene_entity).unwrap();
        assert!((scene_pos.y - state.position.y).abs() < 1e-6);
    }

    #[test]
    fn character_moves_forward() {
        let (mut scene, entity) = scene_with_cube(Vec3::new(0.0, 0.0, 0.0));
        let mut runtime = PlayerGameplayRuntime::new(&mut scene as *mut Scene);
        runtime
            .create(entity, 0.3, 1.8)
            .expect("create");
        // 落地稳定后以 +X 1m/s 前进 1s。
        for _ in 0..30 {
            runtime.step(entity, 0.0, 0.0, false, 1.0 / 60.0).unwrap();
        }
        let before = runtime.position(entity).unwrap();
        for _ in 0..60 {
            runtime.step(entity, 1.0, 0.0, false, 1.0 / 60.0).unwrap();
        }
        let after = runtime.position(entity).unwrap();
        assert!(after.x > before.x + 0.5, "位移 {} -> {}", before.x, after.x);
        assert!(after.y.abs() < 0.05, "贴地 y={}", after.y);
    }

    #[test]
    fn jump_changes_grounded_state() {
        let (mut scene, entity) = scene_with_cube(Vec3::new(0.0, 0.0, 0.0));
        let mut runtime = PlayerGameplayRuntime::new(&mut scene as *mut Scene);
        runtime.create(entity, 0.3, 1.8).expect("create");
        for _ in 0..30 {
            runtime.step(entity, 0.0, 0.0, false, 1.0 / 60.0).unwrap();
        }
        assert!(runtime.grounded(entity));
        let jump = runtime.step(entity, 0.0, 0.0, true, 1.0 / 60.0).unwrap();
        assert!(!jump.grounded, "跳跃后应离地");
        assert!(jump.position.y > 0.0);
    }

    #[test]
    fn invalid_entities_and_parameters_are_safe() {
        let (mut scene, entity) = scene_with_cube(Vec3::new(0.0, 0.0, 0.0));
        let mut runtime = PlayerGameplayRuntime::new(&mut scene as *mut Scene);
        assert!(runtime.create(0xDEAD_BEEF, 0.3, 1.8).is_err());
        assert!(runtime.create(entity, 0.0, 1.8).is_err());
        assert!(runtime.create(entity, 0.3, -1.0).is_err());
        assert!(runtime.step(entity, 0.0, 0.0, false, 0.016).is_err());
        assert!(runtime.position(entity).is_none());
        assert!(!runtime.grounded(entity));
        assert!(!runtime.remove(entity));
        runtime.create(entity, 0.3, 1.8).unwrap();
        assert!(runtime.remove(entity));
        assert!(!runtime.has_character(entity));
    }

    #[test]
    fn null_scene_is_reported() {
        let mut runtime = PlayerGameplayRuntime::new(std::ptr::null_mut());
        assert!(matches!(
            runtime.create(1, 0.3, 1.8),
            Err(GameplayRuntimeError::NoScene)
        ));
    }
}
