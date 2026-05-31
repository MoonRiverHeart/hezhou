using System;
using System.IO;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        // =====================================================
        // VIEW: UI Creation and Layout Methods (SplitView-based)
        // =====================================================

        // === Helper: Map property category string to tab index ===
        // Tab indices: 0=几何, 1=位置, 2=渲染, 3=运动, 4=物理
        private static int MapCategoryToTabIndex(string category)
        {
            if (category == null || category.Length == 0) return 1; // default: 位置
            string lower = category.ToLower();
            if (lower == "geometry" || lower == "mesh" || lower == "几何") return 0;
            if (lower == "position" || lower == "transform" || lower == "位置") return 1;
            if (lower == "render" || lower == "material" || lower == "渲染") return 2;
            if (lower == "motion" || lower == "script" || lower == "运动") return 3;
            if (lower == "physics" || lower == "物理") return 4;
            return 1; // default: 位置
        }

        private static void ShowWorkingDirectoryDialog()
        {
            ulong rootId = UI.GetRootId();
            
            float dialogWidth = 500f;
            float dialogHeight = 400f;
            
            _workingDirectoryDialogId = UI.CreateDialog(rootId, "选择工作目录", dialogWidth, dialogHeight);
            UI.DialogSetOnResult(_workingDirectoryDialogId, _workingDirectoryDialogResultCallback);
            
            ulong contentId = UI.CreateVStack(_workingDirectoryDialogId, 10f);
            
            UI.CreateLabel(contentId, dialogWidth - 40f, 30f, "请选择项目工作目录:");
            
            _workingDirectoryFileBrowserId = UI.CreateFileBrowser(contentId, 10f, 10f, dialogWidth - 60f, dialogHeight - 120f, "scripts");
            UI.FileBrowserSetFilter(_workingDirectoryFileBrowserId, "*.cs;*.json;*.txt");
            UI.FileBrowserSetOnSelect(_workingDirectoryFileBrowserId, _fileBrowserSelectCallback);
            
            UI.DialogSetContent(_workingDirectoryDialogId, contentId);
            UI.DialogAddButton(_workingDirectoryDialogId, "确认", 1);
            UI.DialogAddButton(_workingDirectoryDialogId, "使用默认目录", 2);
            UI.DialogShow(_workingDirectoryDialogId);
        }

        // === Main Layout Creation: SplitView-based resizable layout ===

        private static void CreateEditorLayout()
        {
            // 热重载时从Rust全局SCENE指针恢复旧Scene(包含entity和binding)，首次初始化才创建新Scene
            IntPtr existingPtr = UI.SceneGetExistingPtr();
            Log.Info("Editor", "[CreateEditorLayout] SceneGetExistingPtr返回: " + existingPtr.ToInt64());
            if (existingPtr != IntPtr.Zero)
            {
                try
                {
                    _gameScene = new Scene(existingPtr);
                    Log.Info("Editor", "[CreateEditorLayout] 热重载恢复Scene成功: ptr=" + existingPtr.ToInt64() + ", _entities.Count=" + _gameScene.GetEntityCount() + ", ScenePtr=" + _gameScene.ScenePtr.ToInt64());
                    uint rustEntityCount = UI.SceneRootEntityCount(_gameScene.ScenePtr);
                    Log.Info("Editor", "[CreateEditorLayout] Rust侧root_entities数量: " + rustEntityCount);
                }
                catch (Exception ex)
                {
                    Log.Error("Editor", "[CreateEditorLayout] 恢复Scene失败: " + ex.Message + " — 创建新Scene");
                    _gameScene = new Scene();
                    ulong lightId = _gameScene.CreateDirectionalLight();
                    UI.SceneSetEntityName(_gameScene.ScenePtr, lightId, "DirectionalLight");
                    _gameScene.SetGameState(GameState.Editing);
                }
            }
            else
            {
                Log.Info("Editor", "[CreateEditorLayout] 首次初始化 — 创建新Scene");
                _gameScene = new Scene();
                
                ulong lightId = _gameScene.CreateDirectionalLight();
                UI.SceneSetEntityName(_gameScene.ScenePtr, lightId, "DirectionalLight");
                
                _gameScene.SetGameState(GameState.Editing);
            }
            
            float toolbarY = 0f;
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
            float statusY = _screenHeight - STATUS_BAR_HEIGHT;
            
            ulong rootId = UI.GetRootId();

            // === Toolbar (fixed, top) ===
            _toolbar = new Panel(rootId, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT, 0.15f, 0.15f, 0.15f, 1.0f);
            _toolbarButtons = new HStack(_toolbar.Id, 10f);
            _toolbarButtons.SetPosition(10f, 5f);
            
            CreateToolbarMenus();
            
            var fileMenuLabel = _toolbarButtons.AddLabel(80f, 30f, "文件");
            UI.SetOnClick(fileMenuLabel, _newClickCallback);
            _menuBarLabelIds[0] = fileMenuLabel;
            
            var openMenuLabel = _toolbarButtons.AddLabel(80f, 30f, "打开");
            UI.SetOnClick(openMenuLabel, _openClickCallback);
            _menuBarLabelIds[1] = openMenuLabel;
            
            var saveMenuLabel = _toolbarButtons.AddLabel(80f, 30f, "保存");
            UI.SetOnClick(saveMenuLabel, _saveClickCallback);
            _menuBarLabelIds[2] = saveMenuLabel;
            
            var runBtn = _toolbarButtons.AddButton(100f, 30f, "运行");
            _runButtonId = runBtn.Id;
            runBtn.SetOnClick(_runClickCallback);
            
            _pauseButtonId = UI.CreateButton(_toolbar.Id, 80f, 30f, "暂停");
            UI.SetOnClick(_pauseButtonId, _pauseClickCallback);
            UI.SetWidgetLayout(_pauseButtonId, -100f, -100f, 80f, 30f);
            
            _toggleEditorBtn = new Button(_toolbar.Id, 100f, 30f, "编辑器");
            UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 120f, 5f, 100f, 30f);
            _toggleEditorBtn.SetOnClick(_toggleEditorClickCallback);

            // === SplitView-based Main Layout ===
            // Structure:
            //   OuterSplitView (Horizontal): LeftCenterZone | PropertiesPanel
            //     LeftCenterSplitView (Vertical): TopZone | AssetPanel
            //       InnerSplitView (Horizontal): ProjectPanel | PreviewPanel
            //     PropertiesPanel (with 5-tab TabWidget)

            // Outer Horizontal SplitView: Left+Center (75%) | Properties (25%)
            _outerSplitViewId = UI.CreateSplitView(rootId, 0, mainY, _screenWidth, mainHeight, 0);
            UI.SplitViewSetSplitRatio(_outerSplitViewId, 0.75f);
            UI.SplitViewSetMinRatio(_outerSplitViewId, 0.4f);
            UI.SplitViewSetMaxRatio(_outerSplitViewId, 0.9f);
            UI.SplitViewSetOnRatioChange(_outerSplitViewId, _outerSplitViewRatioCallback);

            // First pane of outer SplitView: LeftCenter zone
            _leftCenterPanel = new Panel(_outerSplitViewId, 0, 0, 0, 0, 0.18f, 0.18f, 0.22f, 1.0f);
            
            // Second pane of outer SplitView: Properties panel
            _rightPanel = new Panel(_outerSplitViewId, 0, 0, 0, 0, 0.2f, 0.2f, 0.2f, 1.0f);

            // LeftCenter Vertical SplitView: Top (70%) | Asset (30%)
            _leftCenterSplitViewId = UI.CreateSplitView(_leftCenterPanel.Id, 0, 0, 0, 0, 1);
            UI.SplitViewSetSplitRatio(_leftCenterSplitViewId, 0.7f);
            UI.SplitViewSetMinRatio(_leftCenterSplitViewId, 0.3f);
            UI.SplitViewSetMaxRatio(_leftCenterSplitViewId, 0.85f);
            UI.SplitViewSetOnRatioChange(_leftCenterSplitViewId, _leftCenterSplitViewRatioCallback);

            // First pane of LeftCenter SplitView: Top zone
            _topPanel = new Panel(_leftCenterSplitViewId, 0, 0, 0, 0, 0.18f, 0.18f, 0.22f, 1.0f);

            // Second pane of LeftCenter SplitView: Asset panel
            _assetPanel = new Panel(_leftCenterSplitViewId, 0, 0, 0, 0, 0.2f, 0.2f, 0.2f, 1.0f);

            // Inner Horizontal SplitView: Project (33%) | Preview (67%)
            _innerHorizontalSplitViewId = UI.CreateSplitView(_topPanel.Id, 0, 0, 0, 0, 0);
            UI.SplitViewSetSplitRatio(_innerHorizontalSplitViewId, 0.33f);
            UI.SplitViewSetMinRatio(_innerHorizontalSplitViewId, 0.15f);
            UI.SplitViewSetMaxRatio(_innerHorizontalSplitViewId, 0.5f);
            UI.SplitViewSetOnRatioChange(_innerHorizontalSplitViewId, _innerHorizontalSplitViewRatioCallback);

            // First pane of Inner SplitView: Project panel
            _projectPanel = new Panel(_innerHorizontalSplitViewId, 0, 0, 0, 0, 0.2f, 0.2f, 0.2f, 1.0f);
            UI.CreateLabel(_projectPanel.Id, 10f, 10f, 200f, 25f, "项目结构");
            CreateProjectStructureTree();

            // Second pane of Inner SplitView: Preview panel
            _previewPanel = new Panel(_innerHorizontalSplitViewId, 0, 0, 0, 0, 0.08f, 0.08f, 0.08f, 0.3f);
            UI.CreateLabel(_previewPanel.Id, 10f, 10f, 200f, 25f, "游戏预览");
            
            float initialPreviewWidth = _screenWidth * 0.75f * 0.67f - 20f;
            float initialPreviewHeight = mainHeight * 0.7f - 50f;
            if (initialPreviewWidth < 100f) initialPreviewWidth = 100f;
            if (initialPreviewHeight < 100f) initialPreviewHeight = 100f;
            
            _previewWindowId = UI.CreatePreviewWindow(_previewPanel.Id, 10f, 40f, initialPreviewWidth, initialPreviewHeight, 1);
            UI.SetWidgetLayer(_previewWindowId, 0);
            UI.SetGamePreviewExtent((uint)(initialPreviewWidth * UI.GetContentScale()), (uint)(initialPreviewHeight * UI.GetContentScale()));

            // === Asset Panel: TabWidget spanning left+center bottom ===
            _assetTabWidget = new TabWidget(_assetPanel.Id, 5f, 5f, 0, 0);
            
            ulong modelTabContent = UI.CreateScrollView(_assetTabWidget.Id, 0, 0, 0, 0);
            _assetModelGridViewId = UI.CreateGridView(modelTabContent, 5f, 5f, 0, 0, 64);
            _assetGridViewId = _assetModelGridViewId; // Legacy alias
            RefreshAssetGridView();
            UI.GridViewSetOnClick(_assetModelGridViewId, _gridViewClickCallback);
            
            ulong materialTabContent = UI.CreateVStack(_assetTabWidget.Id, 5f);
            UI.CreateLabel(materialTabContent, 200f, 25f, "材质列表 (待实现)");
            
            ulong scriptTabContent = UI.CreateVStack(_assetTabWidget.Id, 5f);
            UI.CreateLabel(scriptTabContent, 200f, 25f, "脚本列表 (待实现)");
            
            ulong sceneTabContent = UI.CreateVStack(_assetTabWidget.Id, 5f);
            UI.CreateLabel(sceneTabContent, 200f, 25f, "场景列表 (待实现)");
            
            _assetTabWidget.AddTab("模型", modelTabContent, false);
            _assetTabWidget.AddTab("材质", materialTabContent, false);
            _assetTabWidget.AddTab("脚本", scriptTabContent, false);
            _assetTabWidget.AddTab("场景", sceneTabContent, false);

            // === Properties Panel: 5-tab TabWidget ===
            UI.CreateLabel(_rightPanel.Id, 10f, 10f, 200f, 25f, "属性编辑");
            
            float tabWidgetY = 40f;
            float tabWidgetHeight = mainHeight - tabWidgetY - 10f;
            
            _propsTabWidget = new TabWidget(_rightPanel.Id, 5f, tabWidgetY, 0, tabWidgetHeight);
            
            // Create 5 tab content containers (ScrollView for scrollable property lists)
            _geometryTabContentId = UI.CreateScrollView(_propsTabWidget.Id, 0, 0, 0, 0);
            ulong geometryVStack = UI.CreateVStack(_geometryTabContentId, 5f);
            
            _positionTabContentId = UI.CreateScrollView(_propsTabWidget.Id, 0, 0, 0, 0);
            ulong positionVStack = UI.CreateVStack(_positionTabContentId, 5f);
            
            _renderTabContentId = UI.CreateScrollView(_propsTabWidget.Id, 0, 0, 0, 0);
            ulong renderVStack = UI.CreateVStack(_renderTabContentId, 5f);
            
            // === 管线选择区域 (渲染tab顶部) ===
            UI.CreateLabel(renderVStack, RIGHT_PANEL_WIDTH - 40f, 20f, "管线选择:");
            
            ulong pipelineHStack = UI.CreateHStack(renderVStack, 5f);
            _pipelineDropdownId = UI.CreateDropdown(pipelineHStack, RIGHT_PANEL_WIDTH - 70f, 25f);
            
