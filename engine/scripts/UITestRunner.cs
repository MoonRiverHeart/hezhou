using System;
using System.Collections.Generic;

namespace Hezhou
{
    public enum TestMode
    {
        SequentialTraversal = 0,
        RandomTraversal = 1,
        StressTest = 2
    }

    public class TestStep
    {
        public int StepNumber;
        public ulong WidgetId;
        public string WidgetType;
        public float ClickX;
        public float ClickY;
        public string ScreenshotPath;
        public bool Success;
        public string ErrorMessage;
        public float[] Layout;
        public string WidgetPath;
    }

    public static class UITestRunner
    {
        // === Test State ===
        private static bool _running = false;
        private static bool _completed = false;
        private static TestMode _mode;
        private static List<TestStep> _steps = new List<TestStep>();
        private static List<ulong> _widgetIds = new List<ulong>();
        private static int _currentStepIndex = 0;
        private static int _targetSteps = 100;
        private static float _targetDuration = 30f;
        private static float _elapsedTime = 0f;
        private static int _passedCount = 0;
        private static int _failedCount = 0;

        // === Click Interval ===
        private static float _clickInterval = 0.5f; // seconds between clicks
        private static float _intervalTimer = 0f;

        // === No-Response Timeout ===
        private static float _lastFrameDelta = 0f; // detect engine freeze

        // === Report UI ===
        private static ulong _reportPanelId = 0;
        private static ulong _reportCloseBtnId = 0;
        private static UI.WidgetCallbackDelegate _closeReportCallback;

        // === Cleanup Confirmation Dialog ===
        private static ulong _cleanupDialogId = 0;
        private static UI.DialogResultCallbackDelegate _cleanupDialogResultCallback;

        // === Screenshot Preview Dialog ===
        private static ulong _previewDialogId = 0;
        private static UI.DialogResultCallbackDelegate _previewDialogResultCallback;

        // === Screenshot Click Mapping ===
        private static UI.WidgetCallbackDelegate _screenshotClickCallback;
        private static Dictionary<ulong, int> _screenshotWidgetToStepIndex = new Dictionary<ulong, int>();

        // === Config Dialog ===
        private static ulong _configDialogId = 0;
        private static ulong _configStepsInputId = 0;
        private static ulong _configDurationInputId = 0;
        private static ulong _configIntervalInputId = 0;
        private static ulong _configModeDropdownId = 0;
        private static TestMode _pendingMode;
        private static UI.DialogResultCallbackDelegate _configDialogResultCallback;
        private static UI.DropdownSelectCallbackDelegate _configModeSelectCallback;

        // === Random ===
        private static Random _random = new Random();

        // === Public Properties ===
        public static bool IsRunning { get { return _running; } }
        public static bool IsCompleted { get { return _completed; } }

        // === Config Dialog ===

        public static void ShowTestConfigDialog(TestMode initialMode)
        {
            // Prevent creating multiple dialogs
            if (_configDialogId != 0)
            {
                return;
            }

            _pendingMode = initialMode;
            ulong rootId = UI.GetRootId();
            float cs = UI.GetContentScale();
            float dialogWidth = 420f * cs;
            float dialogHeight = 100f * cs; // Auto-sized to fit content

            _configDialogId = UI.CreateDialog(rootId, "UI Test Config", dialogWidth, dialogHeight);
            _configDialogResultCallback = OnConfigDialogResult;
            UI.DialogSetOnResult(_configDialogId, _configDialogResultCallback);

            ulong content = UI.CreateVStack(_configDialogId, 10f);

            // Mode label (simplified - no dropdown for now)
            string modeName = ModeToString(initialMode);
            UI.CreateLabel(content, dialogWidth - 40f * cs, 28f * cs, "Mode: " + modeName);

            // Steps input
            UI.CreateLabel(content, dialogWidth - 40f * cs, 28f * cs, "Steps:");
            _configStepsInputId = UI.CreateInputField(content, 200f * cs, 28f * cs);
            UI.InputFieldSetPlaceholder(_configStepsInputId, "200");
            UI.InputFieldSetText(_configStepsInputId, "200");

            // Duration input
            UI.CreateLabel(content, dialogWidth - 40f * cs, 28f * cs, "Duration (s):");
            _configDurationInputId = UI.CreateInputField(content, 200f * cs, 28f * cs);
            UI.InputFieldSetPlaceholder(_configDurationInputId, "60");
            UI.InputFieldSetText(_configDurationInputId, "60");

            // Interval input
            UI.CreateLabel(content, dialogWidth - 40f * cs, 28f * cs, "Interval (s):");
            _configIntervalInputId = UI.CreateInputField(content, 200f * cs, 28f * cs);
            UI.InputFieldSetPlaceholder(_configIntervalInputId, "0.5");
            UI.InputFieldSetText(_configIntervalInputId, "0.5");

            UI.DialogSetContent(_configDialogId, content);
            UI.DialogAddButton(_configDialogId, "Start", 1);
            UI.DialogAddButton(_configDialogId, "Cancel", 0);
            UI.DialogShow(_configDialogId);
        }

