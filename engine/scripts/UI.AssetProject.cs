using System;
using System.Runtime.InteropServices;

namespace Hezhou
{
    public enum AssetType
    {
        EntityTemplate = 0,
        Texture = 1,
        Material = 2,
        Script = 3
    }
    
    public struct AssetInfo
    {
        public ulong Id;
        public string Name;
        public AssetType Type;
        public string Description;
    }
    
    public struct ProjectSettings
    {
        public uint GameWidth;
        public uint GameHeight;
        public uint Fps;
    }
    
    public struct ProjectEntityInfo
    {
        public ulong Id;
        public string Name;
        public float[] Position;
        public float[] Rotation;
        public float[] Scale;
        public string MeshType;
    }
    
    public static class AssetLibrary
    {
        public static int GetCategoryCount()
        {
            return UI.AssetLibraryGetCategoryCount();
        }
        
        public static string GetCategoryName(int index)
        {
            return UI.AssetLibraryGetCategoryName(index);
        }
        
        public static int GetAssetCount(int categoryIndex)
        {
            return UI.AssetLibraryGetAssetCount(categoryIndex);
        }
        
        public static AssetInfo GetAssetInfo(int categoryIndex, int assetIndex)
        {
            return UI.AssetLibraryGetAssetInfo(categoryIndex, assetIndex);
        }
        
        public static ulong CreateEntityFromTemplate(IntPtr scene, ulong templateId)
        {
            return UI.AssetLibraryCreateEntityFromTemplate(scene, templateId);
        }
        
        public static ulong CreateMeshEntity(IntPtr scene, int meshType)
        {
            return UI.AssetLibraryCreateMeshEntity(scene, meshType);
        }
        
        public static ulong LoadTexture(string path)
        {
            return UI.AssetLibraryLoadTexture(path);
        }
        
        public static ulong LoadMesh(string path)
        {
            return UI.AssetLibraryLoadMesh(path);
        }
    }
    
    public static class Project
    {
        public static bool CreateNew(string name, string path)
        {
            return UI.ProjectCreateNew(name, path);
        }
        
        public static bool Load(string path)
        {
            return UI.ProjectLoad(path);
        }
        
        public static bool Save()
        {
            return UI.ProjectSave();
        }
        
        public static string GetName()
        {
            return UI.ProjectGetName();
        }
        
        public static string GetPath()
        {
            return UI.ProjectGetPath();
        }
        
        public static int GetEntityCount()
        {
            return UI.ProjectGetEntityCount();
        }
        
        public static bool IsLoaded()
        {
            return UI.ProjectIsLoaded();
        }
        
        public static void SyncToScene(IntPtr scene)
        {
            UI.ProjectSyncToScene(scene);
        }
        
        public static void SyncFromScene(IntPtr scene)
        {
            UI.ProjectSyncFromScene(scene);
        }
        
        public static ProjectSettings GetSettings()
        {
            return UI.ProjectGetSettings();
        }
        
        public static bool SetSettings(uint width, uint height, uint fps)
        {
            return UI.ProjectSetSettings(width, height, fps);
        }
        
        public static ProjectEntityInfo GetEntityInfo(ulong entityId)
        {
            return UI.ProjectGetEntityInfo(entityId);
        }
        
        public static bool AddEntity(ulong entityId, string name, 
            float posX, float posY, float posZ,
            float rotX, float rotY, float rotZ,
            float scaleX, float scaleY, float scaleZ,
            string meshType)
        {
            return UI.ProjectAddEntity(entityId, name, 
                posX, posY, posZ, 
                rotX, rotY, rotZ,
                scaleX, scaleY, scaleZ, 
                meshType);
        }
        
        public static bool RemoveEntity(ulong entityId)
        {
            return UI.ProjectRemoveEntity(entityId);
        }
    }

