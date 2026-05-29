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
    }
}