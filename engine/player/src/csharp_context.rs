//! Player C# Scene Context ABI（task-32 v1 / task-33 v2）：把 `hezhou_core::Scene` 的
//! 最小读写能力以 `#[repr(C)]` 函数指针表暴露给 C# `Hezhou.PlayerGame`。
//!
//! # ABI 契约（C# `[StructLayout(LayoutKind.Sequential)]` 必须一致）
//!
//! v1 基础字段（顺序/偏移不变）：
//!
//! | offset | 字段 | C# 类型 |
//! |---|---|---|
//! | 0  | version: u32 | uint（当前 [`PLAYER_CSHARP_CONTEXT_VERSION`] = 2） |
//! | 4  | reserved: u32 | uint（始终 0） |
//! | 8  | scene: *mut Scene | IntPtr |
//! | 16 | scene_entity_count | IntPtr → delegate `uint(IntPtr)` |
//! | 24 | scene_create_cube | IntPtr → delegate `ulong(IntPtr)` |
//! | 32 | scene_create_entity | IntPtr → delegate `ulong(IntPtr)` |
//! | 40 | scene_set_entity_position | IntPtr → delegate `int(IntPtr, ulong, float, float, float)` |
//! | 48 | scene_get_entity_position | IntPtr → delegate `int(IntPtr, ulong, out float, out float, out float)` |
//! | 56 | log | IntPtr → delegate `void(IntPtr)`（可选；C 字符串） |
//!
//! v2 tail 追加（task-33；保持 v1 字段顺序，旧 v1 读取方忽略尾部）：
//!
//! | offset | 字段 | C# 类型 |
//! |---|---|---|
//! | 64 | scene_set_entity_rotation | `int(IntPtr, ulong, float x, float y, float z)`（欧拉角，度） |
//! | 72 | scene_get_entity_rotation | `int(IntPtr, ulong, out float, out float, out float)`（欧拉角，度） |
//! | 80 | scene_set_entity_scale | `int(IntPtr, ulong, float, float, float)` |
//! | 88 | scene_get_entity_scale | `int(IntPtr, ulong, out float, out float, out float)` |
//! | 96 | scene_entity_exists | `int(IntPtr, ulong)`（1 存在 / 0 不存在） |
//! | 104 | scene_destroy_entity | `int(IntPtr, ulong)`（0 成功 / -1 失败） |
//!
//! 所有函数均为 `extern "C"`、内部 `catch_unwind`；`scene == null`、实体不存在、
//! 输出指针为空时返回 0/-1 且绝不 panic（panic 跨 FFI 是 UB）。rotation 使用
//! **欧拉角（度）**，与 core `Quaternion::from_euler_degrees`/`to_euler_degrees` 对应。
//!
//! # v1 → v2 兼容策略
//!
//! v2 只在尾部追加字段、不改 v1 布局；C# 侧读取 `Version`：
//! - `>= 2`：解析全部字段（rotation/scale/exists/destroy 可用）；
//! - `== 1`：只解析 v1 字段，rotation/scale/destroy 不调用、只保留 position（兼容旧
//!   Player/DLL 组合）；
//! - 其他：记录日志后回退空转。
//!
//! # 生命周期
//!
//! `PlayerCSharpContext` 由 `WindowPlayerHost` 在栈上创建，覆盖整个帧循环；
//! C# 不得在 `Shutdown()` 之后继续持有/调用 `scene` 指针。context 本身按值传给
//! C# `Initialize(IntPtr context)`，C# 必须复制结构体内容，不能保存指向 context
//! 栈内存的指针。

use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use hezhou_core::ecs::{Entity, Scene};
use hezhou_core::math::{Quaternion, Vec3};

/// 当前 Player C# Context ABI 版本。
pub const PLAYER_CSHARP_CONTEXT_VERSION: u32 = 2;