// 设置Dropdown选项 — 通过FFI获取可用管线名
            string[] names = UI.GetPipelineNames();
            if (names.Length > 0)
            {
                _pipelineNames = names;
            }
            else
            {
                // Fallback: include both rasterization and ray_tracing even if not yet registered
                _pipelineNames = new string[] { "rasterization", "ray_tracing" };
            }
            UI.DropdownSetOptions(_pipelineDropdownId, _pipelineNames);
            
            // 设置当前选中项 — 通过FFI获取当前活跃管线名
            string current = UI.GetActivePipelineName();
            _currentPipeline = current;
            int selectedIndex = 0;
            for (int i = 0; i < _pipelineNames.Length; i++)
            {
                if (_pipelineNames[i] == current) { selectedIndex = i; break; }
            }
            UI.DropdownSetSelected(_pipelineDropdownId, (ulong)selectedIndex);
            
            // 注册选择回调
            _pipelineDropdownSelectCallback = new UI.DropdownSelectCallbackDelegate(OnPipelineSelected);
            UI.DropdownSetOnSelect(_pipelineDropdownId, _pipelineDropdownSelectCallback);
            
            // 分隔Label
            UI.CreateLabel(renderVStack, RIGHT_PANEL_WIDTH - 40f, 10f, "");
            
            // 当前管线信息Label
            _pipelineInfoLabelId = UI.CreateLabel(renderVStack, RIGHT_PANEL_WIDTH - 40f, 20f, "当前管线: " + _currentPipeline);
            
            _motionTabContentId = UI.CreateVStack(_propsTabWidget.Id, 5f);
            
            _physicsTabContentId = UI.CreateScrollView(_propsTabWidget.Id, 0, 0, 0, 0);
            ulong physicsVStack = UI.CreateVStack(_physicsTabContentId, 5f);

            // Build property panels for each category tab
            _propertyDescriptors.Clear();
            _propertyCallbacks.Clear();
            _propertyInputFieldCallbacks.Clear();
            
            uint propCount = UI.EntityGetPropertyCount();
            
            // Categorize properties into tab content containers
            for (uint i = 0; i < propCount; i++)
            {
                string name = UI.EntityGetPropertyName(i);
                uint type = UI.EntityGetPropertyType(i);
                string category = UI.EntityGetPropertyCategory(i);
                bool readOnly = UI.EntityGetPropertyReadOnly(i);
                
                int tabIndex = MapCategoryToTabIndex(category);
                ulong targetContainer;
                
                switch (tabIndex)
                {
                    case 0: targetContainer = geometryVStack; break;
                    case 1: targetContainer = positionVStack; break;
                    case 2: targetContainer = renderVStack; break;
                    case 3: targetContainer = _motionTabContentId; break;
                    case 4: targetContainer = physicsVStack; break;
                    default: targetContainer = positionVStack; break;
                }
                
                BuildSingleProperty(targetContainer, name, type, category, readOnly);
            }
            
            // Build Motion tab: script dropdown + 绑定 button + binding list
            BuildMotionTab(_motionTabContentId);
            
            // Set ScrollView content sizes based on property count per tab
            UpdatePropertyScrollViewContentSize(_geometryTabContentId, geometryVStack);
            UpdatePropertyScrollViewContentSize(_positionTabContentId, positionVStack);
            UpdatePropertyScrollViewContentSize(_renderTabContentId, renderVStack);
            UpdatePropertyScrollViewContentSize(_physicsTabContentId, physicsVStack);
            
            _propsTabWidget.AddTab("几何", _geometryTabContentId, false);
            _propsTabWidget.AddTab("位置", _positionTabContentId, false);
            _propsTabWidget.AddTab("渲染", _renderTabContentId, false);
            _propsTabWidget.AddTab("运动", _motionTabContentId, false);
            _propsTabWidget.AddTab("物理", _physicsTabContentId, false);
            _propsTabWidget.SetOnSelect(_tabSelectCallback);

            // === Status Bar (fixed, bottom) ===
            _statusBar = new Panel(rootId, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT, 0.12f, 0.12f, 0.12f, 1.0f);
            _statusItems = new List(_statusBar.Id, 0f, true);
            UI.SetWidgetLayout(_statusItems.Id, 10f, 0f, _screenWidth - 20f, STATUS_BAR_HEIGHT);
            
            float fontSize = 14f;
            float itemHeight = STATUS_BAR_HEIGHT - 4f;
            
            _fpsItem = _statusItems.AddItem("FPS: 0", false);
            UI.SetWidgetLayout(_fpsItem.Id, 0f, 2f, 120f, itemHeight);
            UI.SetListItemFontSize(_fpsItem.Id, fontSize);
            
            _statusItem = _statusItems.AddItem("状态: 就绪", true);
            UI.SetWidgetLayout(_statusItem.Id, 130f, 2f, 150f, itemHeight);
            UI.SetListItemFontSize(_statusItem.Id, fontSize);
            
            _projectItem = _statusItems.AddItem("项目: 未命名", true);
            UI.SetWidgetLayout(_projectItem.Id, 290f, 2f, 150f, itemHeight);
            UI.SetListItemFontSize(_projectItem.Id, fontSize);

            // === Script Editor Panel (pre-created, initially hidden) ===
            // Created under rootId at the same level as outerSplitView, toggled via SetWidgetVisible
            float editorY = TOOLBAR_HEIGHT;
            float editorHeight = mainHeight;
            float editorWidth = _screenWidth;
            
            _scriptEditorPanel = new Panel(rootId, 0, editorY, editorWidth, editorHeight, 0.12f, 0.12f, 0.14f, 1.0f);
            
            // Toolbar: Hot Reload button + label
            _scriptEditorHotReloadBtnId = UI.CreateButton(_scriptEditorPanel.Id, 100f, 30f, "Hot Reload");
            UI.SetWidgetLayout(_scriptEditorHotReloadBtnId, 10f, 10f, 100f, 30f);
            UI.SetOnClick(_scriptEditorHotReloadBtnId, _hotReloadClickCallback);
            
            _scriptEditorLabel = new Label(_scriptEditorPanel.Id, 200f, 25f, "Script Editor - NewScript.cs");
            UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
            
            // Horizontal SplitView: TreePanel(20%) | EditPanel(80%), below toolbar
            _scriptEditorSplitViewId = UI.CreateSplitView(_scriptEditorPanel.Id, 0, 40f, editorWidth, editorHeight - 40f, 0);
            UI.SplitViewSetSplitRatio(_scriptEditorSplitViewId, 0.2f);
            UI.SplitViewSetMinRatio(_scriptEditorSplitViewId, 0.1f);
            UI.SplitViewSetMaxRatio(_scriptEditorSplitViewId, 0.5f);
            _scriptEditorSplitViewRatioCallback = new UI.SplitViewRatioChangeCallbackDelegate(OnScriptEditorSplitViewRatioChange);
            UI.SplitViewSetOnRatioChange(_scriptEditorSplitViewId, _scriptEditorSplitViewRatioCallback);
            
            // Left pane: directory tree
            _scriptEditorTreePanel = new Panel(_scriptEditorSplitViewId, 0, 0, 0, 0, 0.18f, 0.18f, 0.22f, 1.0f);
            
            // Right pane: TextEdit
            _scriptEditorEditPanel = new Panel(_scriptEditorSplitViewId, 0, 0, 0, 0, 0.15f, 0.15f, 0.15f, 1.0f);
            
            _scriptTextEditId = UI.CreateTextEdit(_scriptEditorEditPanel.Id, editorWidth * 0.8f - 20f, editorHeight - 90f);
            UI.SetTextEditShowLineNumbers(_scriptTextEditId, true);
            UI.SetWidgetLayout(_scriptTextEditId, 10f, 10f, editorWidth * 0.8f - 20f, editorHeight - 90f);
            UI.TextEditSetText(_scriptTextEditId, "// NewScript.cs\nusing System;\nusing Hezhou;\n\npublic class NewScript\n{\n    public void Start()\n    {\n        Console.WriteLine(\"NewScript started!\");\n    }\n    \n    public void Update(float deltaTime)\n    {\n        // Update logic here\n    }\n}");
            
            // Initially hidden — only shown when user clicks "编辑器" button
            UI.SetWidgetVisible(_scriptEditorPanel.Id, false);
        }

        // === Build a single property widget into a target container ===
        private static void BuildSingleProperty(ulong parentContainerId, string name, uint type, string category, bool readOnly)
        {
            PropertyDescriptor desc = new PropertyDescriptor();
            desc.Name = name;
            desc.Type = type;
            desc.Category = category;
            desc.ReadOnly = readOnly;
            
            string displayName = name.Length > 0 ? name.Substring(0, 1).ToUpper() + name.Substring(1) : name;
            
            if (type == 1) // Float3
            {
                // Label独占一行(wrap模式)，然后HStack放3个InputField
                desc.LabelId = UI.CreateLabel(parentContainerId, 0, 20f, displayName + ":");
                UI.SetLabelWrapMode(desc.LabelId, 1); // 启用Wrap模式
                
                ulong hStackId = UI.CreateHStack(parentContainerId, 5f);
                desc.WidgetIds = new ulong[3];
                
                string[] axisLabels = new string[] { "X", "Y", "Z" };
                for (int j = 0; j < 3; j++)
                {
                    ulong inputId = UI.CreateInputField(hStackId, 70f, 25f);
                    UI.InputFieldSetPlaceholder(inputId, axisLabels[j]);
                    
                    string propName = name;
                    int compIdx = j;
                    
                    PropertyChangeCallback pcb = delegate(ulong wid, string txt) {
                        HandleFloat3ComponentChange(propName, compIdx, wid, txt);
                    };
                    _propertyCallbacks[inputId] = pcb;
                    
                    UI.InputFieldChangeCallbackDelegate ifcb = delegate(ulong wid, string txt) {
                        HandleFloat3ComponentChange(propName, compIdx, wid, txt);
                    };
                    _propertyInputFieldCallbacks[inputId] = ifcb;
                    UI.InputFieldSetOnChange(inputId, ifcb);
                    
                    desc.WidgetIds[j] = inputId;
                }
            }
            else if (type == 2) // String
            {
                // Label独占一行(wrap模式)，InputField/ValueLabel独占另一行
                desc.LabelId = UI.CreateLabel(parentContainerId, 0, 20f, displayName + ":");
                UI.SetLabelWrapMode(desc.LabelId, 1); // 启用Wrap模式
                
                if (readOnly)
                {
                    ulong valueLabelId = UI.CreateLabel(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 20f, "");
                    desc.WidgetIds = new ulong[] { valueLabelId };
                }
                else
                {
                    ulong inputId = UI.CreateInputField(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 25f);
                    UI.InputFieldSetPlaceholder(inputId, displayName);
                    
                    string propName = name;
                    
                    PropertyChangeCallback pcb = delegate(ulong wid, string txt) {
                        HandleStringPropertyChange(propName, wid, txt);
                    };
                    _propertyCallbacks[inputId] = pcb;
                    
                    UI.InputFieldChangeCallbackDelegate ifcb = delegate(ulong wid, string txt) {
                        pcb(wid, txt);
                    };
                    _propertyInputFieldCallbacks[inputId] = ifcb;
                    UI.InputFieldSetOnChange(inputId, ifcb);
                    
                    desc.WidgetIds = new ulong[] { inputId };
                }
            }
            else if (type == 4) // Int
            {
                // Label独占一行(wrap模式)，InputField/ValueLabel独占另一行
                desc.LabelId = UI.CreateLabel(parentContainerId, 0, 20f, displayName + ":");
                UI.SetLabelWrapMode(desc.LabelId, 1); // 启用Wrap模式
                
                if (readOnly)
                {
                    ulong valueLabelId = UI.CreateLabel(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 20f, "");
                    desc.WidgetIds = new ulong[] { valueLabelId };
                }
                else
                {
                    ulong inputId = UI.CreateInputField(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 25f);
                    UI.InputFieldSetPlaceholder(inputId, displayName);
                    
                    string propName = name;
                    
                    PropertyChangeCallback pcb = delegate(ulong wid, string txt) {
                        HandleIntPropertyChange(propName, wid, txt);
                    };
                    _propertyCallbacks[inputId] = pcb;
                    
                    UI.InputFieldChangeCallbackDelegate ifcb = delegate(ulong wid, string txt) {
                        pcb(wid, txt);
                    };
                    _propertyInputFieldCallbacks[inputId] = ifcb;
                    UI.InputFieldSetOnChange(inputId, ifcb);
                    
                    desc.WidgetIds = new ulong[] { inputId };
                }
            }
            else if (type == 0) // Float
            {
                // Label独占一行(wrap模式)，InputField/ValueLabel独占另一行
                desc.LabelId = UI.CreateLabel(parentContainerId, 0, 20f, displayName + ":");
                UI.SetLabelWrapMode(desc.LabelId, 1); // 启用Wrap模式
                
                if (readOnly)
                {
                    ulong valueLabelId = UI.CreateLabel(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 20f, "");
                    desc.WidgetIds = new ulong[] { valueLabelId };
                }
                else
                {
                    ulong inputId = UI.CreateInputField(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 25f);
                    UI.InputFieldSetPlaceholder(inputId, displayName);
                    
                    string propName = name;
                    
                    PropertyChangeCallback pcb = delegate(ulong wid, string txt) {
                        HandleFloatPropertyChange(propName, wid, txt);
                    };
                    _propertyCallbacks[inputId] = pcb;
                    
                    UI.InputFieldChangeCallbackDelegate ifcb = delegate(ulong wid, string txt) {
                        pcb(wid, txt);
                    };
                    _propertyInputFieldCallbacks[inputId] = ifcb;
                    UI.InputFieldSetOnChange(inputId, ifcb);
                    
                    desc.WidgetIds = new ulong[] { inputId };
                }
            }
            else if (type == 3) // Bool
            {
                // Label独占一行(wrap模式)，InputField/ValueLabel独占另一行
                desc.LabelId = UI.CreateLabel(parentContainerId, 0, 20f, displayName + ":");
                UI.SetLabelWrapMode(desc.LabelId, 1); // 启用Wrap模式
                
                if (readOnly)
                {
                    ulong valueLabelId = UI.CreateLabel(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 20f, "");
                    desc.WidgetIds = new ulong[] { valueLabelId };
                }
                else
                {
                    ulong inputId = UI.CreateInputField(parentContainerId, RIGHT_PANEL_WIDTH - 40f, 25f);
                    UI.InputFieldSetPlaceholder(inputId, "true/false");
                    
                    string propName = name;
                    
                    PropertyChangeCallback pcb = delegate(ulong wid, string txt) {
                        HandleBoolPropertyChange(propName, wid, txt);
                    };
                    _propertyCallbacks[inputId] = pcb;
                    
                    UI.InputFieldChangeCallbackDelegate ifcb = delegate(ulong wid, string txt) {
                        pcb(wid, txt);
                    };
                    _propertyInputFieldCallbacks[inputId] = ifcb;
                    UI.InputFieldSetOnChange(inputId, ifcb);
                    
                    desc.WidgetIds = new ulong[] { inputId };
                }
            }
            else
            {
                // Unsupported property type (Enum=5 etc) - skip
                desc.LabelId = 0;
                desc.WidgetIds = new ulong[0];
            }
            
            _propertyDescriptors.Add(desc);
        }

        // === Build a single script property widget (Slider or InputField + Min/Max/Step/Initial) ===
        private static ScriptPropertyInfo BuildSingleScriptProperty(ulong parentId, ScriptPropertyDescriptor descriptor, ulong entityId, int bindingIndex, ulong instanceId, Type scriptType)
        {
            ScriptPropertyInfo info = new ScriptPropertyInfo();
            info.EntityId = entityId;
            info.BindingIndex = bindingIndex;
            info.PropertyName = descriptor.Name;
            info.ClassName = scriptType.Name;
            info.InstanceId = (long)instanceId;
            info.WidgetType = descriptor.Widget;

            // 显示名: DisplayName优先，空则用字段名首字母大写
            string displayName = descriptor.DisplayName;
            if (displayName == null || displayName.Length == 0)
            {
                displayName = descriptor.Name.Length > 0
                    ? descriptor.Name.Substring(0, 1).ToUpper() + descriptor.Name.Substring(1)
                    : descriptor.Name;
            }

            // 读取runtime当前值
            float currentRuntimeValue = 0.0f;
            if (instanceId != 0 && scriptType != null)
            {
                IntPtr instancePtr = new IntPtr((long)instanceId);
                currentRuntimeValue = HezhouScripts.ScriptEntityHelper.GetFieldValue(instancePtr, descriptor.Name, scriptType);
            }

            // === Label (Wrap模式) ===
            ulong labelId = UI.CreateLabel(parentId, 0, 20f, displayName + ":");
            UI.SetLabelWrapMode(labelId, 1);

            // === 主控件: Slider 或 InputField ===
            ulong mainWidgetId = 0;
            if (descriptor.Widget == "slider")
            {
                mainWidgetId = UI.CreateSlider(parentId, 150f, 24f);
                UI.SliderSetRange(mainWidgetId, descriptor.Min, descriptor.Max);
                UI.SliderSetStep(mainWidgetId, descriptor.Step);
                UI.SliderSetValue(mainWidgetId, currentRuntimeValue);
            }
            else
            {
                // "input" 或其他 → InputField
                mainWidgetId = UI.CreateInputField(parentId, 150f, 24f);
                UI.InputFieldSetText(mainWidgetId, currentRuntimeValue.ToString());
                UI.InputFieldSetPlaceholder(mainWidgetId, displayName);
            }
            info.MainWidgetId = mainWidgetId;

            // === Min/Max/Step/Initial HStack ===
            ulong configHStack = UI.CreateHStack(parentId, 5f);

            // Min
            ulong minLabelId = UI.CreateLabel(configHStack, 30f, 20f, "Min:");
            ulong minInputId = UI.CreateInputField(configHStack, 50f, 24f);
            UI.InputFieldSetText(minInputId, descriptor.Min.ToString());
            UI.InputFieldSetPlaceholder(minInputId, "Min");
            info.MinInputId = minInputId;
            info.CurrentMin = descriptor.Min;

            // Max
            ulong maxLabelId = UI.CreateLabel(configHStack, 30f, 20f, "Max:");
            ulong maxInputId = UI.CreateInputField(configHStack, 50f, 24f);
            UI.InputFieldSetText(maxInputId, descriptor.Max.ToString());
            UI.InputFieldSetPlaceholder(maxInputId, "Max");
            info.MaxInputId = maxInputId;
            info.CurrentMax = descriptor.Max;

            // Step
            ulong stepLabelId = UI.CreateLabel(configHStack, 30f, 20f, "Step:");
            ulong stepInputId = UI.CreateInputField(configHStack, 50f, 24f);
            UI.InputFieldSetText(stepInputId, descriptor.Step.ToString());
            UI.InputFieldSetPlaceholder(stepInputId, "Step");
            info.StepInputId = stepInputId;
            info.CurrentStep = descriptor.Step;

            // Init
            ulong initLabelId = UI.CreateLabel(configHStack, 30f, 20f, "Init:");
            ulong initInputId = UI.CreateInputField(configHStack, 50f, 24f);
            UI.InputFieldSetText(initInputId, descriptor.Initial.ToString());
            UI.InputFieldSetPlaceholder(initInputId, "Init");
            info.InitialInputId = initInputId;
            info.CurrentInitial = descriptor.Initial;

            // 注册 Min/Max/Step/Initial input → mainWidget 映射（回调路由用）
            _scriptMinMaxInitialToMainWidgetMap[minInputId] = mainWidgetId;
            _scriptMinMaxInitialToMainWidgetMap[maxInputId] = mainWidgetId;
            _scriptMinMaxInitialToMainWidgetMap[stepInputId] = mainWidgetId;
            _scriptMinMaxInitialToMainWidgetMap[initInputId] = mainWidgetId;

            // 注册 mainWidget → ScriptPropertyInfo 映射（回调路由用）
            _scriptPropertyInfoMap[mainWidgetId] = info;

            return info;
        }

        // === Build Motion tab: script dropdown + 绑定 button + binding list ===
        private static void BuildMotionTab(ulong motionTabContentId)
        {
            // Script selection section
            UI.CreateLabel(motionTabContentId, RIGHT_PANEL_WIDTH - 40f, 20f, "脚本绑定:");
            
            ulong scriptHStack = UI.CreateHStack(motionTabContentId, 5f);
            _scriptDropdownId = UI.CreateDropdown(scriptHStack, RIGHT_PANEL_WIDTH - 70f, 25f);
            string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
            UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
            UI.DropdownSetOnSelect(_scriptDropdownId, _scriptDropdownSelectCallback);
            
            _bindScriptBtnId = UI.CreateButton(scriptHStack, 50f, 25f, "绑定");
            UI.SetOnClick(_bindScriptBtnId, _bindScriptClickCallback);
            
            // Script bindings list container
            _scriptsListContainerId = UI.CreateVStack(motionTabContentId, 5f);
        }

        // === Update ScrollView content size based on child count ===
        private static void UpdatePropertyScrollViewContentSize(ulong scrollViewId, ulong contentVStackId)
        {
            uint childCount = UI.WidgetGetChildCount(contentVStackId);
            float estimatedHeight = childCount * 30f; // rough estimate per property row
            UI.ScrollViewSetContentSize(scrollViewId, RIGHT_PANEL_WIDTH - 20f, estimatedHeight);
        }

        // === Resize Handling ===

        private static void OnResize(float width, float height)
        {
            _screenWidth = width;
            _screenHeight = height;
            
            UpdateLayout();
        }

        private static void UpdateLayout()
        {
            float toolbarY = 0f;
            float mainY = TOOLBAR_HEIGHT;
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
            float statusY = _screenHeight - STATUS_BAR_HEIGHT;

            // Toolbar and status bar: fixed position panels
            if (_toolbar != null)
                UI.SetWidgetLayout(_toolbar.Id, 0, toolbarY, _screenWidth, TOOLBAR_HEIGHT);
            if (_toggleEditorBtn != null)
                UI.SetWidgetLayout(_toggleEditorBtn.Id, _screenWidth - 100f, 5f, 80f, 30f);
            
            // Outer SplitView: fills the main area between toolbar and status bar
            if (_outerSplitViewId != 0)
                UI.SetWidgetLayout(_outerSplitViewId, 0, mainY, _screenWidth, mainHeight);
            
            // SplitView handles child panel sizing automatically via layout_split_view_children
            
            // Status bar
            if (_statusBar != null)
                UI.SetWidgetLayout(_statusBar.Id, 0, statusY, _screenWidth, STATUS_BAR_HEIGHT);
            
            // Update preview window extent after layout pass
            UpdatePreviewExtent();
            
            // Script editor overlay (if visible)
            if (_scriptEditorPanel != null && _scriptEditorVisible)
            {
                float editorWidth = _screenWidth;
                float editorHeight = mainHeight;
                UI.SetWidgetLayout(_scriptEditorPanel.Id, 0, mainY, editorWidth, editorHeight);
                // SplitView handles child panel sizing automatically
                if (_scriptEditorSplitViewId != 0)
                {
                    UI.SetWidgetLayout(_scriptEditorSplitViewId, 0, 40f, editorWidth, editorHeight - 40f);
                }
                if (_scriptEditorHotReloadBtnId != 0)
                {
                    UI.SetWidgetLayout(_scriptEditorHotReloadBtnId, 10f, 10f, 100f, 30f);
                }
                if (_scriptEditorLabel != null)
                {
                    UI.SetWidgetLayout(_scriptEditorLabel.Id, 120f, 10f, 300f, 25f);
                }
            }
        }

        // === Update preview window extent based on actual preview panel layout ===
        private static void UpdatePreviewExtent()
        {
            if (_previewWindowId == 0 || _previewPanel == null || _scriptEditorVisible) return;
            
            float[] previewLayout = UI.WidgetGetLayout(_previewPanel.Id);
            if (previewLayout != null && previewLayout.Length >= 4)
            {
                float previewWidth = previewLayout[2] - 20f;
                float previewHeight = previewLayout[3] - 50f;
                if (previewWidth < 50f) previewWidth = 50f;
                if (previewHeight < 50f) previewHeight = 50f;
                
                UI.SetWidgetLayout(_previewWindowId, 10f, 40f, previewWidth, previewHeight);
                // Scale logical dimensions by content_scale to match physical pixel FBO size
                uint physWidth = (uint)(previewWidth * UI.GetContentScale());
                uint physHeight = (uint)(previewHeight * UI.GetContentScale());
                UI.SetGamePreviewExtent(physWidth, physHeight);
            }
        }

        // === SplitView Ratio Change Callbacks ===

        private static void OnOuterSplitViewRatioChange(ulong widgetId, float ratio)
        {
            // Outer split changed: update preview extent
            UpdatePreviewExtent();
        }

        private static void OnLeftCenterSplitViewRatioChange(ulong widgetId, float ratio)
        {
            // LeftCenter vertical split changed: update preview extent
            UpdatePreviewExtent();
        }

        private static void OnInnerHorizontalSplitViewRatioChange(ulong widgetId, float ratio)
        {
            // Inner horizontal split changed: update preview extent
            UpdatePreviewExtent();
        }

        private static void OnScriptEditorSplitViewRatioChange(ulong widgetId, float ratio)
        {
            // Script editor split changed: SplitView handles panel sizing automatically
            // No manual TextEdit layout update needed — layout_panel_children will handle it
        }

        // === Show/Hide Main Layout ===

        private static void ShowMainLayout()
        {
            // Simply make the outerSplitView visible again (no recreate)
            if (_outerSplitViewId != 0)
            {
                UI.SetWidgetVisible(_outerSplitViewId, true);
            }
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "编辑器";
            }
            
            if (_selectedEntityId != 0 && _gameScene != null)
            {
                UpdatePropertiesPanel(_selectedEntityId);
            }
            
            // Restore preview extent after main layout is visible again
            UpdatePreviewExtent();
        }
        
        private static void HideMainLayout()
        {
            // Simply make the outerSplitView invisible (no destroy)
            if (_outerSplitViewId != 0)
            {
                UI.SetWidgetVisible(_outerSplitViewId, false);
            }
            
            if (_toggleEditorBtn != null)
            {
                _toggleEditorBtn.Text = "预览";
            }
        }

        // === Script Editor Show/Hide ===

        private static void ShowScriptEditor()
        {
            if (_scriptEditorVisible) return;
            
            // Hide main layout (make outerSplitView invisible)
            HideMainLayout();
            
            // Show the pre-created script editor panel
            if (_scriptEditorPanel != null)
            {
                UI.SetWidgetVisible(_scriptEditorPanel.Id, true);
            }
            
            _scriptEditorVisible = true;
            
            // Refresh directory tree inside the script editor panel
            RefreshDirectoryTree();
        }
        
        private static void HideScriptEditor()
        {
            if (!_scriptEditorVisible) return;
            
            // Hide the script editor panel
            if (_scriptEditorPanel != null)
            {
                UI.SetWidgetVisible(_scriptEditorPanel.Id, false);
            }
            
            _scriptEditorVisible = false;
            
            // Show main layout (make outerSplitView visible again)
            ShowMainLayout();
        }

        // === Project Structure Tree UI Creation ===

        private static void CreateProjectStructureTree()
        {
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
            float estimatedProjectHeight = mainHeight * 0.7f * 0.67f - 50f;
            if (estimatedProjectHeight < 100f) estimatedProjectHeight = 100f;
            
            _projectTreeViewId = UI.CreateTreeView(_projectPanel.Id, 10, 40, 200f, estimatedProjectHeight);
            
            _assetsNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "📦 Assets", 0, true);
            _scenesNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "🎬 Scenes", 0, true);
            _scriptsNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "📜 Scripts", 0, true);
            _entitiesNodeId = UI.TreeViewAddNode(_projectTreeViewId, 0, "🧩 Entities", 0, true);
            
            _nodeIdToName[_assetsNodeId] = "Assets";
            _nodeIdToName[_scenesNodeId] = "Scenes";
            _nodeIdToName[_scriptsNodeId] = "Scripts";
            _nodeIdToName[_entitiesNodeId] = "Entities";
            
            foreach (var script in _availableScripts)
            {
                ulong scriptNodeId = UI.TreeViewAddNode(_projectTreeViewId, _scriptsNodeId, "📝 " + script, 0, false);
                _nodeIdToName[scriptNodeId] = script;
            }
            
            if (_gameScene != null)
            {
                int entityCount = _gameScene.GetEntityCount();
                for (int i = 0; i < entityCount; i++)
                {
                    ulong entityId = _gameScene.GetEntityId(i);
                    string name = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
                    ulong nodeId = UI.TreeViewAddNode(_projectTreeViewId, _entitiesNodeId, "🔷 " + name, entityId, false);
                    _entityNodeMap[entityId] = nodeId;
                    _nodeIdToName[nodeId] = name;
                }
            }
            
            UI.TreeViewSetOnSelect(_projectTreeViewId, _treeNodeSelectCallback);
            UI.TreeViewSetOnToggle(_projectTreeViewId, _treeNodeToggleCallback);
            
            // Restore expansion state
            if (_expandedNodeNames.Count > 0)
            {
                if (_expandedNodeNames.Contains("Assets")) UI.TreeViewExpandNode(_projectTreeViewId, _assetsNodeId);
                else UI.TreeViewCollapseNode(_projectTreeViewId, _assetsNodeId);
                if (_expandedNodeNames.Contains("Scenes")) UI.TreeViewExpandNode(_projectTreeViewId, _scenesNodeId);
                else UI.TreeViewCollapseNode(_projectTreeViewId, _scenesNodeId);
                if (_expandedNodeNames.Contains("Scripts")) UI.TreeViewExpandNode(_projectTreeViewId, _scriptsNodeId);
                else UI.TreeViewCollapseNode(_projectTreeViewId, _scriptsNodeId);
                if (_expandedNodeNames.Contains("Entities")) UI.TreeViewExpandNode(_projectTreeViewId, _entitiesNodeId);
                else UI.TreeViewCollapseNode(_projectTreeViewId, _entitiesNodeId);
            }
            else
            {
                UI.TreeViewExpandNode(_projectTreeViewId, _assetsNodeId);
                UI.TreeViewExpandNode(_projectTreeViewId, _scenesNodeId);
                UI.TreeViewExpandNode(_projectTreeViewId, _scriptsNodeId);
                UI.TreeViewExpandNode(_projectTreeViewId, _entitiesNodeId);
                _expandedNodeNames.Add("Assets");
                _expandedNodeNames.Add("Scenes");
                _expandedNodeNames.Add("Scripts");
                _expandedNodeNames.Add("Entities");
            }
        }

        // === Asset Grid View UI Creation ===

        private static void CreateAssetGridView()
        {
            // Legacy method - now assets are in TabWidget
            // This method is kept for backward compatibility but creates in the model tab
            if (_assetModelGridViewId != 0) return;
            
            _assetModelGridViewId = UI.CreateGridView(_assetPanel.Id, 10, 40, 200f, 150f, 64);
            RefreshAssetGridView();
            UI.GridViewSetOnClick(_assetModelGridViewId, _gridViewClickCallback);
        }

        // === Toolbar Menus UI Creation ===

        private static void CreateToolbarMenus()
        {
            _fileMenuId = UI.CreatePopupMenu(0);
            UI.PopupMenuAddItem(_fileMenuId, "新建场景", "Ctrl+N", 1);
            UI.PopupMenuAddItem(_fileMenuId, "新建脚本", "", 2);
            UI.PopupMenuAddSeparator(_fileMenuId);
            UI.PopupMenuAddItem(_fileMenuId, "退出", "", 3);
            UI.PopupMenuSetOnClick(_fileMenuId, _fileMenuClickCallback);
            UI.SetWidgetLayer(_fileMenuId, 2);
            
            _openMenuId = UI.CreatePopupMenu(0);
            UI.PopupMenuAddItem(_openMenuId, "打开场景", "", 1);
            UI.PopupMenuAddItem(_openMenuId, "打开项目", "", 2);
            UI.PopupMenuAddItem(_openMenuId, "打开资源", "", 3);
            UI.PopupMenuSetOnClick(_openMenuId, _openMenuClickCallback);
            UI.SetWidgetLayer(_openMenuId, 2);
            
            _saveMenuId = UI.CreatePopupMenu(0);
            UI.PopupMenuAddItem(_saveMenuId, "保存场景", "Ctrl+S", 1);
            UI.PopupMenuAddItem(_saveMenuId, "保存全部", "", 2);
            UI.PopupMenuAddItem(_saveMenuId, "另存为...", "", 3);
            UI.PopupMenuSetOnClick(_saveMenuId, _saveMenuClickCallback);
            UI.SetWidgetLayer(_saveMenuId, 2);
        }

        // === Directory Tree UI Creation ===

        private static void RefreshDirectoryTree()
        {
            // Determine parent panel: use scriptEditorPanel when in script editor mode,
            // otherwise use projectPanel (main interface)
            ulong parentPanelId = 0;
            if (_scriptEditorVisible && _scriptEditorTreePanel != null)
            {
                parentPanelId = _scriptEditorTreePanel.Id;  // TreeView goes into the SplitView's left pane
            }
            else if (_projectPanel != null)
            {
                parentPanelId = _projectPanel.Id;
            }
            
            if (parentPanelId == 0) return;
            
            if (_directoryTreeViewId != 0)
            {
                UI.RemoveWidget(_directoryTreeViewId);
                _directoryTreeViewId = 0;
            }
            
            _fileItemPaths.Clear();
            _dirItemPaths.Clear();
            
            float mainHeight = _screenHeight - TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT;
            // In script editor mode, TreeView is inside treePanel (SplitView handles sizing)
            // In main interface mode, TreeView has explicit size
            float treeHeight = _scriptEditorVisible ? mainHeight - 40f : mainHeight - 50f;
            _directoryTreeViewId = UI.CreateTreeView(parentPanelId, 10, 10, 200f, treeHeight);
            
            _directoryRootNodeId = UI.TreeViewAddNode(_directoryTreeViewId, 0, "📁 " + _currentDirectory, 0, true);
            
            if (_currentDirectory != "scripts" && Directory.GetParent(_currentDirectory) != null)
            {
                _directoryBackNodeId = UI.TreeViewAddNode(_directoryTreeViewId, _directoryRootNodeId, "⬆️ 返回上级", 0, false);
                _dirItemPaths[_directoryBackNodeId] = Directory.GetParent(_currentDirectory).FullName;
            }
            
            try
            {
                if (Directory.Exists(_currentDirectory))
                {
                    AddDirectoryItems(_directoryTreeViewId, _directoryRootNodeId, _currentDirectory);
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", "reading directory: " + ex.Message);
            }
            
            UI.TreeViewSetOnSelect(_directoryTreeViewId, _treeNodeSelectCallback);
            UI.TreeViewSetOnToggle(_directoryTreeViewId, _treeNodeToggleCallback);
            UI.TreeViewExpandNode(_directoryTreeViewId, _directoryRootNodeId);
        }
        
        private static void AddDirectoryItems(ulong treeViewId, ulong parentNodeId, string path)
        {
            try
            {
                string[] dirs = Directory.GetDirectories(path);
                foreach (string dir in dirs)
                {
                    string name = Path.GetFileName(dir);
                    ulong nodeId = UI.TreeViewAddNode(treeViewId, parentNodeId, "📁 " + name, 0, true);
                    _dirItemPaths[nodeId] = dir;
                    
                    AddDirectoryItems(treeViewId, nodeId, dir);
                }
                
                string[] files = Directory.GetFiles(path);
                foreach (string file in files)
                {
                    if (file.EndsWith(".cs") || file.EndsWith(".txt") || file.EndsWith(".json"))
                    {
                        string name = Path.GetFileName(file);
                        ulong nodeId = UI.TreeViewAddNode(treeViewId, parentNodeId, "📄 " + name, 0, false);
                        _fileItemPaths[nodeId] = file;
                    }
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", ex.Message);
            }
        }
    }
}