using System;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static void CreateEditorLayout()
        {
            _gameScene = new Scene();
            Log.Info("Editor", $"Scene created: ptr={_gameScene.ScenePtr}");
            
            _testCubeId = _gameScene.CreateCube();
            Log.Info("Editor", $"Test cube created: entityId={_testCubeId}");
            
            _gameScene.SetGameState(GameState.Editing);
            Log.Info("Editor", $"Scene state: {_gameScene.GetGameState()}");
            
            float toolbarY = 0f;
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float bottomY = _screenHeight - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float statusY = _screenHeight - STATUS_BAR_HEIGHT;
            
            float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH;
            float previewX = LEFT_PANEL_WIDTH;

            ulong rootId = UI.GetRootId();
            Log.Info("Editor", $"RootId={rootId}");

            _toolbar = new Panel(rootId, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT, 0.15f, 0.15f, 0.15f, 1.0f);
            _toolbarButtons = new HStack(_toolbar.Id, 10f);
            _toolbarButtons.SetPosition(10f, 5f);
            
            CreateToolbarMenus();
            
            var newBtn = _toolbarButtons.AddButton(100f, 30f, "新建");
            newBtn.SetOnClick((id) => {
                UI.PopupMenuShow(_fileMenuId, 10f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
            });
            
            var openBtn = _toolbarButtons.AddButton(100f, 30f, "打开");
            openBtn.SetOnClick((id) => {
                UI.PopupMenuShow(_openMenuId, 110f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
            });
            
            var saveBtn = _toolbarButtons.AddButton(100f, 30f, "保存");
            saveBtn.SetOnClick((id) => {
                UI.PopupMenuShow(_saveMenuId, 210f * _contentScale, TOOLBAR_HEIGHT * _contentScale);
            });
            
            var runBtn = _toolbarButtons.AddButton(100f, 30f, "运行");
            _runButtonId = runBtn.Id;
            runBtn.SetOnClick(_runClickCallback);
            
            _toggleEditorBtn = new Button(_toolbar.Id, 100f, 30f, "编辑器");
            UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 120f, 5f, 100f, 30f);
            _toggleEditorBtn.SetOnClick(_toggleEditorClickCallback);
            
            Log.Info("Editor", "工具栏创建完成");

            _projectPanel = new Panel(rootId, 0, mainY, LEFT_PANEL_WIDTH, mainHeight, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_projectPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "项目结构");
            CreateProjectStructureTree();
            Log.Info("Editor", "项目结构面板创建完成 (TreeView)");

            _assetPanel = new Panel(rootId, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_assetPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "资产管理");
            CreateAssetGridView();
            Log.Info("Editor", "资产管理面板创建完成 (GridView)");

            _previewPanel = new Panel(rootId, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT, 0.08f, 0.08f, 0.08f, 0.3f);
            UI.CreateLabel(_previewPanel.Id, 10f, 10f, previewWidth - 20f, 25f, "游戏预览");
            
            float previewWindowWidth = previewWidth - 20f;
            float previewWindowHeight = mainHeight - 20f;
            _previewWindowId = UI.CreatePreviewWindow(_previewPanel.Id, 10f, 40f, previewWindowWidth, previewWindowHeight, 1);
            UI.SetWidgetLayer(_previewWindowId, 0);
            
            UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
            Log.Info("Editor", $"游戏预览面板创建完成: PreviewWindow={previewWindowWidth}x{previewWindowHeight}, aspect={previewWindowWidth/previewWindowHeight:F2}");

            _nameInputCallback = OnNameInputChange;
            _posXInputCallback = OnPosXInputChange;
            _posYInputCallback = OnPosYInputChange;
            _posZInputCallback = OnPosZInputChange;
            _rotXInputCallback = OnRotXInputChange;
            _rotYInputCallback = OnRotYInputChange;
            _rotZInputCallback = OnRotZInputChange;
            _scaleXInputCallback = OnScaleXInputChange;
            _scaleYInputCallback = OnScaleYInputChange;
            _scaleZInputCallback = OnScaleZInputChange;
            
            _propertiesPanel = new Panel(rootId, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_propertiesPanel.Id, 10f, 10f, RIGHT_PANEL_WIDTH - 20f, 25f, "属性编辑");
            
            float tabWidgetY = 40f;
            float tabWidgetHeight = mainHeight + BOTTOM_PANEL_HEIGHT - tabWidgetY - 10f;
            
            _propsTabWidget = new TabWidget(_propertiesPanel.Id, 10f, tabWidgetY, RIGHT_PANEL_WIDTH - 20f, tabWidgetHeight);
            
            _transformTabContentId = UI.CreateVStack(_propsTabWidget.Id, 5f);
            UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Entity:");
            _nameInputFieldId = UI.CreateInputField(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 25f);
            UI.InputFieldSetPlaceholder(_nameInputFieldId, "Entity Name");
            UI.InputFieldSetOnChange(_nameInputFieldId, _nameInputCallback);
            
            UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Position:");
            var posHStack = UI.CreateHStack(_transformTabContentId, 5f);
            _posXInputFieldId = UI.CreateInputField(posHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_posXInputFieldId, "X");
            UI.InputFieldSetOnChange(_posXInputFieldId, _posXInputCallback);
            _posYInputFieldId = UI.CreateInputField(posHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_posYInputFieldId, "Y");
            UI.InputFieldSetOnChange(_posYInputFieldId, _posYInputCallback);
            _posZInputFieldId = UI.CreateInputField(posHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_posZInputFieldId, "Z");
            UI.InputFieldSetOnChange(_posZInputFieldId, _posZInputCallback);
            
            UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Rotation:");
            var rotHStack = UI.CreateHStack(_transformTabContentId, 5f);
            _rotXInputFieldId = UI.CreateInputField(rotHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_rotXInputFieldId, "X");
            UI.InputFieldSetOnChange(_rotXInputFieldId, _rotXInputCallback);
            _rotYInputFieldId = UI.CreateInputField(rotHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_rotYInputFieldId, "Y");
            UI.InputFieldSetOnChange(_rotYInputFieldId, _rotYInputCallback);
            _rotZInputFieldId = UI.CreateInputField(rotHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_rotZInputFieldId, "Z");
            UI.InputFieldSetOnChange(_rotZInputFieldId, _rotZInputCallback);
            
            UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Scale:");
            var scaleHStack = UI.CreateHStack(_transformTabContentId, 5f);
            _scaleXInputFieldId = UI.CreateInputField(scaleHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_scaleXInputFieldId, "X");
            UI.InputFieldSetOnChange(_scaleXInputFieldId, _scaleXInputCallback);
            _scaleYInputFieldId = UI.CreateInputField(scaleHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_scaleYInputFieldId, "Y");
            UI.InputFieldSetOnChange(_scaleYInputFieldId, _scaleYInputCallback);
            _scaleZInputFieldId = UI.CreateInputField(scaleHStack, 70f, 25f);
            UI.InputFieldSetPlaceholder(_scaleZInputFieldId, "Z");
            UI.InputFieldSetOnChange(_scaleZInputFieldId, _scaleZInputCallback);
            
            _scriptsTabContentId = UI.CreateVStack(_propsTabWidget.Id, 5f);
            
            var scriptHStack = UI.CreateHStack(_scriptsTabContentId, 5f);
            _scriptDropdownId = UI.CreateDropdown(scriptHStack, RIGHT_PANEL_WIDTH - 70f, 25f);
            string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
            UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
            UI.DropdownSetOnSelect(_scriptDropdownId, _scriptDropdownSelectCallback);
            
            _addScriptBtnId = UI.CreateButton(scriptHStack, 50f, 25f, "Add");
            UI.SetOnClick(_addScriptBtnId, _addScriptClickCallback);
            
            _scriptsListContainerId = UI.CreateVStack(_scriptsTabContentId, 5f);
            
            _propsTabWidget.AddTab("Transform", _transformTabContentId, false);
            _propsTabWidget.AddTab("Scripts", _scriptsTabContentId, false);
            _propsTabWidget.SetOnSelect(_tabSelectCallback);
            
            Log.Info("Editor", "属性面板创建完成 (TabWidget)");

            _statusBar = new Panel(rootId, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT, 0.12f, 0.12f, 0.12f, 1.0f);
            _statusItems = new List(_statusBar.Id, 0f, true);
            UI.SetWidgetLayout(_statusItems.Id, 10f * _contentScale, 0f, _screenWidth - 20f * _contentScale, STATUS_BAR_HEIGHT);
            
            float fontSize = 14f * _contentScale;
            float itemHeight = STATUS_BAR_HEIGHT - 4f * _contentScale;
            
            _fpsItem = _statusItems.AddItem("FPS: 0", false);
            UI.SetWidgetLayout(_fpsItem.Id, 0f, 2f * _contentScale, 120f * _contentScale, itemHeight);
            UI.SetListItemFontSize(_fpsItem.Id, fontSize);
            
            _statusItem = _statusItems.AddItem("状态: 就绪", true);
            UI.SetWidgetLayout(_statusItem.Id, 130f * _contentScale, 2f * _contentScale, 150f * _contentScale, itemHeight);
            UI.SetListItemFontSize(_statusItem.Id, fontSize);
            
            _projectItem = _statusItems.AddItem("项目: 未命名", true);
            UI.SetWidgetLayout(_projectItem.Id, 290f * _contentScale, 2f * _contentScale, 150f * _contentScale, itemHeight);
            UI.SetListItemFontSize(_projectItem.Id, fontSize);
            
            Log.Info("Editor", "状态栏创建完成");
        }

        private static void OnResize(float width, float height)
        {
            _screenWidth = width;
            _screenHeight = height;
            Log.Info("Editor", $"窗口resize: {width}x{height}");
            
            UpdateLayout();
        }

        private static void UpdateLayout()
        {
            float toolbarY = 0f;
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float bottomY = _screenHeight - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float statusY = _screenHeight - STATUS_BAR_HEIGHT;
            
            float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH;
            float previewX = LEFT_PANEL_WIDTH;

            if (_toolbar != null)
                UI.SetWidgetLayout(_toolbar.Id, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT);
            if (_toggleEditorBtn != null)
                UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 100f, 5f, 80f, 30f);
            if (_projectPanel != null)
                UI.SetWidgetLayout(_projectPanel.Id, 0, mainY, LEFT_PANEL_WIDTH, mainHeight);
            if (_assetPanel != null)
                UI.SetWidgetLayout(_assetPanel.Id, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT);
            if (_previewPanel != null)
                UI.SetWidgetLayout(_previewPanel.Id, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT);
            
            if (_previewWindowId != 0 && _previewPanel != null && !_scriptEditorVisible)
            {
                float previewWindowWidth = previewWidth - 20f;
                float previewWindowHeight = mainHeight - 20f;
                UI.SetWidgetLayout(_previewWindowId, 10f, 40f, previewWindowWidth, previewWindowHeight);
                UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
                Log.Info("Editor", $"PreviewWindow resize: {previewWindowWidth}x{previewWindowHeight}");
            }
            
            if (_propertiesPanel != null)
                UI.SetWidgetLayout(_propertiesPanel.Id, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT);
            if (_statusBar != null)
                UI.SetWidgetLayout(_statusBar.Id, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT);
            
            if (_scriptEditorPanel != null && _scriptEditorVisible)
            {
                float editorWidth = _screenWidth - LEFT_PANEL_WIDTH;
                float editorHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
                UI.SetWidgetLayout(_scriptEditorPanel.Id, LEFT_PANEL_WIDTH, TOOLBAR_HEIGHT, editorWidth, editorHeight);
                if (_scriptTextEditId != 0)
                    UI.SetWidgetLayout(_scriptTextEditId, 10f, 50f, editorWidth - 20f, editorHeight - 50f);
            }
            
            Log.Info("Editor", "布局更新完成");
        }

        private static void ShowMainLayout()
        {
            Log.Info("Editor", "显示主界面...");
            
            ulong rootId = UI.GetRootId();
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float bottomY = _screenHeight - STATUS_BAR_HEIGHT - BOTTOM_PANEL_HEIGHT;
            float previewWidth = _screenWidth - LEFT_PANEL_WIDTH - RIGHT_PANEL_WIDTH;
            float previewX = LEFT_PANEL_WIDTH;
            
            if (_projectPanel != null)
            {
                if (_projectTreeViewId != 0)
                {
                    UI.RemoveWidget(_projectTreeViewId);
                }
                _entityNodeMap.Clear();
                CreateProjectStructureTree();
            }
            
            if (_previewPanel == null)
            {
                _previewPanel = new Panel(rootId, previewX, mainY, previewWidth, mainHeight + BOTTOM_PANEL_HEIGHT, 0.08f, 0.08f, 0.08f, 0.3f);
                UI.CreateLabel(_previewPanel.Id, 10f, 10f, previewWidth - 20f, 25f, "游戏预览");
                
                float previewWindowWidth = previewWidth - 20f;
                float previewWindowHeight = mainHeight - 20f;
                _previewWindowId = UI.CreatePreviewWindow(_previewPanel.Id, 10f, 40f, previewWindowWidth, previewWindowHeight, 1);
                UI.SetWidgetLayer(_previewWindowId, 0);
                UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
                Log.Info("Editor", $"PreviewWindow创建完成 (ShowMainLayout): {previewWindowWidth}x{previewWindowHeight}");
            }
            else
            {
                float previewWindowWidth = previewWidth - 20f;
                float previewWindowHeight = mainHeight - 20f;
                UI.SetGamePreviewExtent((uint)previewWindowWidth, (uint)previewWindowHeight);
            }
            
            if (_assetPanel == null)
            {
                _assetPanel = new Panel(rootId, 0, bottomY, LEFT_PANEL_WIDTH, BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
                UI.CreateLabel(_assetPanel.Id, 10f, 10f, LEFT_PANEL_WIDTH - 20f, 25f, "资产管理");
                CreateAssetGridView();
                Log.Info("Editor", "assetPanel重建完成 (GridView)");
            }
            
            if (_propertiesPanel == null)
            {
                _propertiesPanel = new Panel(rootId, _screenWidth - RIGHT_PANEL_WIDTH, mainY, RIGHT_PANEL_WIDTH, mainHeight + BOTTOM_PANEL_HEIGHT, 0.2f, 0.2f, 0.2f, 1.0f);
                UI.CreateLabel(_propertiesPanel.Id, 10f, 10f, RIGHT_PANEL_WIDTH - 20f, 25f, "属性编辑");
                
                float tabWidgetY = 40f;
                float tabWidgetHeight = mainHeight + BOTTOM_PANEL_HEIGHT - tabWidgetY - 10f;
                
                _propsTabWidget = new TabWidget(_propertiesPanel.Id, 10f, tabWidgetY, RIGHT_PANEL_WIDTH - 20f, tabWidgetHeight);
                
                _transformTabContentId = UI.CreateVStack(_propsTabWidget.Id, 5f);
                UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Entity:");
                _nameInputFieldId = UI.CreateInputField(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 25f);
                UI.InputFieldSetPlaceholder(_nameInputFieldId, "Entity Name");
                UI.InputFieldSetOnChange(_nameInputFieldId, _nameInputCallback);
                
                UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Position:");
                var posHStack = UI.CreateHStack(_transformTabContentId, 5f);
                _posXInputFieldId = UI.CreateInputField(posHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_posXInputFieldId, "X");
                UI.InputFieldSetOnChange(_posXInputFieldId, _posXInputCallback);
                _posYInputFieldId = UI.CreateInputField(posHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_posYInputFieldId, "Y");
                UI.InputFieldSetOnChange(_posYInputFieldId, _posYInputCallback);
                _posZInputFieldId = UI.CreateInputField(posHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_posZInputFieldId, "Z");
                UI.InputFieldSetOnChange(_posZInputFieldId, _posZInputCallback);
                
                UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Rotation:");
                var rotHStack = UI.CreateHStack(_transformTabContentId, 5f);
                _rotXInputFieldId = UI.CreateInputField(rotHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_rotXInputFieldId, "X");
                UI.InputFieldSetOnChange(_rotXInputFieldId, _rotXInputCallback);
                _rotYInputFieldId = UI.CreateInputField(rotHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_rotYInputFieldId, "Y");
                UI.InputFieldSetOnChange(_rotYInputFieldId, _rotYInputCallback);
                _rotZInputFieldId = UI.CreateInputField(rotHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_rotZInputFieldId, "Z");
                UI.InputFieldSetOnChange(_rotZInputFieldId, _rotZInputCallback);
                
                UI.CreateLabel(_transformTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "Scale:");
                var scaleHStack = UI.CreateHStack(_transformTabContentId, 5f);
                _scaleXInputFieldId = UI.CreateInputField(scaleHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_scaleXInputFieldId, "X");
                UI.InputFieldSetOnChange(_scaleXInputFieldId, _scaleXInputCallback);
                _scaleYInputFieldId = UI.CreateInputField(scaleHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_scaleYInputFieldId, "Y");
                UI.InputFieldSetOnChange(_scaleYInputFieldId, _scaleYInputCallback);
                _scaleZInputFieldId = UI.CreateInputField(scaleHStack, 70f, 25f);
                UI.InputFieldSetPlaceholder(_scaleZInputFieldId, "Z");
                UI.InputFieldSetOnChange(_scaleZInputFieldId, _scaleZInputCallback);
                
                _scriptsTabContentId = UI.CreateVStack(_propsTabWidget.Id, 5f);
                
                var scriptHStack = UI.CreateHStack(_scriptsTabContentId, 5f);
                _scriptDropdownId = UI.CreateDropdown(scriptHStack, RIGHT_PANEL_WIDTH - 70f, 25f);
                string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
                UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
                UI.DropdownSetOnSelect(_scriptDropdownId, _scriptDropdownSelectCallback);
                
                _addScriptBtnId = UI.CreateButton(scriptHStack, 50f, 25f, "Add");
                UI.SetOnClick(_addScriptBtnId, _addScriptClickCallback);
                
                _scriptsListContainerId = UI.CreateVStack(_scriptsTabContentId, 5f);
                
                _propsTabWidget.AddTab("Transform", _transformTabContentId, false);
                _propsTabWidget.AddTab("Scripts", _scriptsTabContentId, false);
                _propsTabWidget.SetOnSelect(_tabSelectCallback);
                
                Log.Info("Editor", "propertiesPanel重建完成 (TabWidget + InputFields)");
            }
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "编辑器";
            }
            
            if (_selectedEntityId != 0 && _gameScene != null)
            {
                UpdatePropertiesPanel(_selectedEntityId);
            }
            
            Log.Info("Editor", "主界面显示完成");
        }
        
        private static void HideMainLayout()
        {
            Log.Info("Editor", "隐藏主界面...");
            
            if (_previewPanel != null)
            {
                UI.RemoveWidget(_previewPanel.Id);
                _previewPanel = null;
            }
            
            if (_assetPanel != null)
            {
                UI.RemoveWidget(_assetPanel.Id);
                _assetPanel = null;
            }
            
            if (_propertiesPanel != null)
            {
                UI.RemoveWidget(_propertiesPanel.Id);
                _propertiesPanel = null;
            }
            
            if (_projectPanel != null && _projectTreeViewId != 0)
            {
                UI.RemoveWidget(_projectTreeViewId);
                _projectTreeViewId = 0;
                _entityNodeMap.Clear();
            }
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "预览";
            }
            
            Log.Info("Editor", "主界面隐藏完成");
        }

        private static void ShowScriptEditor()
        {
            if (_scriptEditorVisible) return;
            
            Log.Info("Editor", "ShowScriptEditor开始...");
            
            if (_previewPanel != null)
            {
                Log.Info("Editor", "移除previewPanel...");
                UI.RemoveWidget(_previewPanel.Id);
                _previewPanel = null;
            }
            if (_assetPanel != null)
            {
                Log.Info("Editor", "移除assetPanel...");
                UI.RemoveWidget(_assetPanel.Id);
                _assetPanel = null;
            }
            if (_propertiesPanel != null)
            {
                Log.Info("Editor", "移除propertiesPanel...");
                UI.RemoveWidget(_propertiesPanel.Id);
                _propertiesPanel = null;
            }
            
            ulong rootId = UI.GetRootId();
            
            float leftWidth = LEFT_PANEL_WIDTH;
            float editorX = LEFT_PANEL_WIDTH;
            float editorY = TOOLBAR_HEIGHT;
            float editorWidth = _screenWidth - LEFT_PANEL_WIDTH;
            float editorHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
            
            RefreshDirectoryTree();
            Log.Info("Editor", "左侧目录树已刷新");
            
            _scriptEditorPanel = new Panel(rootId, editorX, editorY, editorWidth, editorHeight, 0.12f, 0.12f, 0.14f, 1.0f);
            
            var hotReloadBtn = new Button(_scriptEditorPanel.Id, 100f, 30f, "Hot Reload");
            UI.SetWidgetLayout(hotReloadBtn.Id, 10f, 10f, 100f, 30f);
            hotReloadBtn.SetOnClick(_hotReloadClickCallback);
            
            _scriptEditorLabel = new Label(_scriptEditorPanel.Id, 200f, 25f, "Script Editor - NewScript.cs");
            UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
            
            _scriptTextEditId = UI.CreateTextEdit(_scriptEditorPanel.Id, editorWidth - 20f, editorHeight - 50f);
            UI.SetTextEditShowLineNumbers(_scriptTextEditId, true);
            UI.SetWidgetLayout(_scriptTextEditId, 10f, 50f, editorWidth - 20f, editorHeight - 50f);
            UI.TextEditSetText(_scriptTextEditId, "// NewScript.cs\nusing System;\nusing Hezhou;\n\npublic class NewScript\n{\n    public void Start()\n    {\n        Console.WriteLine(\"NewScript started!\");\n    }\n    \n    public void Update(float deltaTime)\n    {\n        // Update logic here\n    }\n}");
            
            _scriptEditorVisible = true;
            Log.Info("Editor", "Script Editor显示成功");
        }
        
        private static void HideScriptEditor()
        {
            if (!_scriptEditorVisible) return;
            
            if (_scriptEditorPanel != null)
            {
                UI.RemoveWidget(_scriptEditorPanel.Id);
                _scriptEditorPanel = null;
            }
            _scriptEditorVisible = false;
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "编辑器";
            }
            
            Log.Info("Editor", "脚本编辑器隐藏");
        }
    }
}