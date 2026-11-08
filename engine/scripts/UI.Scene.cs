using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Hezhou
{
    public enum GameState
    {
        Editing = 0,
        Running = 1,
        Paused = 2
    }
    
    public struct ScriptBindingInfo
    {
        public string ScriptPath;
        public string ClassName;
        public bool Enabled;
    }
    
    public class ScriptBinding
    {
        public string ScriptPath;
        public string ClassName;
        public bool Enabled;
    }
    
    public class Entity
    {
        public ulong Id;
        public string Name;
        public List<ScriptBinding> Scripts = new List<ScriptBinding>();
    }

    public class Scene
    {
        private IntPtr _scenePtr;
        private Dictionary<ulong, Entity> _entities = new Dictionary<ulong, Entity>();

        public Scene()
        {
            _scenePtr = UI.SceneCreate();
            if (_scenePtr == IntPtr.Zero)
            {
                throw new Exception("Failed to create scene");
            }
        }
        
        // 从已有Rust Scene指针恢复Scene对象（热重载后使用）
        public Scene(IntPtr existingPtr)
        {
            if (existingPtr == IntPtr.Zero)
            {
                throw new Exception("Scene pointer is null — cannot restore");
            }
            _scenePtr = existingPtr;
            
            // 同步Rust侧的root_entities到C# _entities字典
            uint count = UI.SceneRootEntityCount(_scenePtr);
            Log.Info("Editor", "Scene恢复: 同步 " + count + " 个root entities");
            for (uint i = 0; i < count; i++)
            {
                ulong eid = UI.SceneGetRootEntityIdAt(_scenePtr, i);
                if (eid != 0)
                {
                    string name = UI.SceneGetEntityName(_scenePtr, eid);
                    var entity = new Entity { Id = eid, Name = name };
                    _entities[eid] = entity;
                    
                    // 同步ScriptBindings
                    int bindingCount = (int)UI.SceneGetScriptBindingCount(_scenePtr, eid);
                    for (int b = 0; b < bindingCount; b++)
                    {
                        ScriptBindingInfo info = UI.SceneGetScriptBindingInfo(_scenePtr, eid, (ulong)b);
                        entity.Scripts.Add(new ScriptBinding 
                        {
                            ScriptPath = info.ScriptPath,
                            ClassName = info.ClassName,
                            Enabled = info.Enabled
                        });
                    }
                }
            }
        }

        public IntPtr ScenePtr => _scenePtr;

        public void Destroy()
        {
            if (_scenePtr != IntPtr.Zero)
            {
                UI.SceneDestroy(_scenePtr);
                _scenePtr = IntPtr.Zero;
            }
        }

        public ulong CreateCube()
        {
            ulong entityId = UI.SceneCreateCube(_scenePtr);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = "Cube" };
                _entities[entityId] = entity;
            }
            return entityId;
        }

        public ulong CreatePlane()
        {
            ulong entityId = UI.SceneCreatePlane(_scenePtr);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = "Plane" };
                _entities[entityId] = entity;
            }
            return entityId;
        }

        public ulong CreateCornellBox()
        {
            ulong entityId = UI.SceneCreateCornellBox(_scenePtr);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = "CornellBox" };
                _entities[entityId] = entity;
            }
            return entityId;
        }

        public ulong CreateDirectionalLight()
        {
            ulong entityId = UI.SceneCreateDirectionalLight(_scenePtr);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = "DirectionalLight" };
                _entities[entityId] = entity;
            }
            return entityId;
        }
        
        public ulong CreateMeshEntity(int meshType)
        {
            ulong entityId = UI.AssetLibraryCreateMeshEntity(_scenePtr, meshType);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = $"Mesh_{entityId}" };
                _entities[entityId] = entity;
            }
            return entityId;
        }
        
        public ulong CreateEntity()
        {
            ulong entityId = UI.SceneCreateEntity(_scenePtr);
            if (entityId != 0)
            {
                var entity = new Entity { Id = entityId, Name = $"Entity_{entityId}" };
                _entities[entityId] = entity;
            }
            return entityId;
        }
        
        public Entity GetEntity(ulong entityId)
        {
            if (_entities.TryGetValue(entityId, out var entity))
            {
                return entity;
            }
            return null;
        }
        
        public Dictionary<ulong, Entity>.ValueCollection GetAllEntities()
        {
            return _entities.Values;
        }
        
        public int GetEntityCount()
        {
            return _entities.Count;
        }
        
        public ulong GetEntityId(int index)
        {
            if (index < 0 || index >= _entities.Count)
                return 0;
            
            int i = 0;
            foreach (var key in _entities.Keys)
            {
                if (i == index)
                    return key;
                i++;
            }
            return 0;
        }
        
        public void RemoveEntity(ulong entityId)
        {
            if (_entities.ContainsKey(entityId))
            {
                UI.SceneRemoveEntity(_scenePtr, entityId);
                _entities.Remove(entityId);
            }
        }

        public void AttachScript(ulong entityId, string scriptPath, string className)
        {
            UI.SceneAttachScript(_scenePtr, entityId, scriptPath, className);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                var binding = new ScriptBinding { ScriptPath = scriptPath, ClassName = className, Enabled = true };
                entity.Scripts.Add(binding);
            }
        }
        
        public void AttachScriptBinding(ulong entityId, string scriptPath, string className)
        {
            UI.SceneAttachScriptBinding(_scenePtr, entityId, scriptPath, className);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                var binding = new ScriptBinding { ScriptPath = scriptPath, ClassName = className, Enabled = true };
                entity.Scripts.Add(binding);
            }
        }
        
        public void RemoveScriptBinding(ulong entityId, int index)
        {
            UI.SceneRemoveScriptBinding(_scenePtr, entityId, (ulong)index);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                if (index >= 0 && index < entity.Scripts.Count)
                {
                    entity.Scripts.RemoveAt(index);
                }
            }
        }
        
        public int GetScriptBindingCount(ulong entityId)
        {
            return (int)UI.SceneGetScriptBindingCount(_scenePtr, entityId);
        }
        
        public ScriptBindingInfo GetScriptBindingInfo(ulong entityId, int index)
        {
            return UI.SceneGetScriptBindingInfo(_scenePtr, entityId, (ulong)index);
        }
        
        public void SetScriptBindingEnabled(ulong entityId, int index, bool enabled)
        {
            UI.SceneSetScriptBindingEnabled(_scenePtr, entityId, (ulong)index, enabled);
            
            if (_entities.TryGetValue(entityId, out var entity))
            {
                if (index >= 0 && index < entity.Scripts.Count)
                {
                    entity.Scripts[index].Enabled = enabled;
                }
            }
        }

        public void SetGameState(GameState state)
        {
            UI.SceneSetGameState(_scenePtr, (int)state);
        }

        public GameState GetGameState()
        {
            return (GameState)UI.SceneGetGameState(_scenePtr);
        }

        public ulong PickEntity(float ox, float oy, float oz, float dx, float dy, float dz)
        {
            return UI.ScenePickEntity(_scenePtr, ox, oy, oz, dx, dy, dz);
        }

        public void SelectEntity(ulong entityId)
        {
UI.SceneSelectEntity(_scenePtr, entityId);
            UI.SetSelectedEntity(entityId, true);
        }

        public void ClearSelection()
        {
            UI.SetSelectedEntity(0, false);
        }

        public void AddToSelection(ulong entityId)
        {
            UI.SceneAddToSelection(_scenePtr, entityId);
        }

        public void RemoveFromSelection(ulong entityId)
        {
            UI.SceneRemoveFromSelection(_scenePtr, entityId);
        }

        public void ToggleSelection(ulong entityId)
        {
            UI.SceneToggleSelection(_scenePtr, entityId);
        }

        public void SetParent(ulong entityId, ulong parentId)
        {
            UI.SceneSetParent(_scenePtr, entityId, parentId);
        }

        public ulong GetParent(ulong entityId)
        {
            return UI.SceneGetParent(_scenePtr, entityId);
        }

        public ulong GetChildCount(ulong parentId)
        {
            return UI.SceneGetChildCount(_scenePtr, parentId);
        }

        public ulong GetChildId(ulong parentId, ulong index)
        {
            return UI.SceneGetChildId(_scenePtr, parentId, index);
        }

        public void Update(float deltaTime)
        {
            UI.SceneUpdate(_scenePtr, deltaTime);
        }
    }

    public static partial class UI
    {
        public static IntPtr SceneCreate()
        {
            if (_ffi.scene_create == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreate函数指针为空");
                return IntPtr.Zero;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateDelegate>(_ffi.scene_create);
            return func();
        }

        public static void SceneDestroy(IntPtr scene)
        {
            if (_ffi.scene_destroy == IntPtr.Zero)
            {
                Log.Error("C#", "SceneDestroy函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneDestroyDelegate>(_ffi.scene_destroy);
            func(scene);
        }
        
        // 获取当前已存在的Scene指针（热重载时用于恢复Scene，而非创建新Scene）
        public static IntPtr SceneGetExistingPtr()
        {
            if (_ffi.scene_get_existing_ptr == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetExistingPtr函数指针为空");
                return IntPtr.Zero;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetExistingPtrDelegate>(_ffi.scene_get_existing_ptr);
            return func();
        }
        
        public static uint SceneRootEntityCount(IntPtr scene)
        {
            if (_ffi.scene_root_entity_count == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRootEntityCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRootEntityCountDelegate>(_ffi.scene_root_entity_count);
            return func(scene);
        }
        
        public static ulong SceneGetRootEntityIdAt(IntPtr scene, uint index)
        {
            if (_ffi.scene_get_root_entity_id_at == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetRootEntityIdAt函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetRootEntityIdAtDelegate>(_ffi.scene_get_root_entity_id_at);
            return func(scene, index);
        }

        public static ulong SceneCreateCube(IntPtr scene)
        {
            if (_ffi.scene_create_cube == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateCube函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateCubeDelegate>(_ffi.scene_create_cube);
            return func(scene);
        }

        public static ulong SceneCreatePlane(IntPtr scene)
        {
            if (_ffi.scene_create_plane == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreatePlane函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreatePlaneDelegate>(_ffi.scene_create_plane);
            return func(scene);
        }

        public static ulong SceneCreateCornellBox(IntPtr scene)
        {
            if (_ffi.scene_create_cornell_box == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateCornellBox函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateCornellBoxDelegate>(_ffi.scene_create_cornell_box);
            return func(scene);
        }

        public static ulong SceneCreateDirectionalLight(IntPtr scene)
        {
            if (_ffi.scene_create_directional_light == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateDirectionalLight函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateDirectionalLightDelegate>(_ffi.scene_create_directional_light);
            return func(scene);
        }

        public static void SceneAttachScript(IntPtr scene, ulong entityId, string scriptPath, string className)
        {
            if (_ffi.scene_attach_script == IntPtr.Zero)
            {
                Log.Error("C#", "SceneAttachScript函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneAttachScriptDelegate>(_ffi.scene_attach_script);
            func(scene, entityId, scriptPath, className);
        }

        public static void SceneSetGameState(IntPtr scene, int state)
        {
            if (_ffi.scene_set_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetGameState函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetGameStateDelegate>(_ffi.scene_set_game_state);
            func(scene, state);
        }

        public static int SceneGetGameState(IntPtr scene)
        {
            if (_ffi.scene_get_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetGameState函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetGameStateDelegate>(_ffi.scene_get_game_state);
            return func(scene);
        }

        public static ulong ScenePickEntity(IntPtr scene, float ox, float oy, float oz, float dx, float dy, float dz)
        {
            if (_ffi.scene_pick_entity == IntPtr.Zero)
            {
                Log.Error("C#", "ScenePickEntity函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ScenePickEntityDelegate>(_ffi.scene_pick_entity);
            return func(scene, ox, oy, oz, dx, dy, dz);
        }

        public static void SceneSelectEntity(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_select_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSelectEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSelectEntityDelegate>(_ffi.scene_select_entity);
            func(scene, entityId);
        }

        public static void SceneAddToSelection(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_add_to_selection == IntPtr.Zero) return;
            var func = Marshal.GetDelegateForFunctionPointer<SceneAddToSelectionDelegate>(_ffi.scene_add_to_selection);
            func(scene, entityId);
        }

        public static void SceneRemoveFromSelection(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_remove_from_selection == IntPtr.Zero) return;
            var func = Marshal.GetDelegateForFunctionPointer<SceneRemoveFromSelectionDelegate>(_ffi.scene_remove_from_selection);
            func(scene, entityId);
        }

        public static void SceneToggleSelection(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_toggle_selection == IntPtr.Zero) return;
            var func = Marshal.GetDelegateForFunctionPointer<SceneToggleSelectionDelegate>(_ffi.scene_toggle_selection);
            func(scene, entityId);
        }

        public static ulong SceneGetFirstSelectedEntityId(IntPtr scene)
        {
            if (_ffi.scene_get_first_selected_entity_id == IntPtr.Zero) return 0;
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetFirstSelectedEntityIdDelegate>(_ffi.scene_get_first_selected_entity_id);
            return func(scene);
        }

        public static void SceneUpdate(IntPtr scene, float deltaTime)
        {
            if (_ffi.scene_update == IntPtr.Zero)
            {
                Log.Error("C#", "SceneUpdate函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneUpdateDelegate>(_ffi.scene_update);
            func(scene, deltaTime);
        }

        public static void SetRendererGameState(int state)
        {
            if (_ffi.set_renderer_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "SetRendererGameState函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetRendererGameStateDelegate>(_ffi.set_renderer_game_state);
            func(state);
        }

        public static int GetRendererGameState()
        {
            if (_ffi.get_renderer_game_state == IntPtr.Zero)
            {
                Log.Error("C#", "GetRendererGameState函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetRendererGameStateDelegate>(_ffi.get_renderer_game_state);
            return func();
        }

        public static void SetEntityTransform(float px, float py, float pz,
                                               float rx, float ry, float rz, float rw,
                                               float sx, float sy, float sz)
        {
            if (_ffi.set_entity_transform == IntPtr.Zero)
            {
                Log.Error("C#", "SetEntityTransform函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetEntityTransformDelegate>(_ffi.set_entity_transform);
            func(px, py, pz, rx, ry, rz, rw, sx, sy, sz);
        }

        public static void SetEntityAngle(float angle)
        {
            if (_ffi.set_entity_angle == IntPtr.Zero)
            {
                Log.Error("C#", "SetEntityAngle函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetEntityAngleDelegate>(_ffi.set_entity_angle);
            func(angle);
        }

        public static float GetEntityAngle()
        {
            if (_ffi.get_entity_angle == IntPtr.Zero)
            {
                Log.Error("C#", "GetEntityAngle函数指针为空");
                return 0.0f;
            }
            var func = Marshal.GetDelegateForFunctionPointer<GetEntityAngleDelegate>(_ffi.get_entity_angle);
            return func();
        }

        public static void SceneGetEntityPosition(IntPtr scene, ulong entityId, out float x, out float y, out float z)
        {
            if (_ffi.scene_get_entity_position == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityPosition函数指针为空");
                x = 0; y = 0; z = 0;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityPositionDelegate>(_ffi.scene_get_entity_position);
            func(scene, entityId, out x, out y, out z);
        }

        public static void SceneGetEntityRotation(IntPtr scene, ulong entityId, out float x, out float y, out float z, out float w)
        {
            if (_ffi.scene_get_entity_rotation == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityRotation函数指针为空");
                x = 0; y = 0; z = 0; w = 1;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityRotationDelegate>(_ffi.scene_get_entity_rotation);
            func(scene, entityId, out x, out y, out z, out w);
        }

        public static void SceneGetEntityScale(IntPtr scene, ulong entityId, out float x, out float y, out float z)
        {
            if (_ffi.scene_get_entity_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityScale函数指针为空");
                x = 1; y = 1; z = 1;
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityScaleDelegate>(_ffi.scene_get_entity_scale);
            func(scene, entityId, out x, out y, out z);
        }

        public static void SceneSetEntityPosition(IntPtr scene, ulong entityId, float x, float y, float z)
        {
            if (_ffi.scene_set_entity_position == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetEntityPosition函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityPositionDelegate>(_ffi.scene_set_entity_position);
            func(scene, entityId, x, y, z);
        }

        public static void SceneSetEntityScale(IntPtr scene, ulong entityId, float x, float y, float z)
        {
            if (_ffi.scene_set_entity_scale == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetEntityScale函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityScaleDelegate>(_ffi.scene_set_entity_scale);
            func(scene, entityId, x, y, z);
        }

        public static void SceneRotateEntity(IntPtr scene, ulong entityId, float angleDegrees)
        {
            if (_ffi.scene_rotate_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRotateEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRotateEntityDelegate>(_ffi.scene_rotate_entity);
            func(scene, entityId, angleDegrees);
        }

        public static void SceneSetEntityMaterial(IntPtr scene, ulong entityId, string material)
        {
            if (_ffi.scene_set_entity_material == IntPtr.Zero) return;
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityMaterialDelegate>(_ffi.scene_set_entity_material);
            func(scene, entityId, material);
        }

        public static string SceneGetEntityMaterial(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_entity_material == IntPtr.Zero) return "";
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityMaterialDelegate>(_ffi.scene_get_entity_material);
            IntPtr buffer = Marshal.AllocHGlobal(256);
            try
            {
                int len = func(scene, entityId, buffer, 256);
                return len > 0 ? Marshal.PtrToStringAnsi(buffer, len) : "";
            }
            finally { Marshal.FreeHGlobal(buffer); }
        }

        public static void SceneSetEntityName(IntPtr scene, ulong entityId, string name)
        {
            if (_ffi.scene_set_entity_name == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetEntityName函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityNameDelegate>(_ffi.scene_set_entity_name);
            func(scene, entityId, name);
        }

        public static string SceneGetEntityName(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_entity_name == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityName函数指针为空");
                return "";
            }
            
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityNameDelegate>(_ffi.scene_get_entity_name);
            int len = func(scene, entityId, buffer, 256);
            
            string result = len > 0 ? Marshal.PtrToStringAnsi(buffer, len) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            
            return result;
        }

        public static void SetSelectedEntity(ulong entityId, bool selected)
        {
            if (_ffi.set_selected_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SetSelectedEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SetSelectedEntityDelegate>(_ffi.set_selected_entity);
            func(entityId, selected);
        }
        
        public static void SceneAttachScriptBinding(IntPtr scene, ulong entityId, string scriptPath, string className)
        {
            if (_ffi.scene_attach_script_binding == IntPtr.Zero)
            {
                Log.Error("C#", "SceneAttachScriptBinding函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneAttachScriptBindingDelegate>(_ffi.scene_attach_script_binding);
            func(scene, entityId, scriptPath, className);
        }
        
        public static void SceneRemoveScriptBinding(IntPtr scene, ulong entityId, ulong scriptIndex)
        {
            if (_ffi.scene_remove_script_binding == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRemoveScriptBinding函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRemoveScriptBindingDelegate>(_ffi.scene_remove_script_binding);
            func(scene, entityId, scriptIndex);
        }
        
        public static ulong SceneGetScriptBindingCount(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_script_binding_count == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetScriptBindingCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetScriptBindingCountDelegate>(_ffi.scene_get_script_binding_count);
            return func(scene, entityId);
        }
        
        public static ScriptBindingInfo SceneGetScriptBindingInfo(IntPtr scene, ulong entityId, ulong index)
        {
            var info = new ScriptBindingInfo();
            if (_ffi.scene_get_script_binding_info == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetScriptBindingInfo函数指针为空");
                return info;
            }
            
            IntPtr pathBuffer = Marshal.AllocHGlobal(256);
            IntPtr classBuffer = Marshal.AllocHGlobal(256);
            
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetScriptBindingInfoDelegate>(_ffi.scene_get_script_binding_info);
            bool success = func(scene, entityId, index, pathBuffer, 256, classBuffer, 256, out info.Enabled);
            
            if (success)
            {
                info.ScriptPath = Marshal.PtrToStringAnsi(pathBuffer) ?? "";
                info.ClassName = Marshal.PtrToStringAnsi(classBuffer) ?? "";
            }
            
            Marshal.FreeHGlobal(pathBuffer);
            Marshal.FreeHGlobal(classBuffer);
            
            return info;
        }
        
        public static void SceneSetScriptBindingEnabled(IntPtr scene, ulong entityId, ulong index, bool enabled)
        {
            if (_ffi.scene_set_script_binding_enabled == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetScriptBindingEnabled函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetScriptBindingEnabledDelegate>(_ffi.scene_set_script_binding_enabled);
            func(scene, entityId, index, enabled);
        }
        
        public static void SceneSetScriptBindingInstanceId(IntPtr scene, ulong entityId, ulong index, ulong instanceId)
        {
            if (_ffi.scene_set_script_binding_instance_id == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetScriptBindingInstanceId函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetScriptBindingInstanceIdDelegate>(_ffi.scene_set_script_binding_instance_id);
            func(scene, entityId, index, instanceId);
        }
        
        public static ulong SceneGetScriptBindingInstanceId(IntPtr scene, ulong entityId, ulong index)
        {
            if (_ffi.scene_get_script_binding_instance_id == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetScriptBindingInstanceId函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetScriptBindingInstanceIdDelegate>(_ffi.scene_get_script_binding_instance_id);
            return func(scene, entityId, index);
        }
        
        public static ulong SceneCreateEntity(IntPtr scene)
        {
            if (_ffi.scene_create_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneCreateEntity函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneCreateEntityDelegate>(_ffi.scene_create_entity);
            return func(scene);
        }
        
        public static ulong SceneGetEntityCount(IntPtr scene)
        {
            if (_ffi.scene_get_entity_count == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityCount函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityCountDelegate>(_ffi.scene_get_entity_count);
            return func(scene);
        }
        
        public static ulong SceneGetEntityId(IntPtr scene, ulong index)
        {
            if (_ffi.scene_get_entity_id == IntPtr.Zero)
            {
                Log.Error("C#", "SceneGetEntityId函数指针为空");
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityIdDelegate>(_ffi.scene_get_entity_id);
            return func(scene, index);
        }
        
        public static void SceneRemoveEntity(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_remove_entity == IntPtr.Zero)
            {
                Log.Error("C#", "SceneRemoveEntity函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneRemoveEntityDelegate>(_ffi.scene_remove_entity);
            func(scene, entityId);
        }
        
        public static void SceneSetParent(IntPtr scene, ulong entityId, ulong parentId)
        {
            if (_ffi.scene_set_parent == IntPtr.Zero)
            {
                Log.Error("C#", "SceneSetParent函数指针为空");
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneSetParentDelegate>(_ffi.scene_set_parent);
            func(scene, entityId, parentId);
        }
        
        public static ulong SceneGetParent(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_parent == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetParentDelegate>(_ffi.scene_get_parent);
            return func(scene, entityId);
        }
        
        public static ulong SceneGetChildCount(IntPtr scene, ulong parentId)
        {
            if (_ffi.scene_get_child_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetChildCountDelegate>(_ffi.scene_get_child_count);
            return func(scene, parentId);
        }
        
        public static ulong SceneGetChildId(IntPtr scene, ulong parentId, ulong index)
        {
            if (_ffi.scene_get_child_id == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<SceneGetChildIdDelegate>(_ffi.scene_get_child_id);
            return func(scene, parentId, index);
        }

        // === 0.2 per-entity PBR 材质（task-26） ===

        /// <summary>设置/清除实体 PBR 材质（JSON；空串或 "null" 清除）。返回 0 成功。</summary>
        public static int SceneSetEntityPbrMaterial(IntPtr scene, ulong entityId, string json)
        {
            if (_ffi.scene_set_entity_pbr_material_json == IntPtr.Zero)
            {
                return -1;
            }
            IntPtr jsonPtr = Marshal.StringToHGlobalAnsi(json ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<SceneSetEntityPbrMaterialDelegate>(
                    _ffi.scene_set_entity_pbr_material_json);
                return func(scene, entityId, jsonPtr);
            }
            finally
            {
                Marshal.FreeHGlobal(jsonPtr);
            }
        }

        /// <summary>读取实体 PBR 材质 JSON（无材质返回空串）。</summary>
        public static string SceneGetEntityPbrMaterial(IntPtr scene, ulong entityId)
        {
            if (_ffi.scene_get_entity_pbr_material_json == IntPtr.Zero)
            {
                return "";
            }
            const int bufferSize = 1024;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<SceneGetEntityPbrMaterialDelegate>(
                    _ffi.scene_get_entity_pbr_material_json);
                ulong written = func(scene, entityId, buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        /// <summary>立即同步场景 PBR 材质到 renderer，返回 renderer 材质覆盖数。</summary>
        public static int EditorSyncPbrMaterialsNow()
        {
            if (_ffi.editor_sync_pbr_materials_now == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<EditorSyncPbrMaterialsNowDelegate>(
                _ffi.editor_sync_pbr_materials_now);
            return func();
        }

        // === 0.3 编辑器 Profiler（task-39） ===

        /// <summary>读取 Profiler 快照 JSON（失败返回空串）。</summary>
        public static string ProfilerSnapshotJson()
        {
            if (_ffi.profiler_snapshot_json == IntPtr.Zero)
            {
                return "";
            }
            const int bufferSize = 16384;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<ProfilerSnapshotJsonDelegate>(
                    _ffi.profiler_snapshot_json);
                ulong written = func(buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        /// <summary>重置 Profiler 帧统计（启动耗时保留）；返回 0=成功。</summary>
        public static int ProfilerReset()
        {
            if (_ffi.profiler_reset == IntPtr.Zero)
            {
                return -1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProfilerResetDelegate>(
                _ffi.profiler_reset);
            return func();
        }

        /// <summary>累计记录的帧数。</summary>
        public static int ProfilerFrameCount()
        {
            if (_ffi.profiler_frame_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProfilerFrameCountDelegate>(
                _ffi.profiler_frame_count);
            return func();
        }

        // === 0.3 Build Settings（task-41） ===

        /// <summary>读取当前工程 Build Settings JSON（缺失时返回推断默认值）。</summary>
        public static string BuildSettingsSnapshotJson()
        {
            if (_ffi.build_settings_snapshot_json == IntPtr.Zero)
            {
                return "";
            }
            const int bufferSize = 8192;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<BuildSettingsSnapshotJsonDelegate>(
                    _ffi.build_settings_snapshot_json);
                ulong written = func(buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        /// <summary>保存 Build Settings JSON；0=成功。</summary>
        public static int BuildSettingsSaveJson(string json)
        {
            if (_ffi.build_settings_save_json == IntPtr.Zero)
            {
                return -1;
            }
            IntPtr jsonPtr = Marshal.StringToHGlobalAnsi(json ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<BuildSettingsSaveJsonDelegate>(
                    _ffi.build_settings_save_json);
                return func(jsonPtr);
            }
            finally
            {
                Marshal.FreeHGlobal(jsonPtr);
            }
        }

        /// <summary>校验 Build Settings JSON；返回 0=通过，错误文本写入 errors。</summary>
        public static int BuildSettingsValidateJson(string json, out string errors)
        {
            errors = "";
            if (_ffi.build_settings_validate_json == IntPtr.Zero)
            {
                return -1;
            }
            IntPtr jsonPtr = Marshal.StringToHGlobalAnsi(json ?? "");
            const int bufferSize = 4096;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<BuildSettingsValidateJsonDelegate>(
                    _ffi.build_settings_validate_json);
                int rc = func(jsonPtr, buffer, (ulong)bufferSize);
                errors = Marshal.PtrToStringAnsi(buffer) ?? "";
                return rc;
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(jsonPtr);
            }
        }

        // === 0.3 动画时间轴（task-43） ===

        /// <summary>查询 glTF 资产动画/骨骼摘要 JSON（失败返回含 error 的 JSON）。</summary>
        public static string AnimationAssetSummary(string path)
        {
            if (_ffi.animation_asset_summary == IntPtr.Zero)
            {
                return "";
            }
            IntPtr pathPtr = Marshal.StringToHGlobalAnsi(path ?? "");
            const int bufferSize = 32768;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<AnimationAssetSummaryDelegate>(
                    _ffi.animation_asset_summary);
                ulong written = func(pathPtr, buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(pathPtr);
            }
        }

        /// <summary>每帧驱动预览时间（playing>=0 时设置播放状态），返回当前时间。</summary>
        public static double AnimationPreviewTick(float dt, int playing)
        {
            if (_ffi.animation_preview_tick == IntPtr.Zero)
            {
                return 0.0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewTickDelegate>(
                _ffi.animation_preview_tick);
            return func(dt, playing);
        }

        /// <summary>选择 clip；返回 0=成功。</summary>
        public static int AnimationPreviewSetClip(int index)
        {
            if (_ffi.animation_preview_set_clip == IntPtr.Zero)
            {
                return -1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewSetClipDelegate>(
                _ffi.animation_preview_set_clip);
            return func(index);
        }

        /// <summary>设置预览时间（clamp 到 [0,duration]），返回实际时间。</summary>
        public static double AnimationPreviewSetTime(float time)
        {
            if (_ffi.animation_preview_set_time == IntPtr.Zero)
            {
                return 0.0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewSetTimeDelegate>(
                _ffi.animation_preview_set_time);
            return func(time);
        }

        /// <summary>开始播放。</summary>
        public static int AnimationPreviewPlay()
        {
            if (_ffi.animation_preview_play == IntPtr.Zero)
            {
                return -1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewPlayDelegate>(
                _ffi.animation_preview_play);
            return func();
        }

        /// <summary>暂停播放。</summary>
        public static int AnimationPreviewPause()
        {
            if (_ffi.animation_preview_pause == IntPtr.Zero)
            {
                return -1;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewPauseDelegate>(
                _ffi.animation_preview_pause);
            return func();
        }

        /// <summary>从 glTF 初始化 9 状态图；返回 0 成功。</summary>
        public static int AnimationPreviewInitGraph(string path, int clipIndex)
        {
            if (_ffi.animation_preview_init_graph == IntPtr.Zero) return -1;
            IntPtr pathPtr = Marshal.StringToHGlobalAnsi(path ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewInitGraphDelegate>(
                    _ffi.animation_preview_init_graph);
                return func(pathPtr, clipIndex);
            }
            finally { Marshal.FreeHGlobal(pathPtr); }
        }

        /// <summary>请求切换状态（名称大小写不敏感）；返回 0 成功。</summary>
        public static int AnimationPreviewSetState(string name)
        {
            if (_ffi.animation_preview_set_state == IntPtr.Zero) return -1;
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(name ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewSetStateDelegate>(
                    _ffi.animation_preview_set_state);
                return func(namePtr);
            }
            finally { Marshal.FreeHGlobal(namePtr); }
        }

        /// <summary>状态机快照 JSON。</summary>
        public static string AnimationPreviewStateSnapshot()
        {
            if (_ffi.animation_preview_state_snapshot == IntPtr.Zero) return "";
            const int bufferSize = 8192;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewStateSnapshotDelegate>(
                    _ffi.animation_preview_state_snapshot);
                ulong written = func(buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally { Marshal.FreeHGlobal(buffer); }
        }

        /// <summary>释放状态图；返回 0。</summary>
        public static int AnimationPreviewGraphFree()
        {
            if (_ffi.animation_preview_graph_free == IntPtr.Zero) return -1;
            var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewGraphFreeDelegate>(
                _ffi.animation_preview_graph_free);
            return func();
        }

        /// <summary>预览状态快照 JSON。</summary>
        public static string AnimationPreviewSnapshot()
        {
            if (_ffi.animation_preview_snapshot == IntPtr.Zero)
            {
                return "";
            }
            const int bufferSize = 4096;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<AnimationPreviewSnapshotDelegate>(
                    _ffi.animation_preview_snapshot);
                ulong written = func(buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        // === 0.4 项目模板（task-49） ===

        /// <summary>列出可用项目模板（JSON：count/templates[]）。</summary>
        public static string TemplateListJson()
        {
            if (_ffi.template_list_json == IntPtr.Zero) return "";
            const int bufferSize = 16384;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<TemplateListJsonDelegate>(
                    _ffi.template_list_json);
                ulong written = func(buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        /// <summary>从模板创建工程；0=成功，-1=参数/模板无效，-2=目标非空，-3=IO 失败。</summary>
        public static int TemplateCreate(string templateId, string destDir, string projectName)
        {
            if (_ffi.template_create == IntPtr.Zero) return -1;
            IntPtr idPtr = Marshal.StringToHGlobalAnsi(templateId ?? "");
            IntPtr destPtr = Marshal.StringToHGlobalAnsi(destDir ?? "");
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(projectName ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<TemplateCreateDelegate>(
                    _ffi.template_create);
                return func(idPtr, destPtr, namePtr);
            }
            finally
            {
                Marshal.FreeHGlobal(idPtr);
                Marshal.FreeHGlobal(destPtr);
                Marshal.FreeHGlobal(namePtr);
            }
        }

        // === 0.4 包管理器（task-52） ===

        /// <summary>列出本地 registry 包 + 已安装状态 JSON。</summary>
        public static string PackageRegistryListJson(string registryDir)
        {
            if (_ffi.package_registry_list_json == IntPtr.Zero) return "";
            IntPtr dirPtr = Marshal.StringToHGlobalAnsi(registryDir ?? "");
            const int bufferSize = 32768;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<PackageRegistryListJsonDelegate>(
                    _ffi.package_registry_list_json);
                ulong written = func(dirPtr, buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(dirPtr);
            }
        }

        /// <summary>安装包（含依赖闭包）；返回 0 成功，report 为结果/错误 JSON。</summary>
        public static int PackageInstallJson(string projectRoot, string registryDir, string name, out string report)
        {
            report = "";
            if (_ffi.package_install_json == IntPtr.Zero) return -1;
            IntPtr rootPtr = Marshal.StringToHGlobalAnsi(projectRoot ?? "");
            IntPtr dirPtr = Marshal.StringToHGlobalAnsi(registryDir ?? "");
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(name ?? "");
            const int bufferSize = 16384;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<PackageInstallJsonDelegate>(
                    _ffi.package_install_json);
                int rc = func(rootPtr, dirPtr, namePtr, buffer, (ulong)bufferSize);
                report = Marshal.PtrToStringAnsi(buffer) ?? "";
                return rc;
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(rootPtr);
                Marshal.FreeHGlobal(dirPtr);
                Marshal.FreeHGlobal(namePtr);
            }
        }

        /// <summary>读取已安装列表（lockfile JSON）。</summary>
        public static string PackageInstalledJson(string projectRoot)
        {
            if (_ffi.package_installed_json == IntPtr.Zero) return "";
            IntPtr rootPtr = Marshal.StringToHGlobalAnsi(projectRoot ?? "");
            const int bufferSize = 16384;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<PackageInstalledJsonDelegate>(
                    _ffi.package_installed_json);
                ulong written = func(rootPtr, buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(rootPtr);
            }
        }

        /// <summary>卸载包；0=成功。</summary>
        public static int PackageUninstall(string projectRoot, string name)
        {
            if (_ffi.package_uninstall == IntPtr.Zero) return -1;
            IntPtr rootPtr = Marshal.StringToHGlobalAnsi(projectRoot ?? "");
            IntPtr namePtr = Marshal.StringToHGlobalAnsi(name ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<PackageUninstallDelegate>(
                    _ffi.package_uninstall);
                return func(rootPtr, namePtr);
            }
            finally
            {
                Marshal.FreeHGlobal(rootPtr);
                Marshal.FreeHGlobal(namePtr);
            }
        }

        // === 0.4 声明式插件（task-55） ===

        /// <summary>插件列表 JSON（含启用状态）。</summary>
        public static string PluginListJson(string projectRoot)
        {
            if (_ffi.plugin_list_json == IntPtr.Zero) return "";
            IntPtr rootPtr = Marshal.StringToHGlobalAnsi(projectRoot ?? "");
            const int bufferSize = 16384;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<PluginListJsonDelegate>(_ffi.plugin_list_json);
                ulong written = func(rootPtr, buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(rootPtr);
            }
        }

        /// <summary>启用/禁用插件；0=成功。</summary>
        public static int PluginSetEnabled(string projectRoot, string id, bool enabled)
        {
            if (_ffi.plugin_set_enabled == IntPtr.Zero) return -1;
            IntPtr rootPtr = Marshal.StringToHGlobalAnsi(projectRoot ?? "");
            IntPtr idPtr = Marshal.StringToHGlobalAnsi(id ?? "");
            try
            {
                var func = Marshal.GetDelegateForFunctionPointer<PluginSetEnabledDelegate>(_ffi.plugin_set_enabled);
                return func(rootPtr, idPtr, enabled ? 1 : 0);
            }
            finally
            {
                Marshal.FreeHGlobal(rootPtr);
                Marshal.FreeHGlobal(idPtr);
            }
        }

        /// <summary>启用插件的命令 JSON。</summary>
        public static string PluginCommandsJson(string projectRoot)
        {
            if (_ffi.plugin_commands_json == IntPtr.Zero) return "";
            IntPtr rootPtr = Marshal.StringToHGlobalAnsi(projectRoot ?? "");
            const int bufferSize = 16384;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            try
            {
                Marshal.WriteByte(buffer, 0, 0);
                var func = Marshal.GetDelegateForFunctionPointer<PluginCommandsJsonDelegate>(_ffi.plugin_commands_json);
                ulong written = func(rootPtr, buffer, (ulong)bufferSize);
                return written == 0UL ? "" : (Marshal.PtrToStringAnsi(buffer, (int)written) ?? "");
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
                Marshal.FreeHGlobal(rootPtr);
            }
        }
    }
}