/// C# 侧函数指针：`scene_entity_count(scene) -> u32`。
pub type SceneEntityCountFn = unsafe extern "C" fn(*mut Scene) -> u32;
/// C# 侧函数指针：`scene_create_cube(scene) -> entity_id`。
pub type SceneCreateCubeFn = unsafe extern "C" fn(*mut Scene) -> u64;
/// C# 侧函数指针：`scene_create_entity(scene) -> entity_id`。
pub type SceneCreateEntityFn = unsafe extern "C" fn(*mut Scene) -> u64;
/// C# 侧函数指针：`scene_set_entity_position(scene, entity, x, y, z) -> 0/-1`。
pub type SceneSetEntityPositionFn =
    unsafe extern "C" fn(*mut Scene, u64, f32, f32, f32) -> i32;
/// C# 侧函数指针：`scene_get_entity_position(scene, entity, out x, out y, out z) -> 0/-1`。
pub type SceneGetEntityPositionFn =
    unsafe extern "C" fn(*mut Scene, u64, *mut f32, *mut f32, *mut f32) -> i32;
/// C# 侧函数指针：`log(ptr)`（C 字符串，可选）。
pub type PlayerContextLogFn = unsafe extern "C" fn(*const c_char);
/// C# 侧函数指针：`scene_set_entity_rotation(scene, entity, x, y, z) -> 0/-1`
/// （欧拉角，度）。
pub type SceneSetEntityRotationFn =
    unsafe extern "C" fn(*mut Scene, u64, f32, f32, f32) -> i32;
/// C# 侧函数指针：`scene_get_entity_rotation(scene, entity, out x, out y, out z) -> 0/-1`
/// （欧拉角，度）。
pub type SceneGetEntityRotationFn =
    unsafe extern "C" fn(*mut Scene, u64, *mut f32, *mut f32, *mut f32) -> i32;
/// C# 侧函数指针：`scene_set_entity_scale(scene, entity, x, y, z) -> 0/-1`。
pub type SceneSetEntityScaleFn = unsafe extern "C" fn(*mut Scene, u64, f32, f32, f32) -> i32;
/// C# 侧函数指针：`scene_get_entity_scale(scene, entity, out x, out y, out z) -> 0/-1`。
pub type SceneGetEntityScaleFn =
    unsafe extern "C" fn(*mut Scene, u64, *mut f32, *mut f32, *mut f32) -> i32;
/// C# 侧函数指针：`scene_entity_exists(scene, entity) -> 1/0`。
pub type SceneEntityExistsFn = unsafe extern "C" fn(*mut Scene, u64) -> i32;
/// C# 侧函数指针：`scene_destroy_entity(scene, entity) -> 0/-1`。
pub type SceneDestroyEntityFn = unsafe extern "C" fn(*mut Scene, u64) -> i32;

/// `#[repr(C)]` Player C# Context：版本 + Scene 指针 + 6 个函数指针。
#[repr(C)]
pub struct PlayerCSharpContext {
    /// ABI 版本（[`PLAYER_CSHARP_CONTEXT_VERSION`]）。
    pub version: u32,
    /// 保留（0），保证 scene 指针 8 字节对齐。
    pub reserved: u32,
    /// `hezhou_core::ecs::Scene` 裸指针（host 栈上场景，生命周期覆盖帧循环）。
    pub scene: *mut Scene,
    /// 实体数量。
    pub scene_entity_count: Option<SceneEntityCountFn>,
    /// 创建内置 cube，返回实体 id（失败 0）。
    pub scene_create_cube: Option<SceneCreateCubeFn>,
    /// 创建空实体，返回实体 id（失败 0）。
    pub scene_create_entity: Option<SceneCreateEntityFn>,
    /// 设置实体位置，返回 0 成功 / -1 失败。
    pub scene_set_entity_position: Option<SceneSetEntityPositionFn>,
    /// 读取实体位置，返回 0 成功 / -1 失败。
    pub scene_get_entity_position: Option<SceneGetEntityPositionFn>,
    /// 可选日志函数（C 字符串）。
    pub log: Option<PlayerContextLogFn>,
    // ==== v2 tail 追加（task-33）：保持 v1 布局不变 ====
    /// 设置实体 rotation（欧拉角，度），返回 0 成功 / -1 失败。
    pub scene_set_entity_rotation: Option<SceneSetEntityRotationFn>,
    /// 读取实体 rotation（欧拉角，度），返回 0 成功 / -1 失败。
    pub scene_get_entity_rotation: Option<SceneGetEntityRotationFn>,
    /// 设置实体 scale，返回 0 成功 / -1 失败。
    pub scene_set_entity_scale: Option<SceneSetEntityScaleFn>,
    /// 读取实体 scale，返回 0 成功 / -1 失败。
    pub scene_get_entity_scale: Option<SceneGetEntityScaleFn>,
    /// 实体是否存在：1 存在 / 0 不存在。
    pub scene_entity_exists: Option<SceneEntityExistsFn>,
    /// 销毁实体：0 成功 / -1 失败。
    pub scene_destroy_entity: Option<SceneDestroyEntityFn>,
}

