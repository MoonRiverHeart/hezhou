using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace Hezhou
{
    /// <summary>
    /// UITestRunner 的自动化报告扩展：
    /// - 从环境变量启动无人值守测试
    /// - 采集帧时间/FPS/卡顿
    /// - 输出 report.json / summary.txt / ui_tree.txt / final.png
    /// - 写 autotest/DONE 供 Rust 主循环自动退出
    /// </summary>
    public static partial class UITestRunner
    {
        private static bool _autoMode = false;
        private static bool _autoStartPending = false;
        private static int _autoStartDelayFrames = 0;
        private static string _reportDir = "autotest";
        private static int _randomSeed = 0;
        private static bool _hasRandomSeed = false;
        private static int _screenshotEvery = 10;
        private static int _envMaxSteps = 50;
        private static float _envDuration = 15f;
        private static float _envInterval = 0.2f;
        private static string _envMode = "random";

        private static long _metricFrameCount = 0;
        private static double _metricFrameTimeSumMs = 0;
        private static float _metricMinFrameMs = float.MaxValue;
        private static float _metricMaxFrameMs = 0;
        private static float _metricMaxGapMs = 0;
        private static float _metricLastFrameMs = 0;
        private static int _metricFreezeCount = 0;
        private static int _metricWarmupFrames = 0;
        private static DateTime _metricStartedAt;

        public static bool IsAutoMode { get { return _autoMode; } }

        /// <summary>
        /// 由 EditorScript.Initialize() 在 UI 布局创建完成后调用。
        /// 读取 HEZHOU_AUTOTEST* 环境变量；未启用时完全无副作用。
        /// </summary>
        public static void RequestAutoRunFromEnvironment()
        {
            string enabled = Environment.GetEnvironmentVariable("HEZHOU_AUTOTEST");
            if (enabled != "1" && !string.Equals(enabled, "true", StringComparison.OrdinalIgnoreCase))
            {
                return;
            }

            _autoMode = true;
            _reportDir = GetEnv("HEZHOU_AUTOTEST_DIR", "autotest");
            _screenshotEvery = ParseIntEnv("HEZHOU_AUTOTEST_SCREENSHOT_EVERY", 10);
            _envMaxSteps = ParseIntEnv("HEZHOU_AUTOTEST_STEPS", 50);
            _envDuration = ParseFloatEnv("HEZHOU_AUTOTEST_DURATION", 15f);
            _envInterval = ParseFloatEnv("HEZHOU_AUTOTEST_INTERVAL", 0.2f);
            _envMode = GetEnv("HEZHOU_AUTOTEST_MODE", "random");

            string seedText = Environment.GetEnvironmentVariable("HEZHOU_AUTOTEST_SEED");
            int seed;
            if (!string.IsNullOrEmpty(seedText) && int.TryParse(seedText, out seed))
            {
                _randomSeed = seed;
                _hasRandomSeed = true;
            }

            _autoStartPending = true;
            _autoStartDelayFrames = 2;
            Log.Info("UITest", "Auto test requested: mode=" + _envMode + " steps=" + _envMaxSteps + " duration=" + _envDuration + "s interval=" + _envInterval + "s dir=" + _reportDir);
        }

        private static void UpdateAutoStart(float deltaTime)
        {
            if (_autoStartDelayFrames > 0)
            {
                _autoStartDelayFrames--;
                return;
            }

            _autoStartPending = false;
            TestTelemetry.Clear();

            if (string.Equals(_envMode, "intent", StringComparison.OrdinalIgnoreCase))
            {
                _mode = TestMode.IntentTest;
                _running = true;
                BeginAutoTestMetrics();
                IntentTestRunner.RunAll();
                CompleteTest();
                return;
            }

            if (string.Equals(_envMode, "visual", StringComparison.OrdinalIgnoreCase))
            {
                _mode = TestMode.VisualCheck;
                _running = true;
                BeginAutoTestMetrics();
                BeginVisualCheck();
                return;
            }

            TestMode mode = TestMode.RandomTraversal;
            if (string.Equals(_envMode, "sequential", StringComparison.OrdinalIgnoreCase))
            {
                mode = TestMode.SequentialTraversal;
            }
            else if (string.Equals(_envMode, "stress", StringComparison.OrdinalIgnoreCase))
            {
                mode = TestMode.StressTest;
            }

            // 压测模式只受 duration 限制，不受 steps 上限影响。
            int steps = mode == TestMode.StressTest ? 999999 : _envMaxSteps;
            StartTest(mode, steps, _envDuration, _envInterval);
        }

        private static Random CreateRandom()
        {
            if (_hasRandomSeed)
            {
                return new Random(_randomSeed);
            }
            return new Random();
        }

        private static void BeginAutoTestMetrics()
        {
            _metricStartedAt = DateTime.Now;
            _metricFrameCount = 0;
            _metricFrameTimeSumMs = 0;
            _metricMinFrameMs = float.MaxValue;
            _metricMaxFrameMs = 0;
            _metricMaxGapMs = 0;
            _metricLastFrameMs = 0;
            _metricFreezeCount = 0;
            // 启动测试时 UI 树遍历/首次点击会有一次布局尖峰，前几帧不计入性能统计。
            _metricWarmupFrames = 5;
        }

        private static void CollectFrameMetrics(float deltaTimeMs)
        {
            if (deltaTimeMs <= 0)
            {
                return;
            }

            if (_metricWarmupFrames > 0)
            {
                _metricWarmupFrames--;
                _metricLastFrameMs = deltaTimeMs;
                return;
            }

            _metricFrameCount++;
            _metricFrameTimeSumMs += deltaTimeMs;

            if (deltaTimeMs < _metricMinFrameMs) _metricMinFrameMs = deltaTimeMs;
            if (deltaTimeMs > _metricMaxFrameMs) _metricMaxFrameMs = deltaTimeMs;

            if (_metricLastFrameMs > 0)
            {
                float gap = deltaTimeMs - _metricLastFrameMs;
                if (gap > _metricMaxGapMs) _metricMaxGapMs = gap;
            }

            // 单帧超过 250ms 视为一次明显卡顿；超过 3s 由 Update 中 freeze 逻辑记录。
            if (deltaTimeMs > 250f)
            {
                _metricFreezeCount++;
            }

            _metricLastFrameMs = deltaTimeMs;
        }

        private static void FinishAutoTest()
        {
            // 卡顿次数作为性能指标记录；真正导致失败的是步骤失败/未捕获错误/致命错误。
            // >3s 无响应会由 Update 中的 freeze 逻辑生成失败步骤。
            bool pass = _failedCount == 0 && TestTelemetry.ErrorCount == 0 && TestTelemetry.FatalCount == 0;

            try
            {
                Directory.CreateDirectory(_reportDir);
                string finalScreenshot = Path.Combine(_reportDir, "final.png").Replace('\\', '/');
                if (UI.CaptureScreenshotToFile(finalScreenshot) != 0)
                {
                    finalScreenshot = "";
                }

                string uiTree = UI.DebugDumpTree();
                File.WriteAllText(Path.Combine(_reportDir, "ui_tree.txt"), uiTree ?? "");

                string reportJson = BuildAutoReportJson(finalScreenshot, uiTree);
                File.WriteAllText(Path.Combine(_reportDir, "report.json"), reportJson);

                string summary = BuildAutoSummary(pass);
                File.WriteAllText(Path.Combine(_reportDir, "summary.txt"), summary);

                File.WriteAllText(Path.Combine(_reportDir, "DONE"), pass ? "pass" : "fail");
                Log.Info("UITest", "Auto test finished: " + (pass ? "PASS" : "FAIL") + ", report=" + Path.Combine(_reportDir, "report.json"));
            }
            catch (Exception ex)
            {
                TestTelemetry.RecordError("UITest", "Failed to write auto report: " + ex.Message);
                try
                {
                    Directory.CreateDirectory(_reportDir);
                    File.WriteAllText(Path.Combine(_reportDir, "DONE"), "fail");
                }
                catch { }
                Log.Error("UITest", "Failed to write auto report: " + ex.Message);
            }
        }

        private static string BuildAutoSummary(bool pass)
        {
            StringBuilder sb = new StringBuilder();
            sb.AppendLine("HZ_AUTOTEST " + (pass ? "PASS" : "FAIL"));
            sb.AppendLine("mode=" + ModeToString(_mode));
            sb.AppendLine("seed=" + (_hasRandomSeed ? _randomSeed.ToString() : "random"));
            sb.AppendLine("steps=" + _steps.Count + " passed=" + _passedCount + " failed=" + _failedCount);
            sb.AppendLine("elapsed=" + _elapsedTime.ToString("F2") + "s");
            sb.AppendLine("frames=" + _metricFrameCount);
            sb.AppendLine("avg_frame_ms=" + AverageFrameMs().ToString("F2"));
            sb.AppendLine("min_frame_ms=" + MinFrameMs().ToString("F2"));
            sb.AppendLine("max_frame_ms=" + _metricMaxFrameMs.ToString("F2"));
            sb.AppendLine("max_frame_gap_ms=" + _metricMaxGapMs.ToString("F2"));
            sb.AppendLine("stalls_over_250ms=" + _metricFreezeCount);
            sb.AppendLine("telemetry: " + TestTelemetry.BuildSummary());
            sb.AppendLine("report=report.json");
            sb.AppendLine("ui_tree=ui_tree.txt");
            sb.AppendLine("screenshot=final.png");
            return sb.ToString();
        }

        private static string BuildAutoReportJson(string finalScreenshot, string uiTree)
        {
            StringBuilder sb = new StringBuilder();
            sb.Append("{");
            sb.Append("\"schema\":\"hezhou.autotest.v1\",");
            sb.Append("\"mode\":").Append(TestTelemetry.JsonString(ModeToString(_mode))).Append(",");
            sb.Append("\"seed\":").Append(_hasRandomSeed ? _randomSeed.ToString() : "null").Append(",");
            sb.Append("\"started_at\":").Append(TestTelemetry.JsonString(_metricStartedAt.ToString("yyyy-MM-dd HH:mm:ss"))).Append(",");
            sb.Append("\"elapsed_sec\":").Append(_elapsedTime.ToString("F3")).Append(",");
            sb.Append("\"steps_total\":").Append(_steps.Count).Append(",");
            sb.Append("\"steps_passed\":").Append(_passedCount).Append(",");
            sb.Append("\"steps_failed\":").Append(_failedCount).Append(",");
            sb.Append("\"metrics\":{");
            sb.Append("\"frames\":").Append(_metricFrameCount).Append(",");
            sb.Append("\"avg_frame_ms\":").Append(AverageFrameMs().ToString("F3")).Append(",");
            sb.Append("\"min_frame_ms\":").Append(MinFrameMs().ToString("F3")).Append(",");
            sb.Append("\"max_frame_ms\":").Append(_metricMaxFrameMs.ToString("F3")).Append(",");
            sb.Append("\"max_frame_gap_ms\":").Append(_metricMaxGapMs.ToString("F3")).Append(",");
            sb.Append("\"stalls_over_250ms\":").Append(_metricFreezeCount).Append(",");
            sb.Append("\"ui_tree_chars\":").Append(uiTree != null ? uiTree.Length : 0);
            sb.Append("},");
            sb.Append("\"telemetry\":{");
            sb.Append("\"warnings\":").Append(TestTelemetry.WarningCount).Append(",");
            sb.Append("\"errors\":").Append(TestTelemetry.ErrorCount).Append(",");
            sb.Append("\"fatal\":").Append(TestTelemetry.FatalCount);
            sb.Append("},");
            sb.Append("\"ui_tree\":\"ui_tree.txt\",");
            sb.Append("\"final_screenshot\":").Append(TestTelemetry.JsonString(finalScreenshot)).Append(",");
            sb.Append("\"trace_dir\":\"traces\",");
            sb.Append("\"errors\":").Append(TestTelemetry.ToJsonArray()).Append(",");
            sb.Append("\"steps\":[");
            for (int i = 0; i < _steps.Count; i++)
            {
                if (i > 0) sb.Append(",");
                TestStep step = _steps[i];
                sb.Append("{");
                sb.Append("\"n\":").Append(step.StepNumber).Append(",");
                sb.Append("\"widget_id\":").Append(step.WidgetId).Append(",");
                sb.Append("\"type\":").Append(TestTelemetry.JsonString(step.WidgetType)).Append(",");
                sb.Append("\"path\":").Append(TestTelemetry.JsonString(step.WidgetPath)).Append(",");
                sb.Append("\"x\":").Append(step.ClickX.ToString("F2")).Append(",");
                sb.Append("\"y\":").Append(step.ClickY.ToString("F2")).Append(",");
                sb.Append("\"success\":").Append(step.Success ? "true" : "false").Append(",");
                sb.Append("\"error\":").Append(TestTelemetry.JsonString(step.ErrorMessage)).Append(",");
                sb.Append("\"screenshot\":").Append(TestTelemetry.JsonString(step.ScreenshotPath));
                sb.Append("}");
            }
            sb.Append("]");
            sb.Append("}");
            return sb.ToString();
        }

        private static double AverageFrameMs()
        {
            if (_metricFrameCount == 0) return 0;
            return _metricFrameTimeSumMs / _metricFrameCount;
        }

        private static float MinFrameMs()
        {
            return _metricMinFrameMs == float.MaxValue ? 0 : _metricMinFrameMs;
        }

        private static string GetEnv(string name, string fallback)
        {
            string value = Environment.GetEnvironmentVariable(name);
            return string.IsNullOrEmpty(value) ? fallback : value;
        }

        private static int ParseIntEnv(string name, int fallback)
        {
            string value = Environment.GetEnvironmentVariable(name);
            int parsed;
            if (!string.IsNullOrEmpty(value) && int.TryParse(value, out parsed))
            {
                return parsed;
            }
            return fallback;
        }

        private static float ParseFloatEnv(string name, float fallback)
        {
            string value = Environment.GetEnvironmentVariable(name);
            float parsed;
            if (!string.IsNullOrEmpty(value) && float.TryParse(value, out parsed))
            {
                return parsed;
            }
            return fallback;
        }

        // === Visual Check ===

        private class VisualStep
        {
            public string Name;
            public Action Setup;
            public int WaitFrames;

            public VisualStep(string name, Action setup)
                : this(name, setup, 3)
            {
            }

            public VisualStep(string name, Action setup, int waitFrames)
            {
                Name = name;
                Setup = setup;
                WaitFrames = waitFrames;
            }
        }

        private static List<VisualStep> _visualSteps;
        private static int _visualIndex = 0;
        private static int _visualWaitFrames = 0;
        private static string _visualDir = "autotest/visual";

        private static void BeginVisualCheck()
        {
            _visualDir = Path.Combine(_reportDir, "visual");
            Directory.CreateDirectory(_visualDir);
            _visualIndex = 0;
            _visualWaitFrames = 0;

            _visualSteps = new List<VisualStep>();
            // 第一步等待 hot reload 完成（mcs + reload 约 1s，留足 3s）。
            _visualSteps.Add(new VisualStep("01_rotation_script_binding", VisualRotationBindingViaUI, 180));
            _visualSteps.Add(new VisualStep("02_console", VisualConsole));
            _visualSteps.Add(new VisualStep("03_asset_search", VisualAssetSearch));
            _visualSteps.Add(new VisualStep("04_project_search", VisualProjectSearch));
            _visualSteps.Add(new VisualStep("05_multi_select", VisualMultiSelect));
            _visualSteps.Add(new VisualStep("06_theme_light", VisualLightTheme, 20));
            _visualSteps.Add(new VisualStep("07_theme_dark", VisualDarkTheme, 20));
            _visualSteps.Add(new VisualStep("08_widget_gallery_dark", VisualGalleryDark, 10));
            _visualSteps.Add(new VisualStep("09_widget_gallery_light", VisualGalleryLight, 10));
            _visualSteps.Add(new VisualStep("10_editor_restored", VisualGalleryRestore, 10));
            _visualSteps.Add(new VisualStep("11_right_panel_max", VisualRightPanelMax, 10));
            _visualSteps.Add(new VisualStep("12_dock_restored", VisualDockRestore, 10));
            _visualSteps.Add(new VisualStep("13_shift_range_select", VisualShiftRangeSelect, 10));
            _visualSteps.Add(new VisualStep("14_command_palette", VisualCommandPalette, 10));
            _visualSteps.Add(new VisualStep("15_palette_closed", VisualCommandPaletteClose, 10));
            _visualSteps.Add(new VisualStep("16_inspector_rows", VisualInspectorRows, 10));
            _visualSteps.Add(new VisualStep("17_hierarchy_drag", VisualHierarchyDrag, 10));
            _visualSteps.Add(new VisualStep("18_material_tab", VisualMaterialTab, 10));
            _visualSteps.Add(new VisualStep("19_gizmo_translate", VisualGizmoTranslate, 10));
            _visualSteps.Add(new VisualStep("20_material_assign", VisualMaterialAssign, 10));
            _visualSteps.Add(new VisualStep("21_gizmo_scale", VisualGizmoScale, 10));
            _visualSteps.Add(new VisualStep("22_tab_reorder", VisualTabReorder, 10));
            _visualSteps.Add(new VisualStep("23_asset_collapsed", VisualAssetCollapsed, 10));
            _visualSteps.Add(new VisualStep("24_dock_restored_2", VisualDockRestored2, 10));
            _visualSteps.Add(new VisualStep("25_floating_panel", VisualFloatingPanel, 10));
            _visualSteps.Add(new VisualStep("26_floating_moved", VisualFloatingMoved, 10));
            _visualSteps.Add(new VisualStep("27_floating_closed", VisualFloatingClosed, 10));

            RunNextVisualStep();
        }

        private static void VisualRotationBindingViaUI()
        {
            EditorScript.TestNewScene();
            ulong entity = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSelectEntity(entity);
            EditorScript.TestSetPropertyTab(3);
            EditorScript.TestSelectScript("RotationScript.cs");
            EditorScript.TestClickBindButton();
        }

        private static void VisualConsole()
        {
            EditorScript.TestOpenConsole();
        }

        private static void VisualAssetSearch()
        {
            EditorScript.TestCloseConsole();
            EditorScript.TestSetAssetSearch("Cube");
        }

        private static void VisualProjectSearch()
        {
            EditorScript.TestSetAssetSearch("");
            EditorScript.TestSetProjectSearch("Cube");
        }

        private static void VisualLightTheme()
        {
            // 走真实工具栏按钮路径，而不是直接调 SetTheme。
            EditorScript.TestClickThemeButton();
        }

        private static void VisualDarkTheme()
        {
            EditorScript.TestClickThemeButton();
        }

        private static void VisualGalleryDark()
        {
            EditorScript.TestShowWidgetGallery();
        }

        private static void VisualGalleryLight()
        {
            EditorScript.TestClickThemeButton();
            EditorScript.TestShowWidgetGallery();
        }

        private static void VisualGalleryRestore()
        {
            EditorScript.TestHideWidgetGallery();
            EditorScript.TestSetTheme(ThemeMode.Dark);
        }

        private static void VisualRightPanelMax()
        {
            EditorScript.TestToggleRightPanelMax();
        }

        private static void VisualDockRestore()
        {
            EditorScript.TestToggleRightPanelMax();
        }

        private static void VisualShiftRangeSelect()
        {
            EditorScript.TestNewScene();
            ulong a = EditorScript.TestCreateMeshEntity(0);
            ulong b = EditorScript.TestCreateMeshEntity(0);
            ulong c = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSetPosition(a, -2f, 0.5f, 0f);
            EditorScript.TestSetPosition(b, 0f, 0.5f, 0f);
            EditorScript.TestSetPosition(c, 2f, 0.5f, 0f);
            EditorScript.TestSelectEntity(a);
            EditorScript.TestShiftRangeSelect(a, c);
        }

        private static void VisualHierarchyDrag()
        {
            EditorScript.TestNewScene();
            ulong parent = EditorScript.TestCreateMeshEntity(0);
            ulong child = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestHierarchyDrop(child, parent);
            EditorScript.TestExpandHierarchy();
        }

        private static void VisualFloatingPanel()
        {
            EditorScript.TestShowFloatingPanel();
        }

        private static void VisualFloatingMoved()
        {
            EditorScript.TestMoveFloatingPanel(80f, 40f);
        }

        private static void VisualFloatingClosed()
        {
            EditorScript.TestHideFloatingPanel();
        }

        private static void VisualAssetCollapsed()
        {
            EditorScript.TestToggleAssetPanel();
        }

        private static void VisualDockRestored2()
        {
            EditorScript.TestToggleAssetPanel();
        }

        private static void VisualTabReorder()
        {
            EditorScript.TestSelectAssetTab(1);
            EditorScript.TestMoveAssetTab(1, 0);
        }

        private static void VisualGizmoScale()
        {
            EditorScript.TestNewScene();
            ulong cube = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSelectEntity(cube);
            EditorScript.TestGizmoCycleMode();
            EditorScript.TestGizmoScale(1.6f);
        }

        private static void VisualMaterialAssign()
        {
            EditorScript.TestNewScene();
            ulong cube = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSelectEntity(cube);
            EditorScript.TestAssignMaterial("Default Material");
        }

        private static void VisualGizmoTranslate()
        {
            EditorScript.TestNewScene();
            ulong cube = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSelectEntity(cube);
            EditorScript.TestGizmoNudge(1.5f, 0.8f);
        }

        private static void VisualMaterialTab()
        {
            EditorScript.TestSelectAssetTab(1);
        }

        private static void VisualInspectorRows()
        {
            EditorScript.TestNewScene();
            ulong cube = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSelectEntity(cube);
            EditorScript.TestSetPropertyTab(1);
        }

        private static void VisualCommandPalette()
        {
            EditorScript.TestShowCommandPalette();
        }

        private static void VisualCommandPaletteClose()
        {
            EditorScript.TestHideCommandPalette();
        }

        private static void VisualMultiSelect()
        {
            EditorScript.TestSetProjectSearch("");
            EditorScript.TestNewScene();
            ulong first = EditorScript.TestCreateMeshEntity(0);
            ulong second = EditorScript.TestCreateMeshEntity(0);
            EditorScript.TestSetPosition(first, -2f, 0.5f, 0f);
            EditorScript.TestSetPosition(second, 2f, 0.5f, 0f);
            EditorScript.TestSelectEntity(first);
            EditorScript.TestToggleEntitySelection(second);
        }

        private static void RunNextVisualStep()
        {
            if (_visualSteps == null || _visualIndex >= _visualSteps.Count)
            {
                CompleteTest();
                return;
            }

            _visualSteps[_visualIndex].Setup();
            _visualWaitFrames = _visualSteps[_visualIndex].WaitFrames;
        }

        private static void UpdateVisualCheck()
        {
            if (_visualWaitFrames > 0)
            {
                _visualWaitFrames--;
                return;
            }

            string name = _visualSteps[_visualIndex].Name;
            string path = Path.Combine(_visualDir, name + ".png");
            int result = UI.CaptureScreenshotToFile(path);

            TestStep step = new TestStep();
            step.StepNumber = _steps.Count + 1;
            step.WidgetType = "Visual";
            step.WidgetPath = name;
            step.Success = result == 0;
            step.ErrorMessage = result == 0 ? "" : "screenshot failed: code " + result;
            step.ScreenshotPath = result == 0 ? path : "";
            _steps.Add(step);
            if (step.Success) _passedCount++;
            else
            {
                _failedCount++;
                TestTelemetry.RecordError("VisualCheck", step.ErrorMessage);
            }

            _visualIndex++;
            if (_visualIndex >= _visualSteps.Count)
            {
                CompleteTest();
            }
            else
            {
                RunNextVisualStep();
            }
        }
    }
}
