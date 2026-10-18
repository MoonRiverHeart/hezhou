using System;
using System.Collections.Generic;

namespace Hezhou
{
    public static partial class EditorScript
    {
        private static ulong _commandPanelId;
        private static ulong _commandInputId;
        private static UI.WidgetCallbackDelegate _commandClickCallback;
        private static UI.InputFieldChangeCallbackDelegate _commandInputCallback;
        private static readonly List<ulong> _commandButtons = new List<ulong>();
        private static readonly List<string> _commandLabels = new List<string>();
        private static readonly Dictionary<ulong, int> _commandActions = new Dictionary<ulong, int>();

        private static readonly string[] CommandNames = new string[]
        {
            "新建场景            Ctrl+N",
            "保存                Ctrl+S",
            "撤销                Ctrl+Z",
            "重做                Ctrl+Y",
            "运行 / 暂停",
            "打开 Console",
            "切换深浅主题",
            "控件画廊",
            "最大化属性面板",
            "重置 Dock 布局",
            "打开项目            Ctrl+O",
            "收起/展开项目面板",
            "收起/展开资产面板",
            "浮动资产面板",
            "纹理页停靠右侧/回到底部"
        };

        public static bool IsCommandPaletteOpen { get { return _commandPanelId != 0; } }

        private static void OnCommandPaletteInput(ulong widgetId, string text)
        {
            string query = (text ?? "").Trim().ToLowerInvariant();
            for (int i = 0; i < _commandButtons.Count; i++)
            {
                string label = _commandLabels[i].ToLowerInvariant();
                UI.SetWidgetVisible(_commandButtons[i], query.Length == 0 || label.Contains(query));
            }
        }

        private static void OnCommandClicked(ulong widgetId)
        {
            int action;
            if (!_commandActions.TryGetValue(widgetId, out action)) return;
            HideCommandPalette();
            ExecuteCommand(action);
        }

        private static void ExecuteCommand(int action)
        {
            switch (action)
            {
                case 0: NewScene(); break;
                case 1: SaveCurrentProject(); break;
                case 2: Undo(); break;
                case 3: Redo(); break;
                case 4:
                    if (_gameScene != null && _gameScene.GetGameState() == GameState.Running) OnPauseClick(0);
                    else OnRunClick(0);
                    break;
                case 5: OnConsoleClick(0); break;
                case 6: ApplyTheme(Theme.IsDark ? ThemeMode.Light : ThemeMode.Dark, true); break;
                case 7: ShowWidgetGallery(); break;
                case 8: OnRightMaxClick(0); break;
                case 9: ResetDockLayout(); break;
                case 10: ShowOpenProjectDialog(); break;
                case 11: ToggleProjectPanel(); break;
                case 12: ToggleAssetPanel(); break;
                case 13: ToggleFloatingAssetPanel(); break;
                case 14: ToggleTextureDock(); break;
            }
        }

        public static void ToggleCommandPalette()
        {
            if (_commandPanelId != 0) HideCommandPalette();
            else ShowCommandPalette();
        }

        public static void ShowCommandPalette()
        {
            if (_commandPanelId != 0) return;
            if (_commandClickCallback == null) _commandClickCallback = OnCommandClicked;
            if (_commandInputCallback == null) _commandInputCallback = OnCommandPaletteInput;

            ulong root = UI.GetRootId();
            _commandPanelId = UI.CreatePanel(root, 0f, 0f, _screenWidth, _screenHeight, ThemeRole.Overlay);
            UI.SetWidgetLayer(_commandPanelId, 3);

            float cardWidth = 560f;
            float cardHeight = 450f;
            float cardX = (_screenWidth - cardWidth) * 0.5f;
            float cardY = 110f;
            ulong card = UI.CreatePanel(_commandPanelId, cardX, cardY, cardWidth, cardHeight, ThemeRole.Elevated);

            UI.CreateLabel(card, 20f, 14f, 520f, 26f, "Command Palette  ·  Ctrl+Shift+P");
            _commandInputId = UI.CreateInputField(card, 520f, 30f);
            UI.SetWidgetLayout(_commandInputId, 20f, 48f, 520f, 30f);
            UI.InputFieldSetPlaceholder(_commandInputId, "输入命令...");
            UI.InputFieldSetOnChange(_commandInputId, _commandInputCallback);

            _commandButtons.Clear();
            _commandLabels.Clear();
            _commandActions.Clear();
            float y = 92f;
            for (int i = 0; i < CommandNames.Length; i++)
            {
                ulong button = UI.CreateButton(card, 520f, 30f, CommandNames[i]);
                UI.SetWidgetLayout(button, 20f, y, 520f, 30f);
                UI.SetOnClick(button, _commandClickCallback);
                UI.ThemeSetBackground(button, ThemeRole.ButtonSecondaryBg);
                UI.ThemeSetText(button, ThemeRole.TextPrimary);
                _commandButtons.Add(button);
                _commandLabels.Add(CommandNames[i]);
                _commandActions[button] = i;
                y += 32f;
            }
        }

        public static void HideCommandPalette()
        {
            if (_commandPanelId != 0)
            {
                UI.RemoveWidget(_commandPanelId);
                _commandPanelId = 0;
                _commandInputId = 0;
                _commandButtons.Clear();
                _commandLabels.Clear();
                _commandActions.Clear();
            }
        }

        public static void TestShowCommandPalette() { ShowCommandPalette(); }
        public static void TestHideCommandPalette() { HideCommandPalette(); }
    }
}