impl PlayerCSharpContext {
    /// 为场景创建 context；`scene` 必须在本 context 使用期间保持有效。
    pub fn new(scene: *mut Scene) -> Self {
        Self {
            version: PLAYER_CSHARP_CONTEXT_VERSION,
            reserved: 0,
            scene,
            scene_entity_count: Some(scene_entity_count),
            scene_create_cube: Some(scene_create_cube),
            scene_create_entity: Some(scene_create_entity),
            scene_set_entity_position: Some(scene_set_entity_position),
            scene_get_entity_position: Some(scene_get_entity_position),
            log: Some(player_context_log),
            scene_set_entity_rotation: Some(scene_set_entity_rotation),
            scene_get_entity_rotation: Some(scene_get_entity_rotation),
            scene_set_entity_scale: Some(scene_set_entity_scale),
            scene_get_entity_scale: Some(scene_get_entity_scale),
            scene_entity_exists: Some(scene_entity_exists),
            scene_destroy_entity: Some(scene_destroy_entity),
        }
    }

    /// `context` 的裸指针（传给 C# `Initialize(IntPtr)`）。
    pub fn as_ptr(&self) -> *const Self {
        self as *const Self
    }

    /// 版本校验（C# 侧同样校验）。
    pub fn is_supported(&self) -> bool {
        self.version == PLAYER_CSHARP_CONTEXT_VERSION && !self.scene.is_null()
    }
}

impl std::fmt::Debug for PlayerCSharpContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerCSharpContext")
            .field("version", &self.version)
            .field("scene", &self.scene)
            .finish()
    }
}

/// 在 `catch_unwind` + 空指针保护下访问 Scene。
fn with_scene<R>(scene: *mut Scene, default: R, f: impl FnOnce(&mut Scene) -> R) -> R {
    if scene.is_null() {
        return default;
    }
    match catch_unwind(AssertUnwindSafe(|| f(unsafe { &mut *scene }))) {
        Ok(value) => value,
        Err(_) => default,
    }
}

fn find_entity(scene: &Scene, entity_id: u64) -> Option<Entity> {
    scene
        .get_all_entities()
        .into_iter()
        .find(|entity| entity.id == entity_id)
}

/// `scene_entity_count(scene)`：实体数量（空指针/panic → 0）。
pub unsafe extern "C" fn scene_entity_count(scene: *mut Scene) -> u32 {
    with_scene(scene, 0, |scene| {
        scene.entity_count().min(u32::MAX as usize) as u32
    })
}

/// `scene_create_cube(scene)`：创建内置 cube，返回实体 id（失败 0）。
pub unsafe extern "C" fn scene_create_cube(scene: *mut Scene) -> u64 {
    with_scene(scene, 0, |scene| scene.create_cube().id)
}

