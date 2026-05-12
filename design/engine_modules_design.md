# 游戏引擎模块实现设计

> 基于 `features_tree.md` 特性树，使用 Rust + C# 脚本系统框架实现

---

## 架构总览

```
┌─────────────────────────────────────────────────────────────────────┐
│  C# Script Layer (用户脚本)                                          │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  GamePlugin                                                  │   │
│  │  ├── OnLoad() → 注册回调、初始化逻辑                          │   │
│  │  ├── OnUpdate() → 每帧逻辑                                   │   │
│  │  ├── OnTouch() → 处理触摸事件                                │   │
│  │  └───────────────────────────────────────────────────────  │   │
│  │  │  调用引擎 API                                             │   │
│  │  │  ├── Engine.Render.DrawMesh()                            │   │
│  │  │  ├── Engine.Physics.Raycast()                            │   │
│  │  │  ├── Engine.Audio.PlaySound()                            │   │
│  │  │  ├── Engine.Input.GetAxis()                              │   │
│  │  │  ├── Engine.Scene.LoadScene()                            │   │
│  │  │  └───────────────────────────────────────────────────  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓ FFI (ScriptValue)                    │
├─────────────────────────────────────────────────────────────────────┤
│  Rust Engine Core                                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  modules/                                                    │   │
│  │  ├── core/        ← 核心引擎（主循环、ECS、事件）              │   │
│  │  ├── render/      ← 渲染引擎                                 │   │
│  │  ├── physics/     ← 物理引擎                                 │   │
│  │  ├── audio/       ← 音频引擎                                 │   │
│  │  ├── input/       ← 输入系统                                 │   │
│  │  ├── scene/       ← 场景管理                                 │   │
│  │  ├── asset/       ← 资源管理                                 │   │
│  │  ├── ui/          ← UI系统                                   │   │
│  │  ├── network/     ← 网络系统                                 │   │
│  │  ├── ai/          ← AI系统                                   │   │
│  │  └───────────────────────────────────────────────────────  │   │
│  │  │  engine_api.rs ← FFI 导出给 C#                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              ↓ HarmonyOS NAPI                       │
├─────────────────────────────────────────────────────────────────────┤
│  Platform Layer                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  harmony/                                                    │   │
│  │  ├── XComponent Surface (渲染窗口)                           │   │
│  │  ├── NAPI Events (触摸/键盘/生命周期)                        │   │
│  │  └───────────────────────────────────────────────────────  │   │
│  │  │  OHOS SDK                                                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 1. 核心引擎 (Core Engine)

### 1.1 目录结构

```
engine/core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main_loop.rs       ← 主循环
│   ├── memory/            ← 内存管理
│   │   ├── pool_allocator.rs
│   │   ├── stack_allocator.rs
│   │   └── frame_allocator.rs
│   ├── threading/         ← 多线程
│   │   ├── job_system.rs
│   │   ├── worker_pool.rs
│   │   └── lockfree_queue.rs
│   ├── math/              ← 数学库
│   │   ├── vec.rs
│   │   ├── mat.rs
│   │   ├── quat.rs
│   │   └── transform.rs
│   ├── ecs/               ← ECS
│   │   ├── entity.rs
│   │   ├── component.rs
│   │   ├── system.rs
│   │   ├── world.rs
│   │   └── archetypes.rs
│   ├── event/             ← 事件系统
│   │   ├── bus.rs
│   │   ├── dispatcher.rs
│   │   └── queue.rs
│   └── engine.rs          ← Engine 主结构
```

### 1.2 FFI 接口设计

```rust
// engine_api.rs

#[unsafe(no_mangle)]
pub extern "C" fn engine_create() -> *mut Engine;

#[unsafe(no_mangle)]
pub extern "C" fn engine_destroy(engine: *mut Engine);

#[unsafe(no_mangle)]
pub extern "C" fn engine_run_frame(engine: *mut Engine, delta_time: f32);