    public static partial class UI
    {
        public static int AssetLibraryGetCategoryCount()
        {
            if (_ffi.asset_library_get_category_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetCategoryCountDelegate>(_ffi.asset_library_get_category_count);
            return func();
        }
        
        public static string AssetLibraryGetCategoryName(int index)
        {
            if (_ffi.asset_library_get_category_name == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetCategoryNameDelegate>(_ffi.asset_library_get_category_name);
            bool success = func(index, buffer, 256);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static int AssetLibraryGetAssetCount(int categoryIndex)
        {
            if (_ffi.asset_library_get_asset_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetAssetCountDelegate>(_ffi.asset_library_get_asset_count);
            return func(categoryIndex);
        }
        
        public static AssetInfo AssetLibraryGetAssetInfo(int categoryIndex, int assetIndex)
        {
            var info = new AssetInfo();
            if (_ffi.asset_library_get_asset_info == IntPtr.Zero)
            {
                return info;
            }
            
            IntPtr nameBuffer = Marshal.AllocHGlobal(256);
            IntPtr descBuffer = Marshal.AllocHGlobal(256);
            
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryGetAssetInfoDelegate>(_ffi.asset_library_get_asset_info);
            bool success = func(categoryIndex, assetIndex, out info.Id, nameBuffer, 256, out uint typeValue, descBuffer, 256);
            
            if (success)
            {
                info.Name = Marshal.PtrToStringAnsi(nameBuffer) ?? "";
                info.Type = (AssetType)typeValue;
                info.Description = Marshal.PtrToStringAnsi(descBuffer) ?? "";
            }
            
            Marshal.FreeHGlobal(nameBuffer);
            Marshal.FreeHGlobal(descBuffer);
            
            return info;
        }
        
        public static ulong AssetLibraryCreateEntityFromTemplate(IntPtr scene, ulong templateId)
        {
            if (_ffi.asset_library_create_entity_from_template == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryCreateEntityFromTemplateDelegate>(_ffi.asset_library_create_entity_from_template);
            return func(scene, templateId);
        }
        
        public static ulong AssetLibraryCreateMeshEntity(IntPtr scene, int meshType)
        {
            if (_ffi.asset_library_create_mesh_entity == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryCreateMeshEntityDelegate>(_ffi.asset_library_create_mesh_entity);
            return func(scene, (uint)meshType);
        }

        public static ulong AssetLibraryLoadTexture(string path)
        {
            if (_ffi.asset_library_load_texture == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryLoadTextureDelegate>(_ffi.asset_library_load_texture);
            return func(path);
        }

        public static ulong AssetLibraryLoadMesh(string path)
        {
            if (_ffi.asset_library_load_mesh == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<AssetLibraryLoadMeshDelegate>(_ffi.asset_library_load_mesh);
            return func(path);
        }

        public static bool ProjectCreateNew(string name, string path)
        {
            if (_ffi.project_create_new == IntPtr.Zero)
            {
                Log.Error("C#", "ProjectCreateNew函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectCreateNewDelegate>(_ffi.project_create_new);
            return func(name, path);
        }
        
        public static bool ProjectLoad(string path)
        {
            if (_ffi.project_load == IntPtr.Zero)
            {
                Log.Error("C#", "ProjectLoad函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectLoadDelegate>(_ffi.project_load);
            return func(path);
        }
        
        public static bool ProjectSave()
        {
            if (_ffi.project_save == IntPtr.Zero)
            {
                Log.Error("C#", "ProjectSave函数指针为空");
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSaveDelegate>(_ffi.project_save);
            return func();
        }
        
        public static string ProjectGetName()
        {
            if (_ffi.project_get_name == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(256);
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetNameDelegate>(_ffi.project_get_name);
            bool success = func(buffer, 256);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static string ProjectGetPath()
        {
            if (_ffi.project_get_path == IntPtr.Zero)
            {
                return "";
            }
            IntPtr buffer = Marshal.AllocHGlobal(512);
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetPathDelegate>(_ffi.project_get_path);
            bool success = func(buffer, 512);
            string result = success ? Marshal.PtrToStringAnsi(buffer) ?? "" : "";
            Marshal.FreeHGlobal(buffer);
            return result;
        }
        
        public static int ProjectGetEntityCount()
        {
            if (_ffi.project_get_entity_count == IntPtr.Zero)
            {
                return 0;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetEntityCountDelegate>(_ffi.project_get_entity_count);
            return func();
        }
        
        public static bool ProjectIsLoaded()
        {
            if (_ffi.project_is_loaded == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectIsLoadedDelegate>(_ffi.project_is_loaded);
            return func();
        }
        
        public static void ProjectSyncToScene(IntPtr scene)
        {
            if (_ffi.project_sync_to_scene == IntPtr.Zero)
            {
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSyncToSceneDelegate>(_ffi.project_sync_to_scene);
            func(scene);
        }
        
        public static void ProjectSyncFromScene(IntPtr scene)
        {
            if (_ffi.project_sync_from_scene == IntPtr.Zero)
            {
                return;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSyncFromSceneDelegate>(_ffi.project_sync_from_scene);
            func(scene);
        }
        
        public static ProjectSettings ProjectGetSettings()
        {
            var settings = new ProjectSettings();
            if (_ffi.project_get_settings == IntPtr.Zero)
            {
                return settings;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetSettingsDelegate>(_ffi.project_get_settings);
            func(out settings.GameWidth, out settings.GameHeight, out settings.Fps);
            return settings;
        }
        
        public static bool ProjectSetSettings(uint width, uint height, uint fps)
        {
            if (_ffi.project_set_settings == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectSetSettingsDelegate>(_ffi.project_set_settings);
            return func(width, height, fps);
        }
        
        public static ProjectEntityInfo ProjectGetEntityInfo(ulong entityId)
        {
            var info = new ProjectEntityInfo();
            if (_ffi.project_get_entity_info == IntPtr.Zero)
            {
                return info;
            }
            
            IntPtr nameBuffer = Marshal.AllocHGlobal(256);
            IntPtr posBuffer = Marshal.AllocHGlobal(12);
            IntPtr rotBuffer = Marshal.AllocHGlobal(12);
            IntPtr scaleBuffer = Marshal.AllocHGlobal(12);
            IntPtr meshTypeBuffer = Marshal.AllocHGlobal(64);
            
            var func = Marshal.GetDelegateForFunctionPointer<ProjectGetEntityInfoDelegate>(_ffi.project_get_entity_info);
            bool success = func(entityId, nameBuffer, 256, posBuffer, rotBuffer, scaleBuffer, meshTypeBuffer, 64);
            
            if (success)
            {
                info.Id = entityId;
                info.Name = Marshal.PtrToStringAnsi(nameBuffer) ?? "";
                info.MeshType = Marshal.PtrToStringAnsi(meshTypeBuffer) ?? "";
                
                info.Position = new float[3];
                info.Rotation = new float[3];
                info.Scale = new float[3];
                
                Marshal.Copy(posBuffer, info.Position, 0, 3);
                Marshal.Copy(rotBuffer, info.Rotation, 0, 3);
                Marshal.Copy(scaleBuffer, info.Scale, 0, 3);
            }
            
            Marshal.FreeHGlobal(nameBuffer);
            Marshal.FreeHGlobal(posBuffer);
            Marshal.FreeHGlobal(rotBuffer);
            Marshal.FreeHGlobal(scaleBuffer);
            Marshal.FreeHGlobal(meshTypeBuffer);
            
            return info;
        }
        
        public static bool ProjectAddEntity(ulong entityId, string name,
            float posX, float posY, float posZ,
            float rotX, float rotY, float rotZ,
            float scaleX, float scaleY, float scaleZ,
            string meshType)
        {
            if (_ffi.project_add_entity == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectAddEntityDelegate>(_ffi.project_add_entity);
            return func(entityId, name, posX, posY, posZ, rotX, rotY, rotZ, scaleX, scaleY, scaleZ, meshType);
        }
        
        public static bool ProjectRemoveEntity(ulong entityId)
        {
            if (_ffi.project_remove_entity == IntPtr.Zero)
            {
                return false;
            }
            var func = Marshal.GetDelegateForFunctionPointer<ProjectRemoveEntityDelegate>(_ffi.project_remove_entity);
            return func(entityId);
        }
    }
}