/// `scene_create_entity(scene)`：创建空实体，返回实体 id（失败 0）。
pub unsafe extern "C" fn scene_create_entity(scene: *mut Scene) -> u64 {
    with_scene(scene, 0, |scene| scene.create_entity().id)
}

/// `scene_set_entity_position(scene, entity, x, y, z)`：0 成功 / -1 失败。
pub unsafe extern "C" fn scene_set_entity_position(
    scene: *mut Scene,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.set_entity_position(entity, Vec3::new(x, y, z));
        0
    })
}

/// `scene_get_entity_position(scene, entity, out x, out y, out z)`：0 成功 / -1 失败。
///
/// # Safety
/// `out_x/out_y/out_z` 必须是可写的 `f32` 指针（C# `out float` 满足）；空指针返回 -1。
pub unsafe extern "C" fn scene_get_entity_position(
    scene: *mut Scene,
    entity_id: u64,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return -1;
    }
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        let Some(position) = scene.get_entity_position(entity) else {
            return -1;
        };
        unsafe {
            *out_x = position.x;
            *out_y = position.y;
            *out_z = position.z;
        }
        0
    })
}

/// `scene_set_entity_rotation(scene, entity, x, y, z)`：设置欧拉角（度），0 成功 / -1 失败。
pub unsafe extern "C" fn scene_set_entity_rotation(
    scene: *mut Scene,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.set_entity_rotation(entity, Quaternion::from_euler_degrees(x, y, z));
        0
    })
}

/// `scene_get_entity_rotation(scene, entity, out x, out y, out z)`：读取欧拉角（度）。
///
/// # Safety
/// `out_x/out_y/out_z` 必须是可写的 `f32` 指针（C# `out float` 满足）；空指针返回 -1。
pub unsafe extern "C" fn scene_get_entity_rotation(
    scene: *mut Scene,
    entity_id: u64,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return -1;
    }
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        let Some(rotation) = scene.get_entity_rotation(entity) else {
            return -1;
        };
        let euler = rotation.to_euler_degrees();
        unsafe {
            *out_x = euler[0];
            *out_y = euler[1];
            *out_z = euler[2];
        }
        0
    })
}

/// `scene_set_entity_scale(scene, entity, x, y, z)`：0 成功 / -1 失败。
pub unsafe extern "C" fn scene_set_entity_scale(
    scene: *mut Scene,
    entity_id: u64,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.set_entity_scale(entity, Vec3::new(x, y, z));
        0
    })
}

/// `scene_get_entity_scale(scene, entity, out x, out y, out z)`：0 成功 / -1 失败。
///
/// # Safety
/// `out_x/out_y/out_z` 必须是可写的 `f32` 指针（C# `out float` 满足）；空指针返回 -1。
pub unsafe extern "C" fn scene_get_entity_scale(
    scene: *mut Scene,
    entity_id: u64,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return -1;
    }
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        let Some(scale) = scene.get_entity_scale(entity) else {
            return -1;
        };
        unsafe {
            *out_x = scale.x;
            *out_y = scale.y;
            *out_z = scale.z;
        }
        0
    })
}

/// `scene_entity_exists(scene, entity)`：1 存在 / 0 不存在（空指针/panic → 0）。
pub unsafe extern "C" fn scene_entity_exists(scene: *mut Scene, entity_id: u64) -> i32 {
    with_scene(scene, 0, |scene| {
        i32::from(find_entity(scene, entity_id).is_some())
    })
}

/// `scene_destroy_entity(scene, entity)`：销毁实体，0 成功 / -1 失败（不存在/空指针）。
pub unsafe extern "C" fn scene_destroy_entity(scene: *mut Scene, entity_id: u64) -> i32 {
    with_scene(scene, -1, |scene| {
        let Some(entity) = find_entity(scene, entity_id) else {
            return -1;
        };
        scene.remove_entity(entity);
        0
    })
}