#[unsafe(no_mangle)]
pub extern "C" fn engine_get_time(engine: *mut Engine) -> f64;

// ECS API
#[unsafe(no_mangle)]
pub extern "C" fn ecs_create_entity(engine: *mut Engine) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn ecs_destroy_entity(engine: *mut Engine, entity_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn ecs_add_component(engine: *mut Engine, entity_id: u64, component_type: u32, data: ScriptValue);

#[unsafe(no_mangle)]
pub extern "C" fn ecs_get_component(engine: *mut Engine, entity_id: u64, component_type: u32) -> ScriptValue;

#[unsafe(no_mangle)]
pub extern "C" fn ecs_remove_component(engine: *mut Engine, entity_id: u64, component_type: u32);
```

### 1.3 C# 端绑定

```csharp
// Engine.cs

public static class Engine
{
    public static void Initialize() {
        _enginePtr = NativeMethods.engine_create();
    }
    
    public static void Shutdown() {
        NativeMethods.engine_destroy(_enginePtr);
    }
    
    public static void RunFrame(float deltaTime) {
        NativeMethods.engine_run_frame(_enginePtr, deltaTime);
    }
    
    public static double Time => NativeMethods.engine_get_time(_enginePtr);
}

// Entity.cs

public struct Entity
{
    public ulong Id;
    
    public void AddComponent<T>(T component) where T : struct {
        var type = ComponentRegistry.GetTypeId<T>();
        var data = ScriptValue.FromObject(component);
        NativeMethods.ecs_add_component(Engine._enginePtr, Id, type, data);
    }
    
    public T GetComponent<T>() where T : struct {
        var type = ComponentRegistry.GetTypeId<T>();
        var data = NativeMethods.ecs_get_component(Engine._enginePtr, Id, type);
        return data.GetObject<T>();
    }
    
    public void Destroy() {
        NativeMethods.ecs_destroy_entity(Engine._enginePtr, Id);
    }
}
```

---

## 2. 渲染引擎 (Render Engine)

### 2.1 目录结构

```
engine/render/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── device.rs          ← 图形设备抽象
│   ├── pipeline/          ← 渲染管线
│   │   ├── forward.rs
│   │   ├── deferred.rs
│   │   └── pass.rs
│   ├── shader/            ← 着色器
│   │   ├── compiler.rs
│   │   ├── program.rs
│   │   └── variant.rs
│   ├── material/          ← 材质
│   │   ├── pbr.rs
│   │   ├── instance.rs
│   │   └── texture.rs
│   ├── light/             ← 光照
│   │   ├── directional.rs
│   │   ├── point.rs
│   │   ├── spot.rs
│   │   └── gi.rs
│   ├── postprocess/       ← 后处理
│   │   ├── bloom.rs
│   │   ├── tonemap.rs
│   │   ├── dof.rs
│   │   └── antialias.rs
│   ├── mesh/              ← 网格
│   │   ├── model.rs
│   │   ├── primitive.rs
│   │   └── skeleton.rs
│   ├── camera.rs          ← 相机
│   ├── renderer.rs        ← 渲染器
│   └── surface.rs         ← HarmonyOS Surface 管理
```

### 2.2 HarmonyOS Surface 集成

```rust
// surface.rs

use crate::harmony::native_window::OH_NativeWindow;

pub struct RenderSurface {
    window: *mut OH_NativeWindow,
    width: i32,
    height: i32,
   egl_context: Option<EGLContext>,
}

impl RenderSurface {
    pub fn create(window: *mut OH_NativeWindow, width: i32, height: i32) -> Self {
        // 初始化 OpenGL ES / Vulkan
        let egl = EGLContext::create(window, width, height);
        
        Self {
            window,
            width,
            height,
            egl_context: Some(egl),
        }
    }
    
    pub fn resize(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        // 调整 EGL viewport
    }
    
    pub fn present(&self) {
        // Swap buffers
    }
    
    pub fn destroy(&mut self) {
        // 清理 EGL
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn render_init_surface(surface: *mut RenderSurface) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn render_resize(surface: *mut RenderSurface, width: i32, height: i32);

#[unsafe(no_mangle)]
pub extern "C" fn render_present(surface: *mut RenderSurface);
```

### 2.3 FFI 接口

```rust
#[unsafe(no_mangle)]
pub extern "C" fn render_create_camera(engine: *mut Engine) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn render_set_camera_position(camera_id: u64, x: f32, y: f32, z: f32);

#[unsafe(no_mangle)]
pub extern "C" fn render_set_camera_rotation(camera_id: u64, quat: ScriptValue);

#[unsafe(no_mangle)]
pub extern "C" fn render_create_mesh(engine: *mut Engine, mesh_data: ScriptValue) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn render_draw_mesh(mesh_id: u64, material_id: u64, transform: ScriptValue);

#[unsafe(no_mangle)]
pub extern "C" fn render_create_material(engine: *mut Engine, shader_path: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn render_set_material_texture(material_id: u64, slot: u32, texture_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn render_create_texture(engine: *mut Engine, path: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn render_set_clear_color(engine: *mut Engine, r: f32, g: f32, b: f32, a: f32);
```

### 2.4 C# 端绑定

```csharp
public static class Render
{
    public static Camera CreateCamera() {
        var id = NativeMethods.render_create_camera(Engine._enginePtr);
        return new Camera(id);
    }
    
    public static Mesh LoadMesh(string path) {
        var data = Asset.LoadMeshData(path);
        var id = NativeMethods.render_create_mesh(Engine._enginePtr, data);
        return new Mesh(id);
    }
    
    public static Material CreateMaterial(string shaderPath) {
        var id = NativeMethods.render_create_material(Engine._enginePtr, shaderPath);
        return new Material(id);
    }
    
    public static Texture LoadTexture(string path) {
        var id = NativeMethods.render_create_texture(Engine._enginePtr, path);
        return new Texture(id);
    }
    
    public static void SetClearColor(Color color) {
        NativeMethods.render_set_clear_color(Engine._enginePtr, color.R, color.G, color.B, color.A);
    }
}

public class Camera
{
    public ulong Id;
    
    public void SetPosition(Vector3 pos) {
        NativeMethods.render_set_camera_position(Id, pos.X, pos.Y, pos.Z);
    }
    
    public void SetRotation(Quaternion rot) {
        NativeMethods.render_set_camera_rotation(Id, ScriptValue.FromQuaternion(rot));
    }
}

public class Mesh
{
    public ulong Id;
    
    public void Draw(Material material, Transform transform) {
        NativeMethods.render_draw_mesh(Id, material.Id, ScriptValue.FromTransform(transform));
    }
}
```

---

## 3. 物理引擎 (Physics Engine)

### 3.1 目录结构

```
engine/physics/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── world.rs           ← 物理世界
│   ├── collider/          ← 碰撞体
│   │   ├── box.rs
│   │   ├── sphere.rs
│   │   ├── capsule.rs
│   │   ├── convex.rs
│   │   └── mesh.rs
│   ├── rigidbody.rs       ← 刚体
│   ├── raycast.rs         ← 射线检测
│   ├── joint/             ← 关节
│   │   ├── hinge.rs
│   │   ├── slider.rs
│   │   ├── spring.rs
│   │   └── fixed.rs
│   ├── material.rs        ← 物理材质
│   └── trigger.rs         ← 触发器
```

### 3.2 FFI 接口

```rust
#[unsafe(no_mangle)]
pub extern "C" fn physics_create_world(engine: *mut Engine) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn physics_step(world_id: u64, delta_time: f32);

#[unsafe(no_mangle)]
pub extern "C" fn physics_create_rigidbody(world_id: u64, entity_id: u64, collider_type: u32, collider_data: ScriptValue) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn physics_set_rigidbody_mass(body_id: u64, mass: f32);

#[unsafe(no_mangle)]
pub extern "C" fn physics_add_force(body_id: u64, force: ScriptValue);

#[unsafe(no_mangle)]
pub extern "C" fn physics_add_impulse(body_id: u64, impulse: ScriptValue);

#[unsafe(no_mangle)]
pub extern "C" fn physics_raycast(world_id: u64, origin: ScriptValue, direction: ScriptValue, max_distance: f32) -> ScriptValue;

#[unsafe(no_mangle)]
pub extern "C" fn physics_get_rigidbody_position(body_id: u64) -> ScriptValue;

#[unsafe(no_mangle)]
pub extern "C" fn physics_set_rigidbody_position(body_id: u64, position: ScriptValue);

#[unsafe(no_mangle)]
pub extern "C" fn physics_create_joint(world_id: u64, body_a: u64, body_b: u64, joint_type: u32, config: ScriptValue) -> u64;
```

### 3.3 C# 端绑定

```csharp
public static class Physics
{
    public static PhysicsWorld CreateWorld() {
        var id = NativeMethods.physics_create_world(Engine._enginePtr);
        return new PhysicsWorld(id);
    }
    
    public static RaycastHit Raycast(Vector3 origin, Vector3 direction, float maxDistance) {
        var result = NativeMethods.physics_raycast(
            World.Id,
            ScriptValue.FromVector3(origin),
            ScriptValue.FromVector3(direction),
            maxDistance
        );
        return result.GetRaycastHit();
    }
}

public class Rigidbody
{
    public ulong Id;
    
    public void SetMass(float mass) {
        NativeMethods.physics_set_rigidbody_mass(Id, mass);
    }
    
    public void AddForce(Vector3 force) {
        NativeMethods.physics_add_force(Id, ScriptValue.FromVector3(force));
    }
    
    public void AddImpulse(Vector3 impulse) {
        NativeMethods.physics_add_impulse(Id, ScriptValue.FromVector3(impulse));
    }
    
    public Vector3 Position => NativeMethods.physics_get_rigidbody_position(Id).GetVector3();
    
    public void SetPosition(Vector3 pos) {
        NativeMethods.physics_set_rigidbody_position(Id, ScriptValue.FromVector3(pos));
    }
}

public struct RaycastHit
{
    public bool Hit;
    public Vector3 Point;
    public Vector3 Normal;
    public float Distance;
    public ulong EntityId;
}
```

---

## 4. 音频引擎 (Audio Engine)

### 4.1 目录结构

```
engine/audio/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── context.rs         ← 音频上下文
│   ├── source.rs          ← 音频源
│   ├── listener.rs        ← 听者
│   ├── mixer/             ← 混音器
│   │   ├── bus.rs
│   │   ├── effect.rs
│   │   └── filter.rs
│   ├── format/            ← 格式解析
│   │   ├── ogg.rs
│   │   ├── wav.rs
│   │   └── mp3.rs
│   └── spatial.rs         ← 空间音频
```

### 4.2 FFI 接口

```rust
#[unsafe(no_mangle)]
pub extern "C" fn audio_init(engine: *mut Engine) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn audio_create_source(engine: *mut Engine) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn audio_load_clip(engine: *mut Engine, path: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn audio_play(source_id: u64, clip_id: u64, volume: f32, loop: bool);

#[unsafe(no_mangle)]
pub extern "C" fn audio_stop(source_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn audio_set_source_position(source_id: u64, x: f32, y: f32, z: f32);

#[unsafe(no_mangle)]
pub extern "C" fn audio_set_source_volume(source_id: u64, volume: f32);

#[unsafe(no_mangle)]
pub extern "C" fn audio_set_listener_position(engine: *mut Engine, x: f32, y: f32, z: f32);

#[unsafe(no_mangle)]
pub extern "C" fn audio_create_mixer_bus(engine: *mut Engine, name: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn audio_set_bus_volume(bus_id: u64, volume: f32);

#[unsafe(no_mangle)]
pub extern "C" fn audio_add_effect(bus_id: u64, effect_type: u32, params: ScriptValue);
```

### 4.3 C# 端绑定

```csharp
public static class Audio
{
    public static void Initialize() {
        NativeMethods.audio_init(Engine._enginePtr);
    }
    
    public static AudioSource CreateSource() {
        var id = NativeMethods.audio_create_source(Engine._enginePtr);
        return new AudioSource(id);
    }
    
    public static AudioClip LoadClip(string path) {
        var id = NativeMethods.audio_load_clip(Engine._enginePtr, path);
        return new AudioClip(id);
    }
    
    public static MixerBus CreateBus(string name) {
        var id = NativeMethods.audio_create_mixer_bus(Engine._enginePtr, name);
        return new MixerBus(id);
    }
    
    public static void SetListenerPosition(Vector3 pos) {
        NativeMethods.audio_set_listener_position(Engine._enginePtr, pos.X, pos.Y, pos.Z);
    }
}

public class AudioSource
{
    public ulong Id;
    
    public void Play(AudioClip clip, float volume = 1.0f, bool loop = false) {
        NativeMethods.audio_play(Id, clip.Id, volume, loop);
    }
    
    public void Stop() {
        NativeMethods.audio_stop(Id);
    }
    
    public void SetPosition(Vector3 pos) {
        NativeMethods.audio_set_source_position(Id, pos.X, pos.Y, pos.Z);
    }
    
    public void SetVolume(float volume) {
        NativeMethods.audio_set_source_volume(Id, volume);
    }
}
```

---

## 5. 输入系统 (Input System)

### 5.1 与 HarmonyOS 事件对接

```rust
// 已在 harmony/event_bus.rs 实现
// 触摸事件 → EventBus.dispatch_touch_event() → C# OnTouch 回调
// 键盘事件 → EventBus.dispatch_key_event() → C# OnKey 回调

// 扩展：输入状态查询

#[unsafe(no_mangle)]
pub extern "C" fn input_get_touch_position(engine: *mut Engine, pointer_id: i32) -> ScriptValue;

#[unsafe(no_mangle)]
pub extern "C" fn input_get_touch_count(engine: *mut Engine) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn input_is_key_pressed(engine: *mut Engine, keycode: i32) -> bool;

#[unsafe(no_mangle)]
pub extern "C" fn input_get_axis_value(engine: *mut Engine, axis_name: *const c_char) -> f32;

#[unsafe(no_mangle)]
pub extern "C" fn input_is_action_pressed(engine: *mut Engine, action_name: *const c_char) -> bool;
```

### 5.2 C# 端绑定

```csharp
public static class Input
{
    public static Vector2 GetTouchPosition(int pointerId = 0) {
        var result = NativeMethods.input_get_touch_position(Engine._enginePtr, pointerId);
        return result.GetVector2();
    }
    
    public static int TouchCount => NativeMethods.input_get_touch_count(Engine._enginePtr);
    
    public static bool IsKeyPressed(KeyCode key) {
        return NativeMethods.input_is_key_pressed(Engine._enginePtr, (int)key);
    }
    
    public static float GetAxis(string axisName) {
        return NativeMethods.input_get_axis_value(Engine._enginePtr, axisName);
    }
    
    public static bool IsActionPressed(string actionName) {
        return NativeMethods.input_is_action_pressed(Engine._enginePtr, actionName);
    }
}

public enum KeyCode
{
    Back = 1001,
    Home = 1002,
    Menu = 1003,
    VolumeUp = 1004,
    VolumeDown = 1005,
    A = 2001,
    B = 2002,
    C = 2003,
    // ...
}
```

---

## 6. 场景管理 (Scene Management)

### 6.1 目录结构

```
engine/scene/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── manager.rs         ← 场景管理器
│   ├── scene.rs           ← 场景
│   ├── hierarchy.rs       ← 层级管理
│   ├── spatial/           ← 空间划分
│   │   ├── octree.rs
│   │   ├── quadtree.rs
│   │   ├── bvh.rs
│   │   └── grid.rs
│   ├── culling/           ← 剔除
│   │   ├── occlusion.rs
│   │   └── frustum.rs
│   └── lod.rs             ← LOD
```

### 6.2 FFI 接口

```rust
#[unsafe(no_mangle)]
pub extern "C" fn scene_create(engine: *mut Engine, name: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn scene_load(engine: *mut Engine, path: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn scene_unload(engine: *mut Engine, scene_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_active(engine: *mut Engine, scene_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn scene_get_root_entities(scene_id: u64) -> ScriptValue;

#[unsafe(no_mangle)]
pub extern "C" fn scene_find_entity_by_name(scene_id: u64, name: *const c_char) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn scene_set_entity_parent(scene_id: u64, entity_id: u64, parent_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn scene_save(scene_id: u64, path: *const c_char) -> i32;
```

### 6.3 C# 端绑定

```csharp
public static class Scene
{
    public static Scene Create(string name) {
        var id = NativeMethods.scene_create(Engine._enginePtr, name);
        return new Scene(id);
    }
    
    public static Scene Load(string path) {
        var id = NativeMethods.scene_load(Engine._enginePtr, path);
        return new Scene(id);
    }
    
    public static void Unload(Scene scene) {
        NativeMethods.scene_unload(Engine._enginePtr, scene.Id);
    }
    
    public static void SetActive(Scene scene) {
        NativeMethods.scene_set_active(Engine._enginePtr, scene.Id);
    }
    
    public static Entity[] GetRootEntities(Scene scene) {
        var data = NativeMethods.scene_get_root_entities(scene.Id);
        return data.GetEntityArray();
    }
    
    public static Entity FindByName(Scene scene, string name) {
        var id = NativeMethods.scene_find_entity_by_name(scene.Id, name);
        return new Entity(id);
    }
}

public class Scene
{
    public ulong Id;
    
    public void SetParent(Entity entity, Entity parent) {
        NativeMethods.scene_set_entity_parent(Id, entity.Id, parent.Id);
    }
    
    public void Save(string path) {
        NativeMethods.scene_save(Id, path);
    }
}
```

---

## 7. 资源管理 (Asset Management)

### 7.1 目录结构

```
engine/asset/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── manager.rs         ← 资源管理器
│   ├── loader/            ← 加载器
│   │   ├── sync.rs
│   │   ├── async.rs
│   │   └── dependency.rs
│   ├── serializer/        ← 序列化
│   │   ├── binary.rs
│   │   ├── json.rs
│   │   └── yaml.rs
│   ├── bundler/           ← 打包
│   │   ├── pack.rs
│   │   ├── compress.rs
│   │   └── encrypt.rs
│   ├── vfs/               ← 虚拟文件系统
│   │   ├── mount.rs
│   │   ├── path.rs
│   │   ├── cache.rs
│   │   └── archive.rs
│   └── format/            ← 格式解析
│   │   ├── mesh.rs (FBX/OBJ/glTF)
│   │   ├── texture.rs (PNG/JPG/TGA)
│   │   ├── audio.rs (OGG/MP3/WAV)
│   │   └── prefab.rs
```

### 7.2 FFI 接口

```rust
#[unsafe(no_mangle)]
pub extern "C" fn asset_init(engine: *mut Engine) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn asset_load_sync(engine: *mut Engine, path: *const c_char, asset_type: u32) -> u64;

#[unsafe(no_mangle)]
pub extern "C" fn asset_load_async(engine: *mut Engine, path: *const c_char, asset_type: u32, callback: AsyncCallback, context: usize);

#[unsafe(no_mangle)]
pub extern "C" fn asset_unload(engine: *mut Engine, asset_id: u64);

#[unsafe(no_mangle)]
pub extern "C" fn asset_is_loaded(engine: *mut Engine, asset_id: u64) -> bool;

#[unsafe(no_mangle)]
pub extern "C" fn asset_get_path(asset_id: u64) -> *const c_char;

#[unsafe(no_mangle)]
pub extern "C" fn asset_mount_vfs(engine: *mut Engine, archive_path: *const c_char, mount_point: *const c_char) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn asset_bundle_create(engine: *mut Engine, output_path: *const c_char, asset_paths: *const c_char, count: u32) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn asset_bundle_load(engine: *mut Engine, bundle_path: *const c_char) -> i32;
```

### 7.3 C# 端绑定

```csharp
public static class Asset
{
    public static AssetHandle LoadSync(string path, AssetType type) {
        var id = NativeMethods.asset_load_sync(Engine._enginePtr, path, (uint)type);
        return new AssetHandle(id);
    }
    
    public static void LoadAsync(string path, AssetType type, Action<AssetHandle> onComplete) {
        var context = GCHandle.Alloc(onComplete);
        NativeMethods.asset_load_async(
            Engine._enginePtr,
            path,
            (uint)type,
            &AssetLoadThunk,
            (nuint)GCHandle.ToIntPtr(context)
        );
    }
    
    public static void Unload(AssetHandle handle) {
        NativeMethods.asset_unload(Engine._enginePtr, handle.Id);
    }
    
    public static bool IsLoaded(AssetHandle handle) {
        return NativeMethods.asset_is_loaded(Engine._enginePtr, handle.Id);
    }
    
    public static void MountVFS(string archivePath, string mountPoint) {
        NativeMethods.asset_mount_vfs(Engine._enginePtr, archivePath, mountPoint);
    }
    
    public static void CreateBundle(string outputPath, string[] assetPaths) {
        // ...
    }
    
    public static void LoadBundle(string bundlePath) {
        NativeMethods.asset_bundle_load(Engine._enginePtr, bundlePath);
    }
}

public enum AssetType
{
    Mesh = 1,
    Texture = 2,
    Audio = 3,
    Material = 4,
    Shader = 5,
    Scene = 6,
    Prefab = 7,
}
```

---

## 8. 平台抽象层 (Platform Abstraction)

### 8.1 目录结构

```
engine/platform/
├── Cargo.toml
├── src/
│   ├── lib.rs          ← PlatformManager + FFI
│   ├── traits.rs       ← Platform trait
│   ├── event.rs        ← 统一事件类型
│   ├── window.rs       ← WindowHandle
│   ├── glfw_backend.rs ← GLFW 实现
│   └── harmony_backend.rs ← HarmonyOS 实现
```

### 8.2 设计目标

统一平台接口，支持多后端：
- **GLFW**: Windows/Linux/macOS 开发测试
- **HarmonyOS**: 鸿蒙原生平台
- **Win32/X11/Wayland**: 未来原生支持

详见: `design/platform_abstraction_design.md`

### 8.3 FFI 接口

```rust
#[unsafe(no_mangle)]
pub extern "C" fn platform_manager_create() -> *mut PlatformManager;

#[unsafe(no_mangle)]
pub extern "C" fn platform_init_glfw(manager: *mut PlatformManager) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn platform_init_harmony(manager: *mut PlatformManager) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn platform_create_window(manager: *mut PlatformManager, title: *const c_char, width: i32, height: i32) -> WindowHandle;

#[unsafe(no_mangle)]
pub extern "C" fn platform_poll_events(manager: *mut PlatformManager) -> i32;

#[unsafe(no_mangle)]
pub extern "C" fn platform_is_running(manager: *mut PlatformManager) -> bool;

#[unsafe(no_mangle)]
pub extern "C" fn platform_get_time(manager: *mut PlatformManager) -> f64;
```

---

## 9. 实现优先级

### Phase 1 (MVP)

| 模块 | 功能 | 优先级 | 状态 |
|------|------|--------|------|
| **核心引擎** | 主循环、ECS基础、事件系统 | P0 | ✅ 完成 |
| **渲染引擎** | Surface初始化、基础渲染、相机 | P0 | ✅ 完成 |
| **输入系统** | 触摸/键盘事件传递 | P0 | ✅ 完成 |
| **脚本系统** | Rust↔C# 双向调用、Closure支持 | P0 | ✅ 完成 |
| **HarmonyOS对接** | NAPI + XComponent Surface | P0 | ✅ 完成 |
| **平台抽象层** | GLFW + HarmonyOS 统一接口 | P0 | ✅ 完成 |

### Phase 2 (核心功能)

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **渲染引擎** | 材质系统、光照、网格渲染 | P1 |
| **物理引擎** | 碰撞检测、刚体、射线 | P1 |
| **音频引擎** | 播放、混音、空间音频 | P1 |
| **资源管理** | 同步/异步加载、VFS | P1 |
| **场景管理** | 场景加载、层级管理 | P1 |

### Phase 3 (增强功能)

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **渲染引擎** | 后处理、粒子、动画 | P2 |
| **物理引擎** | 关节、物理材质 | P2 |
| **UI系统** | 渲染、布局、交互 | P2 |
| **网络系统** | 传输、同步基础 | P2 |
| **AI系统** | 寻路、状态机 | P2 |

### Phase 4 (高级特性)

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **渲染引擎** | 粒子系统、地形 | P3 |
| **物理引擎** | 布料、流体 | P3 |
| **网络系统** | 带宽优化、延迟补偿 | P3 |
| **AI系统** | 行为树、感知系统 | P3 |
| **编辑器工具** | 可视化编辑器 | P3 |

---

## 10. 已完成工作总结

### 10.1 核心引擎 FFI (`engine/core/src/ffi.rs`)

- 引擎生命周期：`engine_create/destroy/start/stop/run_frame`
- 时间查询：`engine_get_time/delta_time/frame_count`
- ECS 实体管理：`ecs_create_entity/destroy_entity/entity_exists`
- 实体层级：`ecs_set/get_entity_parent/get_entity_children`
- TransformComponent：增删改查完整 API
- NameComponent、TagComponent：增删改查
- EventBus：订阅/发布/分发

### 10.2 渲染引擎 FFI (`engine/render/src/ffi.rs`)

- 渲染引擎生命周期管理
- Surface 初始化/调整/帧控制
- 相机创建与参数设置
- Mesh、Texture、Material 创建接口

### 10.3 平台抽象层 (`engine/platform`)

- Platform trait：统一平台接口
- 统一事件类型：TouchEvent/KeyEvent/MouseEvent/WindowEvent
- WindowHandle：跨平台窗口句柄封装
- GLFW 后端：Windows/Linux/macOS 支持
- HarmonyOS 后端：OH_NativeWindow 支持
- Feature flags：编译时选择后端

### 10.4 测试结果

- 所有模块编译通过
- 15 个单元测试通过
- GLFW Demo 运行成功
- HarmonyOS 模块编译通过

---

## 11. 下一步工作

基于以上设计，Phase 1 MVP 已完成。下一步工作：

### Phase 2 (核心功能)

1. **物理引擎** (`engine/physics/`)
   - 碰撞检测
   - 刚体动力学
   - 射线检测

2. **音频引擎** (`engine/audio/`)
   - 音频播放
   - 混音器
   - 空间音频

3. **资源管理** (`engine/asset/`)
   - 同步/异步加载
   - VFS 虚拟文件系统
   - 资源打包

4. **场景管理** (`engine/scene/`)
   - 场景加载/卸载
   - 层级管理
   - 空间划分

5. **C# 绑定层**
   - Engine API 封装
   - Entity/Component 类型映射
   - Platform 调用封装

---

*文档版本: 2.0 | 更新时间: 2026-05-12*