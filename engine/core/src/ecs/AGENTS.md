# ECS System

Entity-Component-System for game Scene management.

## Overview
Scene manages entities with Transform, ScriptBinding, MeshComponent, EntityName components. FFI bridge exposes entity CRUD + transform get/set to C# scripting layer.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add component type | `components/mod.rs` + new component file | Register in Scene struct + add accessor methods |
| Fix entity creation | `scene.rs` | create_entity, destroy_entity, create_cube (auto-adds MeshComponent) |
| Fix entity transform | `scene_ffi_impl.rs` | 548 lines — all scene_* FFI functions |
| Fix script binding | `components/script_binding.rs` | ScriptBinding maps entity → C# script class |
| Fix entity names | `components/entity_name.rs` | entity_names HashMap for debug/display |

## Key Architecture
```
Scene {
    entities: HashSet<u64>,
    transforms: HashMap<u64, Transform>,
    script_bindings: HashMap<u64, ScriptBinding>,
    mesh_components: HashMap<u64, MeshComponent>,
    entity_names: HashMap<u64, String>,
    next_entity_id: u64,
}
```
- Dense component storage (HashMap per component type, not per entity)
- Entity IDs: monotonically increasing u64, `is_valid()` checks >0
- create_cube: auto-adds MeshComponent with cube mesh data

## FFI Bridge (scene_ffi_impl.rs)
All functions are `#[unsafe(no_mangle)] pub extern "C" fn scene_xxx(*mut Scene, ...)`:
- `scene_create_entity` → returns u64 entity_id
- `scene_destroy_entity` → removes from all component maps
- `scene_create_cube` → entity + Transform + MeshComponent
- `scene_set/get_position`, `scene_set/get_rotation_euler`, `scene_set/get_scale`
- `scene_get_entity_count`, `scene_get_entity_ids`

## Conventions
- Component maps: HashMap<u64, ComponentType> per component, not struct-per-entity
- ScriptBinding: `script_id` (Mono object handle) + `script_class_name` (String)
- Entity name: optional String stored in entity_names HashMap
- Scene pointer passed as `*mut Scene` (raw ptr, caller must ensure lifetime)
- Euler angles in degrees (not radians) for FFI