/// `log(ptr)`：把 C 字符串写到 stderr（Player 日志/控制台）；空指针/panic 安全。
///
/// 该函数仅用于 C# 侧需要把消息回传 Rust 日志时；C# 也可以直接 Console.Error。
pub unsafe extern "C" fn player_context_log(message: *const c_char) {
    if message.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let text = unsafe { CStr::from_ptr(message) }.to_string_lossy();
        eprintln!("[PlayerCSharp] {text}");
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_layout_matches_csharp_contract() {
        assert_eq!(PLAYER_CSHARP_CONTEXT_VERSION, 2);
        // 8 字节头（version + reserved）+ scene 指针 + 12 个函数指针（v1 6 + v2 6）。
        assert_eq!(
            std::mem::size_of::<PlayerCSharpContext>(),
            8 + std::mem::size_of::<usize>() * 13
        );
        assert_eq!(std::mem::align_of::<PlayerCSharpContext>(), 8);
        let scene = Scene::new();
        let context = PlayerCSharpContext::new(&scene as *const Scene as *mut Scene);
        assert!(context.is_supported());
        assert!(context.scene_entity_count.is_some());
        assert!(context.scene_create_cube.is_some());
        assert!(context.scene_create_entity.is_some());
        assert!(context.scene_set_entity_position.is_some());
        assert!(context.scene_get_entity_position.is_some());
        assert!(context.log.is_some());
        // v2 tail。
        assert!(context.scene_set_entity_rotation.is_some());
        assert!(context.scene_get_entity_rotation.is_some());
        assert!(context.scene_set_entity_scale.is_some());
        assert!(context.scene_get_entity_scale.is_some());
        assert!(context.scene_entity_exists.is_some());
        assert!(context.scene_destroy_entity.is_some());
        // 通过 context 里的函数指针调用，验证 ABI 字段可执行。
        let scene_ptr = context.scene;
        let count = unsafe { context.scene_entity_count.unwrap()(scene_ptr) };
        assert_eq!(count, 0);
    }

    #[test]
    fn null_scene_functions_are_safe() {
        unsafe {
            assert_eq!(scene_entity_count(std::ptr::null_mut()), 0);
            assert_eq!(scene_create_cube(std::ptr::null_mut()), 0);
            assert_eq!(scene_create_entity(std::ptr::null_mut()), 0);
            assert_eq!(
                scene_set_entity_position(std::ptr::null_mut(), 1, 1.0, 2.0, 3.0),
                -1
            );
            let mut x = 9.0f32;
            let mut y = 9.0f32;
            let mut z = 9.0f32;
            assert_eq!(
                scene_get_entity_position(
                    std::ptr::null_mut(),
                    1,
                    &mut x,
                    &mut y,
                    &mut z
                ),
                -1
            );
            assert_eq!((x, y, z), (9.0, 9.0, 9.0), "失败时不写输出");
            // v2：空 scene / 不存在实体均安全。
            assert_eq!(
                scene_set_entity_rotation(std::ptr::null_mut(), 1, 0.0, 90.0, 0.0),
                -1
            );
            assert_eq!(
                scene_get_entity_rotation(
                    std::ptr::null_mut(),
                    1,
                    &mut x,
                    &mut y,
                    &mut z
                ),
                -1
            );
            assert_eq!(
                scene_set_entity_scale(std::ptr::null_mut(), 1, 1.0, 1.0, 1.0),
                -1
            );
            assert_eq!(
                scene_get_entity_scale(
                    std::ptr::null_mut(),
                    1,
                    &mut x,
                    &mut y,
                    &mut z
                ),
                -1
            );
            assert_eq!(scene_entity_exists(std::ptr::null_mut(), 1), 0);
            assert_eq!(scene_destroy_entity(std::ptr::null_mut(), 1), -1);
            player_context_log(std::ptr::null());
        }
    }

    #[test]
    fn functions_create_and_position_entities() {
        let mut scene = Scene::new();
        let scene_ptr = &mut scene as *mut Scene;
        let before = unsafe { scene_entity_count(scene_ptr) };
        assert_eq!(before, 0);

        let cube = unsafe { scene_create_cube(scene_ptr) };
        let empty = unsafe { scene_create_entity(scene_ptr) };
        assert_ne!(cube, 0);
        assert_ne!(empty, 0);
        assert_eq!(unsafe { scene_entity_count(scene_ptr) }, 2);

        assert_eq!(
            unsafe { scene_set_entity_position(scene_ptr, cube, 2.5, 0.5, -1.5) },
            0
        );
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;
        assert_eq!(
            unsafe { scene_get_entity_position(scene_ptr, cube, &mut x, &mut y, &mut z) },
            0
        );
        assert_eq!((x, y, z), (2.5, 0.5, -1.5));

        // 不存在的实体：失败且不 panic。
        assert_eq!(
            unsafe { scene_set_entity_position(scene_ptr, 0xDEAD_BEEF, 1.0, 1.0, 1.0) },
            -1
        );
        assert_eq!(
            unsafe {
                scene_get_entity_position(scene_ptr, 0xDEAD_BEEF, &mut x, &mut y, &mut z)
            },
            -1
        );
        // 空输出指针：失败安全。
        assert_eq!(
            unsafe { scene_get_entity_position(scene_ptr, cube, std::ptr::null_mut(), &mut y, &mut z) },
            -1
        );
    }

    #[test]
    fn v2_sets_rotation_scale_and_destroys_entity() {
        let mut scene = Scene::new();
        let scene_ptr = &mut scene as *mut Scene;
        let cube = unsafe { scene_create_cube(scene_ptr) };
        assert_eq!(unsafe { scene_entity_exists(scene_ptr, cube) }, 1);

        // 欧拉角（度）：只绕 Y 轴 90°。
        assert_eq!(
            unsafe { scene_set_entity_rotation(scene_ptr, cube, 0.0, 90.0, 0.0) },
            0
        );
        let mut rx = 0.0f32;
        let mut ry = 0.0f32;
        let mut rz = 0.0f32;
        assert_eq!(
            unsafe { scene_get_entity_rotation(scene_ptr, cube, &mut rx, &mut ry, &mut rz) },
            0
        );
        assert!((rx - 0.0).abs() < 0.1, "rx={rx}");
        assert!((ry - 90.0).abs() < 0.1, "ry={ry}");
        assert!((rz - 0.0).abs() < 0.1, "rz={rz}");
        // core 侧真实生效（不是只改出参）。
        let entity = scene.get_all_entities()[0];
        let euler = scene.get_entity_rotation(entity).unwrap().to_euler_degrees();
        assert!((euler[1] - 90.0).abs() < 0.1, "core euler={euler:?}");

        assert_eq!(
            unsafe { scene_set_entity_scale(scene_ptr, cube, 1.5, 2.0, 0.25) },
            0
        );
        let mut sx = 0.0f32;
        let mut sy = 0.0f32;
        let mut sz = 0.0f32;
        assert_eq!(
            unsafe { scene_get_entity_scale(scene_ptr, cube, &mut sx, &mut sy, &mut sz) },
            0
        );
        assert_eq!((sx, sy, sz), (1.5, 2.0, 0.25));

        // 销毁后 exists=false，重复销毁/读取/设置均失败但不 panic。
        assert_eq!(unsafe { scene_destroy_entity(scene_ptr, cube) }, 0);
        assert_eq!(unsafe { scene_entity_exists(scene_ptr, cube) }, 0);
        assert_eq!(unsafe { scene_destroy_entity(scene_ptr, cube) }, -1);
        assert_eq!(
            unsafe { scene_set_entity_scale(scene_ptr, cube, 1.0, 1.0, 1.0) },
            -1
        );
        assert_eq!(
            unsafe { scene_get_entity_rotation(scene_ptr, cube, &mut rx, &mut ry, &mut rz) },
            -1
        );
        assert_eq!(unsafe { scene_entity_count(scene_ptr) }, 0);
    }
}
