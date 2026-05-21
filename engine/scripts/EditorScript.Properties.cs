using System;
using System.IO;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static void UpdatePropertiesPanel(ulong entityId)
        {
            if (_propsList == null || _gameScene == null) return;
            
            if (!_propertiesDirty && _lastPropertiesEntityId == entityId)
            {
                return;
            }
            
            _propertiesDirty = false;
            _lastPropertiesEntityId = entityId;
            
            _selectedEntityId = entityId;
            
            string name = UI.SceneGetEntityName(_gameScene.ScenePtr, entityId);
            UI.InputFieldSetText(_nameInputFieldId, name);
            
            float px, py, pz;
            UI.SceneGetEntityPosition(_gameScene.ScenePtr, entityId, out px, out py, out pz);
            UI.InputFieldSetText(_posXInputFieldId, px.ToString("F2"));
            UI.InputFieldSetText(_posYInputFieldId, py.ToString("F2"));
            UI.InputFieldSetText(_posZInputFieldId, pz.ToString("F2"));
            
            float rx, ry, rz, rw;
            UI.SceneGetEntityRotation(_gameScene.ScenePtr, entityId, out rx, out ry, out rz, out rw);
            float eulerX = (float)Math.Atan2(2.0 * (rw * rx + ry * rz), 1.0 - 2.0 * (rx * rx + ry * ry)) * 180.0f / (float)Math.PI;
            float eulerY = (float)Math.Asin(2.0 * (rw * ry - rz * rx)) * 180.0f / (float)Math.PI;
            float eulerZ = (float)Math.Atan2(2.0 * (rw * rz + rx * ry), 1.0 - 2.0 * (ry * ry + rz * rz)) * 180.0f / (float)Math.PI;
            UI.InputFieldSetText(_rotXInputFieldId, eulerX.ToString("F0"));
            UI.InputFieldSetText(_rotYInputFieldId, eulerY.ToString("F0"));
            UI.InputFieldSetText(_rotZInputFieldId, eulerZ.ToString("F0"));
            
            float sx, sy, sz;
            UI.SceneGetEntityScale(_gameScene.ScenePtr, entityId, out sx, out sy, out sz);
            UI.InputFieldSetText(_scaleXInputFieldId, sx.ToString("F2"));
            UI.InputFieldSetText(_scaleYInputFieldId, sy.ToString("F2"));
            UI.InputFieldSetText(_scaleZInputFieldId, sz.ToString("F2"));
            
            UpdateScriptBindingsList(entityId);
            
            Log.Info("Editor", $"Properties panel updated for entity {entityId}: pos=({px}, {py}, {pz})");
        }
        
        private static void UpdateScriptBindingsList(ulong entityId)
        {
            if (_gameScene == null || _scriptsListContainerId == 0) return;
            
            int scriptCount = _gameScene.GetScriptBindingCount(entityId);
            
            if (scriptCount == _lastScriptBindingCount && _lastPropertiesEntityId == entityId)
            {
                return;
            }
            
            _lastScriptBindingCount = scriptCount;
            _removeScriptBtnIndices.Clear();
            
            if (scriptCount == 0)
            {
                Log.Info("Editor", "UpdateScriptBindingsList: no scripts attached");
                return;
            }
            
            Log.Info("Editor", $"UpdateScriptBindingsList: entityId={entityId}, count={scriptCount}");
            
            for (int i = 0; i < scriptCount; i++)
            {
                var info = _gameScene.GetScriptBindingInfo(entityId, i);
                Log.Info("Editor", $"  Script[{i}]: path={info.ScriptPath}, class={info.ClassName}, enabled={info.Enabled}");
                
                string scriptName = Path.GetFileName(info.ScriptPath);
                string labelText = $"{scriptName} ({info.ClassName}) [{(info.Enabled ? "ON" : "OFF")}]";
                
                var scriptRow = UI.CreateHStack(_scriptsListContainerId, 5f);
                UI.CreateLabel(scriptRow, RIGHT_PANEL_WIDTH - 90f, 20f, labelText);
                
                ulong removeBtnId = UI.CreateButton(scriptRow, 40f, 20f, "X");
                UI.SetOnClick(removeBtnId, _removeScriptClickCallback);
                _removeScriptBtnIndices[removeBtnId] = i;
            }
        }
        
        private static void ClearPropertiesPanel()
        {
            if (_propsList == null) return;
            
            _selectedEntityId = 0;
            UI.InputFieldSetText(_nameInputFieldId, "");
            UI.InputFieldSetText(_posXInputFieldId, "0");
            UI.InputFieldSetText(_posYInputFieldId, "0");
            UI.InputFieldSetText(_posZInputFieldId, "0");
            UI.InputFieldSetText(_rotXInputFieldId, "0");
            UI.InputFieldSetText(_rotYInputFieldId, "0");
            UI.InputFieldSetText(_rotZInputFieldId, "0");
            UI.InputFieldSetText(_scaleXInputFieldId, "1");
            UI.InputFieldSetText(_scaleYInputFieldId, "1");
            UI.InputFieldSetText(_scaleZInputFieldId, "1");
            
            Log.Info("Editor", "Properties panel cleared");
        }
        
        private static void OnNameInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            UI.SceneSetEntityName(_gameScene.ScenePtr, _selectedEntityId, text);
            UpdateEntityNameInTree(_selectedEntityId, text);
            _propertiesDirty = true;
            Log.Info("Editor", $"Entity name changed to: {text}");
        }
        
        private static void OnPosXInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float x))
            {
                float px, py, pz;
                UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, out px, out py, out pz);
                UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, x, py, pz);
                Log.Info("Editor", $"Entity position X changed to: {x}");
            }
        }
        
        private static void OnPosYInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float y))
            {
                float px, py, pz;
                UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, out px, out py, out pz);
                UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, px, y, pz);
                Log.Info("Editor", $"Entity position Y changed to: {y}");
            }
        }
        
        private static void OnPosZInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float z))
            {
                float px, py, pz;
                UI.SceneGetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, out px, out py, out pz);
                UI.SceneSetEntityPosition(_gameScene.ScenePtr, _selectedEntityId, px, py, z);
                Log.Info("Editor", $"Entity position Z changed to: {z}");
            }
        }
        
        private static void OnRotXInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float deg))
            {
                Log.Info("Editor", $"Entity rotation X changed to: {deg}°");
            }
        }
        
        private static void OnRotYInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float deg))
            {
                Log.Info("Editor", $"Entity rotation Y changed to: {deg}°");
            }
        }
        
        private static void OnRotZInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float deg))
            {
                UI.SceneRotateEntity(_gameScene.ScenePtr, _selectedEntityId, deg);
                Log.Info("Editor", $"Entity rotation Z changed to: {deg}°");
            }
        }
        
        private static void OnScaleXInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float x))
            {
                float sx, sy, sz;
                UI.SceneGetEntityScale(_gameScene.ScenePtr, _selectedEntityId, out sx, out sy, out sz);
                UI.SceneSetEntityScale(_gameScene.ScenePtr, _selectedEntityId, x, sy, sz);
                Log.Info("Editor", $"Entity scale X changed to: {x}");
            }
        }
        
        private static void OnScaleYInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float y))
            {
                float sx, sy, sz;
                UI.SceneGetEntityScale(_gameScene.ScenePtr, _selectedEntityId, out sx, out sy, out sz);
                UI.SceneSetEntityScale(_gameScene.ScenePtr, _selectedEntityId, sx, y, sz);
                Log.Info("Editor", $"Entity scale Y changed to: {y}");
            }
        }
        
        private static void OnScaleZInputChange(ulong widgetId, string text)
        {
            if (_selectedEntityId == 0 || _gameScene == null) return;
            if (float.TryParse(text, out float z))
            {
                float sx, sy, sz;
                UI.SceneGetEntityScale(_gameScene.ScenePtr, _selectedEntityId, out sx, out sy, out sz);
                UI.SceneSetEntityScale(_gameScene.ScenePtr, _selectedEntityId, sx, sy, z);
                Log.Info("Editor", $"Entity scale Z changed to: {z}");
            }
        }
    }
}