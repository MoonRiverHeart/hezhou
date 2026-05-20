# Known Bugs

## C# Callback Delegate GC Collection Issue

**Status**: Open  
**Severity**: High  
**Affected**: mono_editor_demo, mono_ui_thunk_demo  

### Description
Button click callbacks work initially but fail after ~20-30 clicks. After pause, callbacks sometimes recover temporarily.

### Symptoms
- Click "新建" or other toolbar buttons ~20 times
- Button stops responding (no log output)
- Previously: pausing for a few seconds allowed temporary recovery
- After adding static delegate storage: immediate failure (worse)

### Root Cause (Suspected)
Mono runtime's GC behavior differs from .NET:
1. `Marshal.GetFunctionPointerForDelegate()` creates unmanaged function pointer
2. Delegate object has no managed reference after this call
3. GC may collect the delegate, invalidating the function pointer
4. Rust stores function pointer, but C# delegate is gone

### Attempted Fixes
1. **Static fields for Register callbacks** (UI.cs)
   - Added `_savedResizeCallback`, `_savedGlobalClickCallback`, etc.
   - Added `_onclickCallbacks` Dictionary for onclick delegates
   
2. **Static fields for EditorScript callbacks** (EditorScript.cs)
   - Added 15 static fields for all button callbacks
   - Initialize() assigns delegates before registering

3. **Result**: Made problem worse (immediate failure instead of gradual)

### Potential Solutions
1. **GCHandle.Alloc()** - Pin delegate in memory
   ```csharp
   GCHandle handle = GCHandle.Alloc(callback, GCHandleType.Normal);
   IntPtr ptr = Marshal.GetFunctionPointerForDelegate(callback);
   // Store handle somewhere to prevent GC
   ```

2. **Store delegate in static field before GetFunctionPointer**
   - Current approach but may be insufficient for Mono

3. **Rust side storage** - Keep managed object reference
   - Mono C# object lifecycle tied to Rust's reference

4. **Thunk wrapper** - Use stable thunk function that receives managed delegate as parameter
   - Current implementation may need review

### Test Steps
1. Run `cargo run --bin mono_editor_demo --features mono --release`
2. Click "新建" button 50+ times rapidly
3. Observe log output - should see `点击"新建"按钮, id=xxx`
4. When callback fails, no log appears

### Related Files
- `engine/scripts/UI.cs` - SetOnClick, RegisterXxxCallback methods
- `engine/scripts/EditorScript.cs` - Button callbacks
- `engine/ui/src/ffi.rs` - ui_button_set_on_click_thunk implementation
- `engine/rhi-vulkan/src/ui_vulkan_renderer.rs` - callback storage

### References
- [Mono Internals - Delegate marshaling](https://www.mono-project.com/docs/advanced/pinvoke/)
- [GCHandle documentation](https://docs.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.gchandle)