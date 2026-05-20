using System;
using Hezhou;

public class AssetProjectTest
{
    public static void RunTests()
    {
        Log.Info("AssetProjectTest", "=== 系统资产库测试 ===");
        
        int categoryCount = AssetLibrary.GetCategoryCount();
        Log.Info("AssetProjectTest", $"分类数量: {categoryCount}");
        
        for (int i = 0; i < categoryCount; i++)
        {
            string categoryName = AssetLibrary.GetCategoryName(i);
            int assetCount = AssetLibrary.GetAssetCount(i);
            Log.Info("AssetProjectTest", $"分类 [{categoryName}]: {assetCount} 个资产");
            
            for (int j = 0; j < assetCount; j++)
            {
                var asset = AssetLibrary.GetAssetInfo(i, j);
                Log.Info("AssetProjectTest", $"  - [{asset.Id}] {asset.Name} ({asset.Type})");
                if (!string.IsNullOrEmpty(asset.Description))
                {
                    Log.Info("AssetProjectTest", $"    描述: {asset.Description}");
                }
            }
        }
        
        Log.Info("AssetProjectTest", "");
        Log.Info("AssetProjectTest", "=== 项目文件系统测试 ===");
        
        string testPath = System.IO.Path.Combine(
            System.IO.Path.GetTempPath(), 
            "HezhouTestProject"
        );
        
        Log.Info("AssetProjectTest", $"测试路径: {testPath}");
        
        bool created = Project.CreateNew("TestProject", testPath);
        Log.Info("AssetProjectTest", $"创建项目: {created}");
        
        if (created)
        {
            string projectName = Project.GetName();
            string projectPath = Project.GetPath();
            bool isLoaded = Project.IsLoaded();
            int entityCount = Project.GetEntityCount();
            
            Log.Info("AssetProjectTest", $"项目名称: {projectName}");
            Log.Info("AssetProjectTest", $"项目路径: {projectPath}");
            Log.Info("AssetProjectTest", $"是否加载: {isLoaded}");
            Log.Info("AssetProjectTest", $"Entity数量: {entityCount}");
            
            var settings = Project.GetSettings();
            Log.Info("AssetProjectTest", $"设置: {settings.GameWidth}x{settings.GameHeight} @ {settings.Fps}fps");
            
            bool saved = Project.Save();
            Log.Info("AssetProjectTest", $"保存项目: {saved}");
            
            Log.Info("AssetProjectTest", "测试完成 - 请检查项目文件夹结构");
        }
        
        Log.Info("AssetProjectTest", "=== 测试结束 ===");
    }
}