        private static void OnConfigModeSelect(ulong widgetId, ulong index)
        {
            _pendingMode = (TestMode)index;
        }

        private static void OnConfigDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_configDialogId);

            if (result == 1)
            {
                // Parse inputs
                int steps = ParseInt(UI.InputFieldGetText(_configStepsInputId), 200);
                float duration = ParseFloat(UI.InputFieldGetText(_configDurationInputId), 60f);
                float interval = ParseFloat(UI.InputFieldGetText(_configIntervalInputId), 0.5f);

                // Clamp interval to minimum 0.1s
                if (interval < 0.1f) interval = 0.1f;

                StartTest(_pendingMode, steps, duration, interval);
            }

            _configDialogId = 0;
        }

        private static int ParseInt(string text, int defaultVal)
        {
            try
            {
                if (text != null && text.Length > 0)
                {
                    return int.Parse(text);
                }
            }
            catch { }
            return defaultVal;
        }

        private static float ParseFloat(string text, float defaultVal)
        {
            try
            {
                if (text != null && text.Length > 0)
                {
                    return float.Parse(text);
                }
            }
            catch { }
            return defaultVal;
        }

        // === Start Test ===

        public static void StartSequentialTraversal(int maxSteps, float maxDurationSeconds, float clickInterval)
        {
            StartTest(TestMode.SequentialTraversal, maxSteps, maxDurationSeconds, clickInterval);
        }

        public static void StartRandomTraversal(int maxSteps, float maxDurationSeconds, float clickInterval)
        {
            StartTest(TestMode.RandomTraversal, maxSteps, maxDurationSeconds, clickInterval);
        }

        public static void StartStressTest(float maxDurationSeconds, float clickInterval)
        {
            StartTest(TestMode.StressTest, 99999, maxDurationSeconds, clickInterval);
        }

        private static void StartTest(TestMode mode, int maxSteps, float maxDurationSeconds, float clickInterval)
        {
            _mode = mode;
            _targetSteps = maxSteps;
            _targetDuration = maxDurationSeconds;
            _clickInterval = clickInterval;
            _intervalTimer = 0f;
            _lastFrameDelta = 0f;
            _steps = new List<TestStep>();
            _widgetIds = new List<ulong>();
            _currentStepIndex = 0;
            _elapsedTime = 0f;
            _passedCount = 0;
            _failedCount = 0;
            _random = new Random();
            _completed = false;

            // Collect all widget IDs via tree traversal
            CollectWidgetIds();

            if (_widgetIds.Count == 0)
            {
                Log.Error("UITest", "No widgets found in UI tree");
                return;
            }

            // Shuffle for random/stress modes
            if (mode == TestMode.RandomTraversal || mode == TestMode.StressTest)
            {
                ShuffleList(_widgetIds);
            }

            // Ensure screenshot directory exists
            try
            {
                System.IO.Directory.CreateDirectory("screenshots");
            }
            catch (Exception ex)
            {
                Log.Error("UITest", "Cannot create screenshots dir: " + ex.Message);
            }

            _running = true;
            string modeStr = ModeToString(mode);
            Log.Info("UITest", "Test started: mode=" + modeStr + " steps=" + maxSteps + " duration=" + maxDurationSeconds + "s interval=" + clickInterval + "s widgets=" + _widgetIds.Count);
        }

        // === Update (called from EditorScript.Update) ===

        public static void Update(float deltaTime)
        {
            if (!_running) return;

            // deltaTime in milliseconds from editor
            float dtSeconds = deltaTime / 1000f;

            // No-response timeout detection: if a single frame delta > 3s, engine was frozen
            if (deltaTime > 3000f && _lastFrameDelta > 0f)
            {
                // Engine froze for >3 seconds — record as timeout
                TestStep timeoutStep = new TestStep();
                timeoutStep.StepNumber = _steps.Count + 1;
                timeoutStep.WidgetId = 0;
                timeoutStep.WidgetType = "Engine Freeze";
                timeoutStep.Success = false;
                timeoutStep.ErrorMessage = "No response for " + (deltaTime / 1000f).ToString("F1") + "s (timeout >3s)";
                timeoutStep.ScreenshotPath = "";
                timeoutStep.ClickX = 0;
                timeoutStep.ClickY = 0;
                timeoutStep.Layout = null;
                timeoutStep.WidgetPath = "";
                _steps.Add(timeoutStep);
                _failedCount++;

                // Try to capture screenshot of frozen state
                try
                {
                    string freezePath = "screenshots/step_" + timeoutStep.StepNumber + "_freeze.png";
                    int errResult = UI.CaptureScreenshotToFile(freezePath);
                    if (errResult == 0)
                    {
                        timeoutStep.ScreenshotPath = freezePath;
                    }
                }
                catch { }

                Log.Error("UITest", "Engine freeze detected: " + (deltaTime / 1000f).ToString("F1") + "s gap");
            }
            _lastFrameDelta = deltaTime;

            _elapsedTime += dtSeconds;

            // Check termination conditions
            bool shouldStop = false;
            if (_elapsedTime >= _targetDuration) shouldStop = true;
            if (_currentStepIndex >= _targetSteps) shouldStop = true;
            if (_mode != TestMode.StressTest && _currentStepIndex >= _widgetIds.Count) shouldStop = true;

            if (shouldStop)
            {
                CompleteTest();
                return;
            }

            // Click interval control: accumulate time, only click when interval elapsed
            _intervalTimer += dtSeconds;
            if (_intervalTimer < _clickInterval)
            {
                return; // Wait for next interval
            }
            _intervalTimer -= _clickInterval; // Keep residual time for next step

            // Select widget for this step
            ulong widgetId;
            if (_mode == TestMode.StressTest)
            {
                widgetId = _widgetIds[_random.Next(_widgetIds.Count)];
            }
            else
            {
                widgetId = _widgetIds[_currentStepIndex];
            }

            ExecuteStep(widgetId);
            _currentStepIndex++;
        }

        // === Widget Collection ===

        private static void CollectWidgetIds()
        {
            ulong rootId = UI.GetRootId();
            if (rootId == 0)
            {
                Log.Error("UITest", "Root widget ID is 0");
                return;
            }
            TraverseWidgetTree(rootId);
        }

        private static void TraverseWidgetTree(ulong id)
        {
            if (id == 0) return;

            _widgetIds.Add(id);

            uint childCount = UI.WidgetGetChildCount(id);
            for (uint i = 0; i < childCount; i++)
            {
                ulong childId = UI.WidgetGetChildId(id, i);
                TraverseWidgetTree(childId);
            }
        }

        private static void ShuffleList(List<ulong> list)
        {
            int n = list.Count;
            for (int i = n - 1; i > 0; i--)
            {
                int j = _random.Next(i + 1);
                ulong temp = list[i];
                list[i] = list[j];
                list[j] = temp;
            }
        }

        // === Step Execution ===

        private static void ExecuteStep(ulong widgetId)
        {
            TestStep step = new TestStep();
            step.StepNumber = _steps.Count + 1;
            step.WidgetId = widgetId;
            step.WidgetType = UI.WidgetGetType(widgetId);
            step.Layout = UI.WidgetGetLayout(widgetId);
            step.ScreenshotPath = "";
            step.ErrorMessage = "";
            step.Success = false;
            step.ClickX = 0;
            step.ClickY = 0;
            step.WidgetPath = BuildWidgetPath(widgetId);

            try
            {
                float x = step.Layout[0];
                float y = step.Layout[1];
                float w = step.Layout[2];
                float h = step.Layout[3];

                // Skip zero-size widgets (layout containers, invisible)
                if (w <= 1f || h <= 1f)
                {
                    step.Success = true;
                    step.ErrorMessage = "Skipped (zero-size widget)";
                    _steps.Add(step);
                    return;
                }

                // Click at widget center
                step.ClickX = x + w / 2f;
                step.ClickY = y + h / 2f;

                // Simulate click
                UI.SimulateClickAt(step.ClickX, step.ClickY);

                // Capture screenshot
                string screenshotPath = "screenshots/step_" + step.StepNumber + ".png";
                int result = UI.CaptureScreenshotToFile(screenshotPath);
                if (result == 0)
                {
                    step.ScreenshotPath = screenshotPath;
                }
                else
                {
                    step.ScreenshotPath = "(screenshot failed: code " + result + ")";
                }

                step.Success = true;
                _passedCount++;
            }
            catch (Exception ex)
            {
                step.Success = false;
                step.ErrorMessage = ex.Message;
                _failedCount++;

                // Attempt error screenshot
                try
                {
                    string errorPath = "screenshots/step_" + step.StepNumber + "_error.png";
                    int errResult = UI.CaptureScreenshotToFile(errorPath);
                    if (errResult == 0)
                    {
                        step.ScreenshotPath = errorPath;
                    }
                }
                catch { }
            }

            _steps.Add(step);
        }

        // === Widget Path Builder ===

        private static string BuildWidgetPath(ulong widgetId)
        {
            List<string> parts = new List<string>();
            ulong current = widgetId;
            ulong rootId = UI.GetRootId();
            int maxDepth = 20; // Prevent infinite loops

            while (current != 0 && current != rootId && maxDepth > 0)
            {
                string type = UI.WidgetGetType(current);
                if (type != null && type.Length > 0)
                {
                    parts.Add(type);
                }
                ulong parent = UI.WidgetGetParent(current);
                if (parent == 0 || parent == current) break;
                current = parent;
                maxDepth--;
            }

            if (parts.Count == 0) return "Root";

            // Reverse: parts is leaf-to-root, we want root-to-leaf
            string[] arr = new string[parts.Count];
            for (int i = 0; i < parts.Count; i++)
            {
                arr[i] = parts[parts.Count - 1 - i];
            }

            string result = "Root";
            for (int i = 0; i < arr.Length; i++)
            {
                result += " > " + arr[i];
            }
            return result;
        }

        // === Test Completion ===

        private static void CompleteTest()
        {
            _running = false;
            _completed = true;

            string modeStr = ModeToString(_mode);
            Log.Info("UITest", "Test completed: mode=" + modeStr + " steps=" + _steps.Count + " passed=" + _passedCount + " failed=" + _failedCount + " time=" + _elapsedTime.ToString("F1") + "s");

            ShowReport();
        }

        // === Report Viewer ===

        private static void ShowReport()
        {
            ulong rootId = UI.GetRootId();
            UI.GetScreenSize(out float sw, out float sh);
            float cs = UI.GetContentScale();

            // Clear screenshot click mapping from any previous report
            _screenshotWidgetToStepIndex.Clear();
            _screenshotClickCallback = OnScreenshotClick;

            // Full-screen overlay panel
            _reportPanelId = UI.CreatePanel(rootId, 0, 0, sw, sh, 0.06f, 0.06f, 0.08f, 0.97f);
            UI.SetWidgetLayer(_reportPanelId, 3); // Overlay

            // === Title Bar ===
            ulong titleBar = UI.CreateHStack(_reportPanelId, 10f * cs);
            UI.SetWidgetLayout(titleBar, 15f * cs, 10f * cs, sw - 30f * cs, 40f * cs);

            ulong titleLabel = UI.CreateLabel(titleBar, 400f * cs, 35f * cs, "=== UI Automated Test Report ===");
            UI.SetWidgetBackgroundColor(titleLabel, 0.2f, 0.5f, 0.7f, 0.4f);

            // Close button
            _reportCloseBtnId = UI.CreateButton(titleBar, 120f * cs, 35f * cs, "Close Report");
            _closeReportCallback = OnCloseReport;
            UI.SetOnClick(_reportCloseBtnId, _closeReportCallback);

            // === Summary Section ===
            string modeStr = ModeToString(_mode);
            float passRate = _steps.Count > 0 ? (_passedCount * 100f / _steps.Count) : 0f;

            string summaryLine1 = "Mode: " + modeStr + "  |  Total Steps: " + _steps.Count + "  |  Passed: " + _passedCount + "  |  Failed: " + _failedCount;
            string summaryLine2 = "Pass Rate: " + passRate.ToString("F1") + "%  |  Duration: " + _elapsedTime.ToString("F1") + "s  |  Interval: " + _clickInterval.ToString("F2") + "s  |  Widgets: " + _widgetIds.Count;

            ulong summaryPanel = UI.CreatePanel(_reportPanelId, 15f * cs, 55f * cs, sw - 30f * cs, 70f * cs, 0.12f, 0.12f, 0.15f, 1.0f);
            ulong summaryContent = UI.CreateVStack(summaryPanel, 5f * cs);
            UI.SetFlexExpand(summaryContent, true);
            UI.SetCrossAxisFill(summaryContent, true);

            UI.CreateLabel(summaryContent, sw - 50f * cs, 25f * cs, summaryLine1);
            UI.CreateLabel(summaryContent, sw - 50f * cs, 25f * cs, summaryLine2);

            // Progress bar (green portion = pass rate)
            float barFullWidth = sw - 50f * cs;
            float barPassWidth = barFullWidth * passRate / 100f;
            ulong barBg = UI.CreatePanel(summaryContent, 0, 0, barFullWidth, 8f * cs, 0.3f, 0.15f, 0.15f, 0.8f);
            ulong barGreen = UI.CreatePanel(barBg, 0, 0, barPassWidth, 8f * cs, 0.2f, 0.7f, 0.2f, 1.0f);

            // === Click Path Section ===
            string clickPath = BuildClickPathSummary();
            ulong clickPathPanel = UI.CreatePanel(_reportPanelId, 15f * cs, 130f * cs, sw - 30f * cs, 35f * cs, 0.1f, 0.15f, 0.12f, 1.0f);
            ulong clickPathContent = UI.CreateVStack(clickPathPanel, 3f * cs);
            UI.SetFlexExpand(clickPathContent, true);
            UI.SetCrossAxisFill(clickPathContent, true);
            UI.CreateLabel(clickPathContent, sw - 50f * cs, 15f * cs, "Click Path:");
            UI.CreateLabel(clickPathContent, sw - 50f * cs, 15f * cs, clickPath);

            // === Step Details (ScrollView) ===
            float scrollY = 170f * cs;
            float scrollH = sh - scrollY - 25f * cs;
            ulong scrollView = UI.CreateScrollView(_reportPanelId, 15f * cs, scrollY, sw - 30f * cs, scrollH);
            UI.ScrollViewSetShowScrollbars(scrollView, true, false);

            ulong scrollContent = UI.CreateVStack(scrollView, 4f * cs);
            UI.SetFlexExpand(scrollContent, true);
            UI.SetCrossAxisFill(scrollContent, true);

            // Column headers
            ulong headerRow = UI.CreatePanel(scrollContent, 0, 0, sw - 50f * cs, 28f * cs, 0.18f, 0.18f, 0.22f, 1.0f);
            ulong headerContent = UI.CreateHStack(headerRow, 10f * cs);
            UI.SetFlexExpand(headerContent, true);
            UI.SetCrossAxisFill(headerContent, true);
            UI.CreateLabel(headerContent, 50f * cs, 22f * cs, "#");
            UI.CreateLabel(headerContent, 60f * cs, 22f * cs, "Status");
            UI.CreateLabel(headerContent, 120f * cs, 22f * cs, "Widget");
            UI.CreateLabel(headerContent, 100f * cs, 22f * cs, "Click");
            UI.CreateLabel(headerContent, 80f * cs, 22f * cs, "Path");
            UI.CreateLabel(headerContent, 200f * cs, 22f * cs, "Screenshot/Error");

            // Step rows
            for (int i = 0; i < _steps.Count; i++)
            {
                AddStepRow(scrollContent, _steps[i], sw, cs);
            }

            // === Footer ===
            ulong footer = UI.CreatePanel(_reportPanelId, 15f * cs, sh - 20f * cs, sw - 30f * cs, 18f * cs, 0.1f, 0.1f, 0.12f, 1.0f);
            UI.CreateLabel(footer, sw - 50f * cs, 15f * cs, "Report: " + DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss") + "  |  Press Close to resume editor");
        }

        // === Click Path Summary ===

        private static string BuildClickPathSummary()
        {
            if (_steps.Count == 0) return "(no steps)";

            string result = "";
            for (int i = 0; i < _steps.Count; i++)
            {
                TestStep step = _steps[i];
                if (i > 0) result += " -> ";
                result += step.StepNumber + ":" + step.WidgetType;
                // Limit length to prevent overflow in UI label
                if (result.Length > 200)
                {
                    result += " ...";
                    break;
                }
            }
            return result;
        }

        // === Step Row ===

        private static void AddStepRow(ulong parent, TestStep step, float screenWidth, float cs)
        {
            float rowWidth = screenWidth - 50f * cs;

            // Color: green for pass, red for fail, orange for timeout/freeze
            float bgR, bgG, bgB;
            if (step.Success)
            {
                bgR = 0.12f; bgG = 0.22f; bgB = 0.12f;
            }
            else if (step.WidgetType == "Engine Freeze")
            {
                bgR = 0.25f; bgG = 0.18f; bgB = 0.08f; // Orange for timeout
            }
            else
            {
                bgR = 0.25f; bgG = 0.12f; bgB = 0.12f;
            }

            ulong rowPanel = UI.CreatePanel(parent, 0, 0, rowWidth, 55f * cs, bgR, bgG, bgB, 0.85f);
            ulong rowContent = UI.CreateVStack(rowPanel, 2f * cs);
            UI.SetFlexExpand(rowContent, true);
            UI.SetCrossAxisFill(rowContent, true);

            // Line 1: step number, status, widget type + id, click position, widget path
            string statusIcon = step.WidgetType == "Engine Freeze" ? "TIMEOUT" : (step.Success ? "PASS" : "FAIL");
            string line1 = "#" + step.StepNumber + " [" + statusIcon + "] " + step.WidgetType;
            if (step.WidgetId != 0)
            {
                line1 += "(id=" + step.WidgetId + ")";
            }

            if (step.ClickX != 0 || step.ClickY != 0)
            {
                line1 += "  Click:(" + step.ClickX.ToString("F0") + "," + step.ClickY.ToString("F0") + ")";
            }

            // Widget path (abbreviated if too long)
            if (step.WidgetPath != null && step.WidgetPath.Length > 0 && step.WidgetPath != "Root")
            {
                string path = step.WidgetPath;
                if (path.Length > 60)
                {
                    // Show last few segments only
                    int lastSep = path.LastIndexOf(" > ");
                    if (lastSep > 0)
                    {
                        int prevSep = path.LastIndexOf(" > ", lastSep - 1);
                        if (prevSep > 0)
                        {
                            path = "..." + path.Substring(prevSep + 3);
                        }
                    }
                }
                line1 += "  Path:" + path;
            }

            UI.CreateLabel(rowContent, rowWidth - 10f * cs, 22f * cs, line1);

            // Line 2: screenshot (clickable button) or error message
            bool hasScreenshot = step.ScreenshotPath != null && step.ScreenshotPath.Length > 0
                && !step.ScreenshotPath.StartsWith("(screenshot failed");

            if (hasScreenshot)
            {
                // Clickable screenshot button — opens preview dialog on click
                ulong screenshotBtn = UI.CreateButton(rowContent, rowWidth - 10f * cs, 22f * cs, "[IMG] " + step.ScreenshotPath);
                UI.SetWidgetBackgroundColor(screenshotBtn, 0.15f, 0.25f, 0.35f, 0.6f);

                // Map button widgetId to step index for click callback lookup
                _screenshotWidgetToStepIndex[screenshotBtn] = step.StepNumber - 1;
                UI.SetOnClick(screenshotBtn, _screenshotClickCallback);
            }
            else if (!step.Success && step.ErrorMessage != null && step.ErrorMessage.Length > 0)
            {
                string err = step.ErrorMessage;
                if (err.Length > 100) err = err.Substring(0, 100) + "...";
                UI.CreateLabel(rowContent, rowWidth - 10f * cs, 18f * cs, "Error: " + err);
            }
            else if (step.Success && step.ErrorMessage != null && step.ErrorMessage.Length > 0)
            {
                UI.CreateLabel(rowContent, rowWidth - 10f * cs, 18f * cs, step.ErrorMessage); // "Skipped" message
            }
        }

        // === Close Report → Cleanup Confirmation Dialog ===

        private static void OnCloseReport(ulong widgetId)
        {
            // Don't open cleanup dialog if one is already open or preview is open
            if (_cleanupDialogId != 0) return;
            if (_previewDialogId != 0) return;

            ulong rootId = UI.GetRootId();
            float cs = UI.GetContentScale();
            float dialogWidth = 380f * cs;
            float dialogHeight = 100f * cs;

            _cleanupDialogId = UI.CreateDialog(rootId, "Close Report", dialogWidth, dialogHeight);
            _cleanupDialogResultCallback = OnCleanupDialogResult;
            UI.DialogSetOnResult(_cleanupDialogId, _cleanupDialogResultCallback);

            ulong content = UI.CreateVStack(_cleanupDialogId, 10f * cs);
            UI.CreateLabel(content, dialogWidth - 40f * cs, 30f * cs, "是否清理所有测试产生的数据？");
            UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "(截图文件等)");

            UI.DialogSetContent(_cleanupDialogId, content);
            UI.DialogAddButton(_cleanupDialogId, "清理并关闭", 2);
            UI.DialogAddButton(_cleanupDialogId, "仅关闭", 1);
            UI.DialogShow(_cleanupDialogId);
        }

        private static void OnCleanupDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_cleanupDialogId);
            _cleanupDialogId = 0;

            // Also close any open preview dialog
            if (_previewDialogId != 0)
            {
                UI.DialogHide(_previewDialogId);
                _previewDialogId = 0;
            }

            if (result == 2)
            {
                CleanupScreenshots();
            }

            // Close the report panel
            if (_reportPanelId != 0)
            {
                UI.RemoveWidget(_reportPanelId);
                _reportPanelId = 0;
            }
            _completed = false;
            _screenshotWidgetToStepIndex.Clear();
        }

        private static void CleanupScreenshots()
        {
            try
            {
                string[] files = System.IO.Directory.GetFiles("screenshots");
                for (int i = 0; i < files.Length; i++)
                {
                    try
                    {
                        System.IO.File.Delete(files[i]);
                    }
                    catch { }
                }
                Log.Info("UITest", "Cleaned up " + files.Length + " screenshot files");
            }
            catch (Exception ex)
            {
                Log.Error("UITest", "Failed to cleanup screenshots: " + ex.Message);
            }
        }

        // === Screenshot Preview Dialog ===

        private static void OnScreenshotClick(ulong widgetId)
        {
            if (!_screenshotWidgetToStepIndex.ContainsKey(widgetId)) return;
            if (_previewDialogId != 0) return;
            if (_cleanupDialogId != 0) return;

            int stepIndex = _screenshotWidgetToStepIndex[widgetId];
            TestStep step = _steps[stepIndex];

            ulong rootId = UI.GetRootId();
            float cs = UI.GetContentScale();
            float dialogWidth = 500f * cs;
            float dialogHeight = 100f * cs;

            _previewDialogId = UI.CreateDialog(rootId, "Step #" + step.StepNumber + " Screenshot", dialogWidth, dialogHeight);
            _previewDialogResultCallback = OnPreviewDialogResult;
            UI.DialogSetOnResult(_previewDialogId, _previewDialogResultCallback);

            ulong content = UI.CreateVStack(_previewDialogId, 8f * cs);

            // Screenshot path
            if (step.ScreenshotPath != null && step.ScreenshotPath.Length > 0)
            {
                UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "Screenshot: " + step.ScreenshotPath);
            }

            // Click position
            UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "Click: (" + step.ClickX.ToString("F0") + ", " + step.ClickY.ToString("F0") + ")");

            // Widget info
            string widgetInfo = step.WidgetType;
            if (step.WidgetId != 0) widgetInfo += "(id=" + step.WidgetId + ")";
            UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "Widget: " + widgetInfo);

            // Widget path
            if (step.WidgetPath != null && step.WidgetPath.Length > 0)
            {
                UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "Path: " + step.WidgetPath);
            }

            // Status
            string status = step.Success ? "PASS" : "FAIL";
            if (step.WidgetType == "Engine Freeze") status = "TIMEOUT";
            UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "Status: " + status);

            // Error message if present
            if (!step.Success && step.ErrorMessage != null && step.ErrorMessage.Length > 0)
            {
                string err = step.ErrorMessage;
                if (err.Length > 80) err = err.Substring(0, 80) + "...";
                UI.CreateLabel(content, dialogWidth - 40f * cs, 22f * cs, "Error: " + err);
            }

            UI.DialogSetContent(_previewDialogId, content);
            UI.DialogAddButton(_previewDialogId, "Close", 0);
            UI.DialogShow(_previewDialogId);
        }

        private static void OnPreviewDialogResult(ulong dialogId, int result)
        {
            UI.DialogHide(_previewDialogId);
            _previewDialogId = 0;
        }

        // === Utility ===

        private static string ModeToString(TestMode mode)
        {
            if (mode == TestMode.SequentialTraversal) return "Sequential Traversal";
            if (mode == TestMode.RandomTraversal) return "Random Traversal";
            return "Stress Test";
        }
    }
}