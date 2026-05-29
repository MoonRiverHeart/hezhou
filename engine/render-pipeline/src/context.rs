//! 渲染上下文类型定义
//!
//! RenderContext — 每帧渲染参数（场景指针、摄像机、视口、选中实体）
//! RenderMode — 编辑/运行模式映射（对应 GameState）

use hezhou_core::ecs::Scene;

/// 渲染模式，映射 GameState
///
/// Editing模式渲染网格+选中高亮+实体，
/// Running模式仅渲染实体（无编辑辅助元素）。
/// 对应关系: GameState::Editing → RenderMode::Editing,
///           GameState::Running/Paused → RenderMode::Running
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// 编辑模式 — 网格 + 选中高亮 + 实体 (对应 GameState::Editing)
    Editing,
    /// 运行模式 — 仅实体 (对应 GameState::Running/Paused)
    Running,
}

impl RenderMode {
    /// 从 GameState 转换
    ///
    /// Editing → Editing, Running/Paused → Running
    pub fn from_game_state(state: hezhou_core::ecs::GameState) -> Self {
        match state {
            hezhou_core::ecs::GameState::Editing => RenderMode::Editing,
            hezhou_core::ecs::GameState::Running => RenderMode::Running,
            hezhou_core::ecs::GameState::Paused => RenderMode::Running, // Paused时仍用Running渲染
        }
    }
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::Editing
    }
}

/// 摄像机参数
///
/// 包含摄像机位置、朝向角度和投影参数，
/// 用于构建view/projection矩阵。
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraParams {
    /// 摄像机世界坐标位置 [x, y, z]
    pub position: [f32; 3],
    /// 摄像机yaw角度（弧度，绕Y轴旋转）
    pub yaw: f32,
    /// 摄像机pitch角度（弧度，绕X轴旋转）
    pub pitch: f32,
    /// 视场角（弧度）
    pub fov: f32,
    /// 近裁剪面距离
    pub near: f32,
    /// 远裁剪面距离
    pub far: f32,
}

/// 视口范围
///
/// 描述渲染目标的像素尺寸。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewportExtent {
    /// 宽度（像素）
    pub width: u32,
    /// 高度（像素）
    pub height: u32,
}

impl ViewportExtent {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// 渲染上下文 — 每帧传递给管线的数据
///
/// 包含场景指针、摄像机参数、视口大小、选中实体列表等。
/// Scene用raw pointer因为renderer已持有Scene所有权，管线只读取。
///
/// **注意**: 不实现Send/Sync（因为包含*mut Scene），
/// 管线trait要求Send+Sync，但RenderContext是每帧临时传递的引用参数，
/// 不需要跨线程存储。
pub struct RenderContext {
    /// 场景指针（renderer持有Scene所有权，管线只读取）
    pub scene: *mut Scene,
    /// 当前渲染模式
    pub mode: RenderMode,
    /// 摄像机参数
    pub camera: CameraParams,
    /// 视口范围
    pub viewport_extent: ViewportExtent,
    /// 当前选中的实体ID列表（Editing模式下使用）
    pub selected_entity_ids: Vec<hezhou_core::ecs::EntityId>,
    /// 当前帧索引（用于动态资源轮转）
    pub frame_index: u32,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            scene: std::ptr::null_mut(),
            mode: RenderMode::Editing,
            camera: CameraParams::default(),
            viewport_extent: ViewportExtent::new(1280, 720),
            selected_entity_ids: Vec::new(),
            frame_index: 0,
        }
    }
}