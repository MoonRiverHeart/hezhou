# AI Agent Instructions

## Build Commands

### Rust
```bash
cd engine
cargo build
cargo test
cargo run --bin mono_rotation_demo --features mono      # Mono JIT demo (console)
cargo run --bin mono_triangle_demo --features mono      # Mono JIT + Vulkan triangle
cargo run --bin mono_hot_reload_test --features mono    # Mono hot reload test
cargo run --bin rotation_demo --features native-aot    # NativeAOT demo
```

### C# Scripts

#### Mono Version (Development - Hot Reload)
```bash
cd engine/scripts
powershell -ExecutionPolicy Bypass -File build_mono.ps1
# OR
build_mono.bat
```

#### NativeAOT Version (Release - High Performance)
```bash
cd engine/scripts
dotnet build RotationScript.NativeAOT.csproj -c Release
```

## Demo Programs

### mono_triangle_demo
- Vulkan rendering with rotating triangle
- Mono JIT calls C# `UpdateRotation(deltaTime)` to calculate rotation
- Press **R** key to trigger hot reload:
  1. Recompiles C# script using `mcs`
  2. Reloads Mono assembly
  3. New rotation speed takes effect immediately

### mono_rotation_demo
- Console-based demo without rendering
- Tests Mono JIT script execution
- Press Enter every 180 frames to trigger hot reload

### mono_hot_reload_test
- Automated test for hot reload functionality
- Modifies C# source, recompiles, reloads, verifies changes

## Important Notes

### Mono vs NativeAOT
- **Mono JIT**: Development mode, supports hot reload (Assembly unload/reload)
- **NativeAOT**: Release mode, high performance (function pointer calls)
- **Single C# source**: RotationScript.cs uses `#if MONO` / `#if NATIVEAOT`

### Mono/.NET 8 Incompatibility
- .NET 8 compiled DLLs have 0 methods when loaded by Mono
- **Solution**: Use Mono compiler `mcs` for Mono version
- Mono SDK must be installed: https://www.mono-project.com/download/stable/

### wrapped_mono Method Lookup Issue
- `mono_class_get_method_from_name` returns null for valid methods
- **Solution**: Iterate methods with `mono_class_get_methods`, match by name, then use `mono_runtime_invoke`

### DLL Paths
- Mono DLL: `engine/scripts/bin/Mono/Release/net8.0/RotationScript.Mono.dll`
- NativeAOT DLL: `engine/scripts/bin/NativeAOT/Release/net8.0/RotationScript.NativeAOT.dll`

### Hot Reload Flow
1. User modifies `InitializeDefaults()` in RotationScript.cs to set new `_rotationSpeed`
2. Press **R** key in the demo window
3. Rust calls `build_mono.ps1` to recompile C# with `mcs`
4. Rust calls `executor.reload()` to unload old assembly and load new one
5. Rust calls `ResetAll()` which triggers `InitializeDefaults()` with new speed value
6. New rotation speed takes effect immediately

### Example Hot Reload Test
```bash
# 1. Run the demo
cd engine/examples
cargo run --bin mono_triangle_demo --features mono

# 2. While demo is running, edit RotationScript.cs:
#    Change: _rotationSpeed = 90.0f;
#    To:     _rotationSpeed = 180.0f;

# 3. Press R in the demo window

# 4. Observe:
#    [HotReload] Calling ResetAll to reinit static vars...
#    [C# Static Constructor] Initialized: speed=180°/s
#    [HotReload] Speed after reset: 180°/s
#    Frame 120: angle=..., speed=180°/s
```

## Testing

```bash
# Mono JIT + Vulkan triangle (with hot reload)
cd engine/examples
cargo run --bin mono_triangle_demo --features mono

# Mono JIT console demo
cd engine/examples
cargo run --bin mono_rotation_demo --features mono

# Mono hot reload automated test
cd engine/examples
cargo run --bin mono_hot_reload_test --features mono

# NativeAOT version
cd engine/examples
cargo run --bin rotation_demo --features native-aot
```