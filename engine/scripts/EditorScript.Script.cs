using System;
using System.IO;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static void ScanScripts()
        {
            _availableScripts.Clear();
            
            try
            {
                if (Directory.Exists("scripts"))
                {
                    string[] files = Directory.GetFiles("scripts", "*.cs");
                    foreach (string file in files)
                    {
                        string fileName = Path.GetFileName(file);
                        if (fileName != "UI.cs" && fileName != "DFX.cs" && fileName != "EditorScript.cs")
                        {
                            _availableScripts.Add(fileName);
                        }
                    }
                    
                    Log.Info("Editor", $"扫描scripts目录: 找到 {_availableScripts.Count} 个可用脚本");
                    foreach (var script in _availableScripts)
                    {
                        Log.Info("Editor", $"  - {script}");
                    }
                }
            }
            catch (Exception ex)
            {
                Log.Error("Editor", $"扫描scripts目录失败: {ex.Message}");
            }
        }

        private static void OnScriptDropdownSelect(ulong widgetId, ulong index)
        {
            _selectedScriptIndex = (int)index;
            int idx = (int)index;
            Log.Info("Editor", $"选择脚本: index={index}, script={(idx < _availableScripts.Count ? _availableScripts[idx] : "none")}");
        }
        
        private static void OnAddScriptClick(ulong widgetId)
        {
            Log.Info("Editor", "点击\"Add Script\"按钮");
            
            if (_selectedEntityId == 0 || _gameScene == null)
            {
                Log.Error("Editor", "未选中Entity!");
                return;
            }
            
            if (_availableScripts.Count == 0 || _selectedScriptIndex >= _availableScripts.Count)
            {
                Log.Error("Editor", "无可用脚本!");
                return;
            }
            
            string scriptName = _availableScripts[_selectedScriptIndex];
            string scriptPath = $"scripts/{scriptName}";
            string className = Path.GetFileNameWithoutExtension(scriptName);
            
            _gameScene.AttachScriptBinding(_selectedEntityId, scriptPath, className);
            Log.Info("Editor", $"添加脚本绑定: entityId={_selectedEntityId}, script={scriptPath}, class={className}");
            
            UpdatePropertiesPanel(_selectedEntityId);
        }
        
        private static void OnRemoveScriptClick(ulong widgetId)
        {
            Log.Info("Editor", $"点击\"Remove Script\"按钮: widgetId={widgetId}");
            
            if (_selectedEntityId == 0 || _gameScene == null)
            {
                Log.Error("Editor", "未选中Entity!");
                return;
            }
            
            if (_removeScriptBtnIndices.TryGetValue(widgetId, out int index))
            {
                _gameScene.RemoveScriptBinding(_selectedEntityId, index);
                Log.Info("Editor", $"移除脚本绑定: entityId={_selectedEntityId}, index={index}");
                
                UpdatePropertiesPanel(_selectedEntityId);
            }
        }
        
        private static void OnHotReloadComplete()
        {
            Log.Info("Editor", "=== OnHotReloadComplete回调 ===");
            
            ScanScripts();
            Log.Info("Editor", $"脚本扫描完成: {_availableScripts.Count} 个脚本");
            
            if (_scriptDropdownId != 0)
            {
                string[] scriptOptions = _availableScripts.Count > 0 ? _availableScripts.ToArray() : new string[] { "无可用脚本" };
                UI.DropdownSetOptions(_scriptDropdownId, scriptOptions);
                Log.Info("Editor", "脚本下拉菜单已刷新");
            }
            
            if (_statusItem != null)
            {
                _statusItem.Text = "状态: 就绪";
            }
            
            Log.Info("Editor", "OnHotReloadComplete完成");
        }
        
        private static void OnTabSelect(ulong widgetId, ulong index)
        {
            Log.Info("Editor", $"TabWidget选择: widgetId={widgetId}, index={index}");
            string tabName = index == 0 ? "Transform" : "Scripts";
            _statusItem.Text = $"属性页: {tabName}";
        }
        
        private static void OnNewScriptClick(ulong widgetId)
        {
            Log.Info("Editor", "创建新脚本...");
            ShowScriptEditor();
        }

        private static void OnHotReloadClick(ulong widgetId)
        {
            Log.Info("Editor", "=== Hot Reload按钮点击 ===");
            
            if (_statusItem != null)
            {
                _statusItem.Text = "正在保存脚本...";
            }
            
            if (_scriptTextEditId != 0)
            {
                try
                {
                    string scriptContent = UI.TextEditGetText(_scriptTextEditId);
                    string scriptPath = "scripts/bin/Mono/EditorScript.cs";
                    
                    System.IO.Directory.CreateDirectory("scripts/bin/Mono");
                    System.IO.File.WriteAllText(scriptPath, scriptContent);
                    Log.Info("Editor", $"✓ 脚本已保存: {scriptPath} ({scriptContent.Length} chars)");
                    
                    UI.SetStatusText("正在热更新脚本...");
                    UI.TriggerHotReload();
                    
                    Log.Info("Editor", "已触发Rust端HotReload流程");
                }
                catch (Exception ex)
                {
                    Log.Error("Editor", $"保存脚本失败: {ex.Message}");
                    if (_statusItem != null)
                    {
                        _statusItem.Text = $"保存失败: {ex.Message}";
                    }
                }
            }
            else
            {
                Log.Error("Editor", "TextEdit未创建");
                if (_statusItem != null)
                {
                    _statusItem.Text = "错误: 编辑器未初始化";
                }
            }
        }
    }
}