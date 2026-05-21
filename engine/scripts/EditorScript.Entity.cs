using System;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static void OnCreateEntityClick(ulong widgetId)
        {
            Log.Info("Editor", "点击\"创建Entity\"按钮");
            
            if (_gameScene == null)
            {
                Log.Error("Editor", "Scene未创建!");
                return;
            }
            
            ulong entityId = _gameScene.CreateEntity();
            if (entityId != 0)
            {
                Log.Info("Editor", $"创建新Entity: id={entityId}");
                _statusItem.Text = $"创建Entity: {entityId}";
                
                string name = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
                AddEntityToTree(entityId, name);
                SelectEntity(entityId);
            }
            else
            {
                Log.Error("Editor", "创建Entity失败!");
            }
        }
        
        private static void SelectEntity(ulong entityId)
        {
            if (_gameScene == null) return;
            _gameScene.SelectEntity(entityId);
            _selectedEntityId = entityId;
            UpdatePropertiesPanel(entityId);
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeViewSetSelected(_projectTreeViewId, nodeId);
            }
        }

        private static void AddEntityToTree(ulong entityId, string name)
        {
            if (_projectTreeViewId != 0 && _entitiesNodeId != 0)
            {
                ulong nodeId = UI.TreeViewAddNode(_projectTreeViewId, _entitiesNodeId, name, entityId, false);
                _entityNodeMap[entityId] = nodeId;
                Log.Info("Editor", $"添加Entity到树: entityId={entityId}, name={name}");
            }
        }
        
        private static void UpdateEntityNameInTree(ulong entityId, string newName)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeNodeSetText(nodeId, newName);
                Log.Info("Editor", $"更新Entity名称: entityId={entityId}, name={newName}");
            }
        }
        
        private static void RemoveEntityFromTree(ulong entityId)
        {
            if (_entityNodeMap.TryGetValue(entityId, out var nodeId))
            {
                UI.TreeViewRemoveNode(_projectTreeViewId, nodeId);
                _entityNodeMap.Remove(entityId);
                Log.Info("Editor", $"从树移除Entity: entityId={entityId}");
            }
        }
        
        private static void CreateProjectStructureTree()
        {
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            _projectTreeViewId = UI.CreateTreeView(_projectPanel.Id, 10, 40, LEFT_PANEL_WIDTH - 20, mainHeight - 50);
            
            _assetsNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "Assets", 0, true);
            _scenesNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "Scenes", 0, true);
            _scriptsNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "Scripts", 0, true);
            _entitiesNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "Entities", 0, true);
            
            foreach (var script in _availableScripts)
            {
                UI.TreeViewAddNode(_projectTreeViewId, _scriptsNodeId, script, 0, false);
            }
            
            if (_gameScene != null)
            {
                int entityCount = _gameScene.GetEntityCount();
                for (int i = 0; i < entityCount; i++)
                {
                    ulong entityId = _gameScene.GetEntityId(i);
                    string name = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
                    ulong nodeId = UI.TreeViewAddNode(_projectTreeViewId, _entitiesNodeId, name, entityId, false);
                    _entityNodeMap[entityId] = nodeId;
                }
            }
            
            UI.TreeViewSetOnSelect(_projectTreeViewId, _treeNodeSelectCallback);
            UI.TreeViewExpandNode(_projectTreeViewId, _assetsNodeId);
            UI.TreeViewExpandNode(_projectTreeViewId, _scenesNodeId);
            UI.TreeViewExpandNode(_projectTreeViewId, _scriptsNodeId);
            UI.TreeViewExpandNode(_projectTreeViewId, _entitiesNodeId);
            
            Log.Info("Editor", $"项目结构树创建完成: entities={_entityNodeMap.Count}");
        }
        
        private static void OnTreeNodeSelect(ulong widgetId, ulong userData)
        {
            if (userData != 0)
            {
                SelectEntity(userData);
                Log.Info("Editor", $"TreeView选中Entity: entityId={userData}");
            }
        }

        private static void CreateAssetGridView()
        {
            _assetGridViewId = UI.CreateGridView(_assetPanel.Id, 10, 40, LEFT_PANEL_WIDTH - 20, BOTTOM_PANEL_HEIGHT - 50, 64);
            RefreshAssetGridView();
            UI.GridViewSetOnClick(_assetGridViewId, _gridViewClickCallback);
            Log.Info("Editor", "资产GridView创建完成");
        }
        
        private static void RefreshAssetGridView()
        {
            UI.GridViewClear(_assetGridViewId);
            
            UI.GridViewAddItem(_assetGridViewId, "Cube", 1);
            UI.GridViewAddItem(_assetGridViewId, "Sphere", 2);
            UI.GridViewAddItem(_assetGridViewId, "Plane", 3);
            UI.GridViewAddItem(_assetGridViewId, "Cylinder", 4);
            
            Log.Info("Editor", $"资产GridView刷新完成: {UI.GridViewItemCount(_assetGridViewId)}项");
        }
        
        private static void OnGridViewClick(ulong widgetId, int index, ulong userData)
        {
            if (_gameScene != null && userData != 0)
            {
                ulong entityId = _gameScene.CreateEntity();
                if (entityId != 0)
                {
                    string name = $"Asset_{userData}";
                    UI.SceneSetEntityName(_gameScene.ScenePtr, entityId, name);
                    AddEntityToTree(entityId, name);
                    SelectEntity(entityId);
                    _statusItem.Text = $"从资产库创建Entity: {entityId}";
                    Log.Info("Editor", $"从资产GridView创建Entity: index={index}, userData={userData}, entityId={entityId}");
                }
            }
        }
    }
}