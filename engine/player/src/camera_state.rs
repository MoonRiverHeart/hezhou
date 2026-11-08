//! Player 第三人称跟随相机状态（task-59）：C# 每帧通过 ABI v7
//! `camera_follow(entity, distance, height, yaw, pitch)` 设置目标；窗口 host 读取目标
//! 实体脚底位置，计算相机位姿并应用 `set_camera_params`。
//!
//! 相机约定（与 rhi-vulkan `rotation.vert` 一致）：
//! - `yaw` 弧度，0 时视线朝 -Z；forward = (sin yaw, 0, -cos yaw)；
//! - `pitch` 弧度（负值俯视，第三人称默认约 -15°）；
//! - 相机位置 = 角色脚底 + `height` 的观察点 - forward * `distance`（即角色后上方）；
//! - 目标实体不存在时返回 None，host 保持上一帧相机并 WARN（避免瞬移/丢相机）。

use hezhou_core::ecs::Scene;
use hezhou_core::math::Vec3;

/// 默认相机距离（米）。
pub const DEFAULT_CAMERA_DISTANCE: f32 = 5.0;
/// 默认相机观察高度（米）。
pub const DEFAULT_CAMERA_HEIGHT: f32 = 2.5;
/// 默认俯仰（弧度，约 -15°）。
pub const DEFAULT_CAMERA_PITCH: f32 = -0.261_799_39;
/// 相机距离允许范围。
pub const MAX_CAMERA_DISTANCE: f32 = 50.0;

/// 相机状态错误。
#[derive(Clone, Debug, PartialEq)]
pub enum CameraStateError {
    /// 距离/高度/角度非法（NaN/∞/越界）。
    InvalidParameter { name: &'static str, value: f32 },
    /// 目标实体 id 为 0。
    InvalidTarget { entity: u64 },
}

impl std::fmt::Display for CameraStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParameter { name, value } => write!(f, "非法相机参数 {name}={value}"),
            Self::InvalidTarget { entity } => write!(f, "非法相机目标实体 {entity}"),
        }
    }
}

impl std::error::Error for CameraStateError {}

/// 相机位姿（传给 `UIVulkanRenderer::set_camera_params`）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    /// 偏航角（弧度）。
    pub yaw: f32,
    /// 俯仰角（弧度）。
    pub pitch: f32,
    /// 相机 X。
    pub x: f32,
    /// 相机 Y。
    pub y: f32,
    /// 相机 Z。
    pub z: f32,
}

/// 第三人称跟随相机状态。
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerCameraState {
    target: Option<u64>,
    distance: f32,
    height: f32,
    yaw: f32,
    pitch: f32,
}

impl Default for PlayerCameraState {
    fn default() -> Self {
        Self {
            target: None,
            distance: DEFAULT_CAMERA_DISTANCE,
            height: DEFAULT_CAMERA_HEIGHT,
            yaw: 0.0,
            pitch: DEFAULT_CAMERA_PITCH,
        }
    }
}

impl PlayerCameraState {
    /// 无目标相机状态（host 使用默认固定相机）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置跟随目标与相机参数；非法值返回错误（不修改状态）。
    pub fn follow(
        &mut self,
        entity_id: u64,
        distance: f32,
        height: f32,
        yaw: f32,
        pitch: f32,
    ) -> Result<(), CameraStateError> {
        if entity_id == 0 {
            return Err(CameraStateError::InvalidTarget { entity: entity_id });
        }
        if !distance.is_finite() || distance <= 0.0 || distance > MAX_CAMERA_DISTANCE {
            return Err(CameraStateError::InvalidParameter {
                name: "distance",
                value: distance,
            });
        }
        if !height.is_finite() || height.abs() > 100.0 {
            return Err(CameraStateError::InvalidParameter {
                name: "height",
                value: height,
            });
        }
        if !yaw.is_finite() {
            return Err(CameraStateError::InvalidParameter {
                name: "yaw",
                value: yaw,
            });
        }
        if !pitch.is_finite() || pitch.abs() > std::f32::consts::FRAC_PI_2 {
            return Err(CameraStateError::InvalidParameter {
                name: "pitch",
                value: pitch,
            });
        }
        self.target = Some(entity_id);
        self.distance = distance;
        self.height = height;
        self.yaw = yaw;
        self.pitch = pitch;
        Ok(())
    }

