# Core Engine

ECS Scene system, Transform components, AssetLibrary, Project file system, math primitives, event bus.

## Structure
```
core/src/
├── lib.rs          # 10 pub modules
├── ecs/            # Entity-Component-System (see ecs/AGENTS.md)
│   ├── mod.rs      # Scene struct + methods
│   ├── scene.rs    # 529 lines — Scene, create_entity, destroy_entity, create_cube
│   ├── scene_ffi_impl.rs # 548 lines — Scene FFI bridge (create/destroy/get_transform)
│   └── components/ # Transform, ScriptBinding, MeshComponent, EntityName
│       ├── mod.rs
│       ├── transform.rs
│       ├── script_binding.rs # ScriptBinding (script_id + script_class_name)
│       ├── mesh.rs
│       └── entity_name.rs
├── math/           # Math primitives
│   ├── mod.rs
│   ├── vector3.rs
│   ├── matrix4.rs
│   ├── quaternion.rs
│   └── transform.rs
├── event/          # Event bus system
│   ├── mod.rs
│   ├── event_bus.rs
│   └── event_types.rs
├── time_loop.rs    # MainLoop struct (target_fps, vsync)
├── asset_library.rs # AssetLibrary (textures, materials, 5 primitives)
├── asset_ffi.rs    # Asset FFI bridge
├── project.rs      # Project file system (JSON format)
├── scene_ffi.rs    # Scene FFI extern "C" declarations
├── property_ffi.rs # Property FFI (set/get position/rotation/scale)
└── ffi.rs          # Core FFI module root
```

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add entity component | `ecs/components/` + `ecs/mod.rs` | Register in Scene, add getter/setter |
| Fix Scene FFI | `ecs/scene_ffi_impl.rs` | 548 lines, all entity/transform FFI functions |
| Fix asset loading | `asset_library.rs` | Texture creation from RGBA data, material shaders |
| Fix project save/load | `project.rs` | JSON format, ProjectMetadata |
| Fix transform math | `math/` | Vec3, Mat4, Quaternion — used by renderer |

## Key Types
- `Scene`: Main ECS container — entities, transforms, script bindings, entity names
- `Transform`: position (Vec3), rotation (Quaternion), scale (Vec3)
- `ScriptBinding`: script_id (u64) + script_class_name (String) — links entity to C# script
- `AssetLibrary`: texture_id/material_id generation, RGBA → GPU texture upload
- `Project`: JSON-based project files (metadata + entity list)

## Conventions
- Scene FFI: `scene_create_entity`, `scene_destroy_entity`, `scene_create_cube` (snake_case, no ui_ prefix)
- All Scene FFI functions take `*mut Scene` as first arg (raw pointer, not handle)
- Entity ID: u64 (not WidgetId), valid range checked with `is_valid()` method
- Transform FFI: `scene_set_position`, `scene_get_rotation_euler` — euler angles in degrees

## Architecture Notes
- **Inverted dependency**: core depends on scripting (needs FfiContext types). Core is NOT self-contained.
- **Scene FFI raw pointer**: Unlike UI FFI (WidgetTreeHandle), Scene FFI uses `*mut Scene` directly — no handle wrapper