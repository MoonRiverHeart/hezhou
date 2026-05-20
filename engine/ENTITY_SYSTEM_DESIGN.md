# Entity System Architecture Design

## Overview

Reference: Unreal Engine Details Panel architecture

## Goals

1. Remove camera pitch limits (free movement in game mode)
2. Create assets from asset panel (Entity, Script)
3. Entity properties reflection to Rust
4. Script controls Entity behavior (rotation, etc.)
5. Hot reload Entity-Script binding
6. Dropdown UI for script selection

## Architecture

### 1. Asset Management

```
AssetPanel
├── CreateEntityButton → Creates new Entity in Scene
├── CreateScriptButton → Opens script editor
├── AssetList
│   ├── Entities (clickable to select)
│   └── Scripts (.cs files, clickable to assign)
```

### 2. Entity System

```rust
// Rust side
struct Entity {
    id: u64,
    name: String,
    transform: Transform,  // position, rotation, scale
    scripts: Vec<ScriptBinding>,
    selected: bool,
}

struct Transform {
    position: Vector3,
    rotation: Quaternion,
    scale: Vector3,
}

struct ScriptBinding {
    script_path: String,
    class_name: String,
    enabled: bool,
}
```

```csharp
// C# side
class Entity {
    ulong Id;
    string Name;
    Transform Transform;
    List<ScriptBinding> Scripts;
}

class Transform {
    Vector3 Position;
    Quaternion Rotation;
    Vector3 Scale;
    
    // Reflection properties for UI
    float PositionX, PositionY, PositionZ;
    float RotationX, RotationY, RotationZ;  // Euler angles
    float ScaleX, ScaleY, ScaleZ;
}
```

### 3. Properties Panel (UE-style)

```
PropertiesPanel
├── Header: "Entity: [Name]"
├── Section: Transform
│   ├── CategoryHeader: "Transform" (collapsible)
│   ├── Property: Position (Vector3)
│   │   ├── Label: "Position"
│   │   ├── HStack: [X: InputField] [Y: InputField] [Z: InputField]
│   ├── Property: Rotation (Euler angles)
│   │   ├── Label: "Rotation"
│   │   ├── HStack: [X: InputField] [Y: InputField] [Z: InputField]
│   ├── Property: Scale (Vector3)
│   │   ├── Label: "Scale"
│   │   ├── HStack: [X: InputField] [Y: InputField] [Z: InputField]
├── Section: Scripts
│   ├── CategoryHeader: "Scripts" (collapsible)
│   ├── ScriptSlot: [Dropdown: Select Script] [Button: Add]
│   ├── ScriptList
│   │   ├── ScriptItem: "[ScriptName.cs]" [Button: Remove] [Checkbox: Enabled]
```

### 4. Dropdown Component

```
Dropdown
├── Button: "[Selected Value ▼]"
├── DropdownPanel (hidden initially)
│   ├── VStack: Options
│   │   ├── OptionButton: "Script1.cs"
│   │   ├── OptionButton: "Script2.cs"
│   │   ├── ...
```

API:
```csharp
// C#
class Dropdown {
    ulong Id;
    string SelectedValue;
    List<string> Options;
    
    void SetOptions(List<string> options);
    void SetOnSelect(Action<string> callback);
    void SetSelected(string value);
}
```

### 5. Script System

```
Scripts/
├── EditorScript.cs
├── RotationScript.cs
├── CustomScript.cs (user created)

// Script interface
interface IEntityScript {
    void OnStart(Entity entity);
    void OnUpdate(Entity entity, float deltaTime);
    void OnDestroy(Entity entity);
}
```

### 6. Hot Reload Flow

```
1. User edits script in TextEdit
2. Click "Hot Reload" button
3. Rust: 
   - Recompile script (mcs)
   - Reload Mono assembly
   - Preserve Entity-Script bindings
4. C#:
   - Reinitialize script instances
   - Resume execution
```

## Implementation Steps

### Phase 1: Core Infrastructure
1. Remove camera pitch limit
2. Create Dropdown widget (Rust + C#)
3. Add InputField widget for property editing

### Phase 2: Entity System
4. Add Entity name/properties to Rust Scene
5. Implement Entity reflection (Rust → C#)
6. Add InputField FFI for Transform editing

### Phase 3: Script System
7. Scan scripts directory for .cs files
8. Implement Dropdown script selection
9. Add Entity-Script binding API

### Phase 4: Hot Reload
10. Implement script compilation flow
11. Implement assembly reload with binding preservation
12. Add UI feedback for reload status

### Phase 5: Asset Panel
13. Add CreateEntity/CreateScript buttons
14. Implement asset list refresh
15. Add asset click handlers

## Key Files to Modify

### Rust
- `engine/ui/src/widgets/dropdown.rs` (NEW)
- `engine/ui/src/widgets/input_field.rs` (NEW)
- `engine/ui/src/widgets/mod.rs`
- `engine/ui/src/ffi.rs`
- `engine/core/src/scene.rs`
- `engine/core/src/entity.rs`
- `engine/scripting/src/ffi_context.rs`

### C#
- `engine/scripts/UI.cs` (Dropdown, InputField API)
- `engine/scripts/EditorScript.cs` (Asset panel, Properties panel)
- `engine/scripts/Entity.cs` (NEW)
- `engine/scripts/ScriptBinding.cs` (NEW)

## UE Details Panel Reference

Key features to implement:
- Collapsible categories (Transform, Scripts, etc.)
- Property rows with label + value editors
- Vector3 as 3 float inputs (X/Y/Z labels)
- Dropdown for enum/script selection
- Array/list properties (Scripts list)