    /// 清除跟随目标（host 回退默认相机）。
    pub fn clear(&mut self) {
        self.target = None;
    }

    /// 跟随目标实体 id。
    pub fn target(&self) -> Option<u64> {
        self.target
    }

    /// 当前距离。
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// 当前观察高度。
    pub fn height(&self) -> f32 {
        self.height
    }

    /// 当前 yaw。
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// 当前 pitch。
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// 计算目标在场景中的相机位姿；目标不存在/无目标 → None。
    pub fn pose_for(&self, scene: &Scene) -> Option<CameraPose> {
        let target = self.target?;
        let entity = scene
            .get_all_entities()
            .into_iter()
            .find(|entity| entity.id == target)?;
        let feet: Vec3 = scene.get_entity_position(entity)?;
        let forward = Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos());
        let pivot = Vec3::new(feet.x, feet.y + self.height, feet.z);
        let camera = Vec3::new(
            pivot.x - forward.x * self.distance,
            pivot.y,
            pivot.z - forward.z * self.distance,
        );
        Some(CameraPose {
            yaw: self.yaw,
            pitch: self.pitch,
            x: camera.x,
            y: camera.y,
            z: camera.z,
        })
    }

    /// 单行摘要。
    pub fn summary(&self) -> String {
        match self.target {
            Some(target) => format!(
                "相机跟随 entity={target} distance={:.1} height={:.1} yaw={:.2} pitch={:.2}",
                self.distance, self.height, self.yaw, self.pitch
            ),
            None => "相机：无跟随目标（默认相机）".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_with_entity(position: Vec3) -> (Scene, u64) {
        let mut scene = Scene::new();
        let cube = scene.create_cube();
        scene.set_entity_position(cube, position);
        (scene, cube.id)
    }

    #[test]
    fn follow_computes_behind_and_above_pose() {
        let (scene, entity) = scene_with_entity(Vec3::new(0.0, 0.0, 0.0));
        let mut camera = PlayerCameraState::new();
        camera
            .follow(entity, 5.0, 2.5, 0.0, DEFAULT_CAMERA_PITCH)
            .expect("follow");
        assert_eq!(camera.target(), Some(entity));
        let pose = camera.pose_for(&scene).expect("pose");
        // yaw=0：forward=-Z，相机在 +Z 后方 5m；高度 = 观察高度。
        assert!((pose.x - 0.0).abs() < 1e-5);
        assert!((pose.z - 5.0).abs() < 1e-5);
        assert!((pose.y - 2.5).abs() < 1e-5);
        assert!((pose.yaw - 0.0).abs() < 1e-6);
        assert!((pose.pitch - DEFAULT_CAMERA_PITCH).abs() < 1e-6);
    }

    #[test]
    fn yaw_rotates_camera_offset() {
        let (scene, entity) = scene_with_entity(Vec3::new(1.0, 0.0, 2.0));
        let mut camera = PlayerCameraState::new();
        // yaw=90°：forward=+X，相机在 -X 后方。
        camera
            .follow(entity, 5.0, 2.5, std::f32::consts::FRAC_PI_2, -0.1)
            .expect("follow");
        let pose = camera.pose_for(&scene).unwrap();
        assert!((pose.x - (1.0 - 5.0)).abs() < 1e-5, "x={}", pose.x);
        assert!((pose.z - 2.0).abs() < 1e-5);
    }

    #[test]
    fn missing_target_or_invalid_params_are_safe() {
        let (scene, entity) = scene_with_entity(Vec3::new(0.0, 0.0, 0.0));
        let mut camera = PlayerCameraState::new();
        assert!(camera.follow(entity, f32::NAN, 2.5, 0.0, -0.2).is_err());
        assert!(camera.follow(entity, 5.0, 2.5, 0.0, f32::NAN).is_err());
        assert!(camera.follow(0, 5.0, 2.5, 0.0, -0.2).is_err());
        camera.follow(entity, 5.0, 2.5, 0.0, -0.2).unwrap();
        camera.clear();
        assert!(camera.target().is_none());
        assert!(camera.pose_for(&scene).is_none());
        // 目标实体不存在。
        camera.follow(0xDEAD_BEEF, 5.0, 2.5, 0.0, -0.2).unwrap();
        assert!(camera.pose_for(&scene).is_none());
        assert!(camera.summary().contains("entity"));
    }
}
