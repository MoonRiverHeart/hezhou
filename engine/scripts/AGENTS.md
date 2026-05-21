# C# Scripts

Editor UI and scripting API layer. Runs on Mono JIT runtime.

## Overview
C# scripts create all editor UI, manage Scene entities, handle user interaction. UI.cs is the FFI wrapper layer (partial class split across multiple files); EditorScript.cs is the application logic.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add C# widget wrapper | `UI.Widgets.cs` or `UI.ComplexWidgets.cs` or `UI.NewWidgets.cs` | Partial class method + helper class |
| Add FFI delegate/FfiContext field | `UI.cs` | Delegates and FfiContext only in main file |
| Edit editor layout | `EditorScript.cs` CreateEditorLayout() | ~1693 lines |
| Fix UI state bug | `EditorScript.cs` ShowMainLayout() | Full rebuild on state switch |
| Fix callback | `EditorScript.cs` + UI partial files | Delegate must be static field (not local) |
| Add asset/project API | `UI.AssetProject.cs` AssetLibrary/Project classes | New static methods |
| Add Scene API | `UI.Scene.cs` Scene/Entity classes | New static methods |

## Key Files
| File | Role | Lines |
|------|------|-------|
| `UI.cs` | FFI delegates, FfiContext, static fields, InitFromContext, Register callbacks | ~750 |
| `UI.Widgets.cs` | VStack/HStack/Button/Label/Panel + widget creation/operation methods | ~600 |
| `UI.ComplexWidgets.cs` | Dropdown/InputField/List + Dropdown/InputField methods | ~400 |
| `UI.NewWidgets.cs` | TabWidget/TreeView/PopupMenu/GridView/Dialog/FileBrowser + methods | ~900 |
| `UI.Scene.cs` | Scene/Entity/ScriptBinding/GameState + Scene FFI methods | ~600 |
| `UI.AssetProject.cs` | AssetLibrary/Project + Asset/Project FFI methods | ~400 |
| `EditorScript.cs` | Editor application logic | ~1693 |
| `DFX.cs` | Logging bridge | ~50 |
| `AssetProjectTest.cs` | Asset/Project test script | ~100 |

## Conventions
- All callback delegates stored as **static fields** (GC won't collect them)
- FfiContext struct fields: IntPtr for each FFI function pointer
- Widget creation: `UI.CreateXxx(parentId, x, y, w, h)` returns ulong widget id
- Widget operations: `UI.XxxSetYyy(widgetId, value)` pattern
- Scene API: `UI.SceneXxx(scenePtr, entityId, ...)` pattern
- AssetLibrary API: `AssetLibrary.GetXxx()` static methods
- Project API: `Project.CreateNew(name, path)` static methods

## Anti-Patterns
- NEVER store callback delegates as local variables — must be static fields (GC collection risk)
- NEVER skip FfiContext initialization for new FFI functions
- NEVER use .NET APIs not available in Mono mcs (no LINQ advanced, no async)
- NEVER destroy/recreate widgets on state switch without preserving IDs — use SetWidgetVisible instead
- NEVER define delegates or FfiContext fields outside `UI.cs` main file — delegates and struct must be in single file
- NEVER duplicate static fields across partial class files — all `_ffi`, `_widgetTree`, callback dictionaries in UI.cs only