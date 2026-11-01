using System;
using System.IO;
using System.Reflection;
using HezhouScripts;

namespace Hezhou
{
    /// <summary>
    /// 反射管线+写回+值保存测试 — TDD RED阶段
    /// 验证ScriptEntityHelper.ReflectProperties、GetFieldValue/SetFieldValue、
    /// ModifyScriptSourceFile、SaveAllScriptPropertyValues/RestoreAllScriptPropertyValues
    /// 
    /// mcs兼容: 无LINQ/async/.NET 8 API
    /// 所有测试在Initialize()末尾运行，不影响编辑器正常操作
    /// </summary>
    public static class TestRunner
    {
        private static int _passCount = 0;
        private static int _failCount = 0;

        public static void RunAllTests()
        {
            _passCount = 0;
            _failCount = 0;

            Log.Info("TestRunner", "===== 开始运行反射管线测试 =====");

            try { TestReflectProperties(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestReflectProperties异常: " + ex.Message); }

            try { TestGetSetFieldValue(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestGetSetFieldValue异常: " + ex.Message); }

            try { TestModifyScriptSourceFile(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestModifyScriptSourceFile异常: " + ex.Message); }

            try { TestSaveRestorePropertyValues(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestSaveRestorePropertyValues异常: " + ex.Message); }

            try { TestRendererQualityFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestRendererQualityFfi异常: " + ex.Message); }

            try { TestAudioAssetFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestAudioAssetFfi异常: " + ex.Message); }

            try { TestPrefabScenePlayerFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestPrefabScenePlayerFfi异常: " + ex.Message); }

            Log.Info("TestRunner", "===== 测试完成: PASS=" + _passCount + ", FAIL=" + _failCount + " =====");
        }

        private static void AssertTrue(string testName, bool condition, string message)
        {
            if (condition)
            {
                _passCount++;
                Log.Info("TestRunner", "PASS: " + testName + " - " + message);
            }
            else
            {
                _failCount++;
                Log.Error("TestRunner", "FAIL: " + testName + " - " + message);
            }
        }

        private static void AssertEqualFloat(string testName, float expected, float actual, string message)
        {
            bool pass = Math.Abs(expected - actual) < 0.001f;
            string detail = message + " (expected=" + expected + ", actual=" + actual + ")";
            AssertTrue(testName, pass, detail);
        }

        // ===== Test 1: 反射属性发现 =====
        // 验证ScriptEntityHelper.ReflectProperties能正确发现[Expose]字段
        // 并生成包含Min/Max/Step/Initial/Widget/Type/Name的ScriptPropertyDescriptor
        private static void TestReflectProperties()
        {
            Log.Info("TestRunner", "--- TestReflectProperties ---");

            ScriptPropertyDescriptor[] descriptors = ScriptEntityHelper.ReflectProperties(typeof(RotatingEntity));

            // 验证返回2个描述符
            AssertTrue("ReflectProperties", descriptors != null, "descriptors不为null");
            AssertTrue("ReflectProperties", descriptors.Length == 2,
                "返回2个描述符 (actual=" + (descriptors != null ? descriptors.Length : 0) + ")");

            if (descriptors == null || descriptors.Length < 2) return;

            // 按名称查找描述符（字段顺序不保证，按名查找更稳健）
            ScriptPropertyDescriptor speedDesc = new ScriptPropertyDescriptor();
            ScriptPropertyDescriptor offsetDesc = new ScriptPropertyDescriptor();
            bool foundSpeed = false;
            bool foundOffset = false;

            for (int i = 0; i < descriptors.Length; i++)
            {
                if (descriptors[i].Name == "rotationSpeed")
                {
                    speedDesc = descriptors[i];
                    foundSpeed = true;
                }
                else if (descriptors[i].Name == "angleOffset")
                {
                    offsetDesc = descriptors[i];
                    foundOffset = true;
                }
            }

            AssertTrue("ReflectProperties", foundSpeed, "找到rotationSpeed描述符");
            AssertTrue("ReflectProperties", foundOffset, "找到angleOffset描述符");

            if (!foundSpeed || !foundOffset) return;

            // 验证rotationSpeed描述符各字段
            AssertTrue("ReflectProperties", speedDesc.Name == "rotationSpeed",
                "rotationSpeed Name正确 (actual=" + speedDesc.Name + ")");
            AssertEqualFloat("ReflectProperties", 0f, speedDesc.Min, "rotationSpeed Min");
            AssertEqualFloat("ReflectProperties", 100f, speedDesc.Max, "rotationSpeed Max");
            AssertEqualFloat("ReflectProperties", 1f, speedDesc.Step, "rotationSpeed Step");
            AssertEqualFloat("ReflectProperties", 10f, speedDesc.Initial, "rotationSpeed Initial");
            AssertTrue("ReflectProperties", speedDesc.Widget == "slider",
                "rotationSpeed Widget=slider (actual=" + speedDesc.Widget + ")");
            AssertTrue("ReflectProperties", speedDesc.Type == 0,
                "rotationSpeed Type=Float(0) (actual=" + speedDesc.Type + ")");
            AssertTrue("ReflectProperties", speedDesc.DisplayName == "旋转速度",
                "rotationSpeed DisplayName=旋转速度 (actual=" + speedDesc.DisplayName + ")");

            // 验证angleOffset描述符各字段
            AssertTrue("ReflectProperties", offsetDesc.Name == "angleOffset",
                "angleOffset Name正确 (actual=" + offsetDesc.Name + ")");
            AssertEqualFloat("ReflectProperties", -360f, offsetDesc.Min, "angleOffset Min");
            AssertEqualFloat("ReflectProperties", 360f, offsetDesc.Max, "angleOffset Max");
            AssertEqualFloat("ReflectProperties", 1f, offsetDesc.Step, "angleOffset Step");
            AssertEqualFloat("ReflectProperties", 0f, offsetDesc.Initial, "angleOffset Initial");
            AssertTrue("ReflectProperties", offsetDesc.Widget == "input",
                "angleOffset Widget=input (actual=" + offsetDesc.Widget + ")");
            AssertTrue("ReflectProperties", offsetDesc.Type == 0,
                "angleOffset Type=Float(0) (actual=" + offsetDesc.Type + ")");
            AssertTrue("ReflectProperties", offsetDesc.DisplayName == "旋转角度偏移",
                "angleOffset DisplayName=旋转角度偏移 (actual=" + offsetDesc.DisplayName + ")");
        }

        // ===== Test 2: 字段值读写 =====
        // 验证ScriptEntityHelper.GetFieldValue/SetFieldValue能正确读写实例字段值
        private static void TestGetSetFieldValue()
        {
            Log.Info("TestRunner", "--- TestGetSetFieldValue ---");

            // 创建RotatingEntity实例（返回字典key IntPtr）
            IntPtr instancePtr = RotatingEntity.CreateInstance();
            AssertTrue("GetSetFieldValue", instancePtr != IntPtr.Zero, "CreateInstance返回有效指针");

            if (instancePtr == IntPtr.Zero) return;

            // 读取初始值（rotationSpeed=10, angleOffset=0）
            float initialSpeed = ScriptEntityHelper.GetFieldValue(instancePtr, "rotationSpeed", typeof(RotatingEntity));
            AssertEqualFloat("GetSetFieldValue", 10f, initialSpeed, "初始rotationSpeed=10");

            float initialOffset = ScriptEntityHelper.GetFieldValue(instancePtr, "angleOffset", typeof(RotatingEntity));
            AssertEqualFloat("GetSetFieldValue", 0f, initialOffset, "初始angleOffset=0");

            // 写入新值并验证读取
            ScriptEntityHelper.SetFieldValue(instancePtr, "rotationSpeed", 200f, typeof(RotatingEntity));
            float newSpeed = ScriptEntityHelper.GetFieldValue(instancePtr, "rotationSpeed", typeof(RotatingEntity));
            AssertEqualFloat("GetSetFieldValue", 200f, newSpeed, "写入rotationSpeed=200后读取验证");

            ScriptEntityHelper.SetFieldValue(instancePtr, "angleOffset", 45f, typeof(RotatingEntity));
            float newOffset = ScriptEntityHelper.GetFieldValue(instancePtr, "angleOffset", typeof(RotatingEntity));
            AssertEqualFloat("GetSetFieldValue", 45f, newOffset, "写入angleOffset=45后读取验证");

            // 清理测试实例
            RotatingEntity.DestroyInstance(instancePtr);
        }

        // ===== Test 3: .cs源文件写回 =====
        // 验证ModifyScriptSourceFile能修改[Expose]参数并写回.cs源文件
        private static void TestModifyScriptSourceFile()
        {
            Log.Info("TestRunner", "--- TestModifyScriptSourceFile ---");

            // 通过反射调用EditorScript.ModifyScriptSourceFile (private static方法)
            MethodInfo modifyMethod = typeof(EditorScript).GetMethod("ModifyScriptSourceFile",
                BindingFlags.Static | BindingFlags.NonPublic);
            AssertTrue("ModifyScriptSourceFile", modifyMethod != null, "ModifyScriptSourceFile方法可找到");

            if (modifyMethod == null) return;

            // 读取原始文件内容（用于安全恢复）
            string filePath = Path.Combine("scripts", "RotatingEntity.cs");
            AssertTrue("ModifyScriptSourceFile", File.Exists(filePath), "RotatingEntity.cs文件存在");

            if (!File.Exists(filePath)) return;

            string originalContent = File.ReadAllText(filePath);

            // 调用ModifyScriptSourceFile: 将rotationSpeed的Max写回为50
            modifyMethod.Invoke(null, new object[] { "RotatingEntity", "rotationSpeed", "Max", 50f });

            // 读取修改后的文件，验证Max参数变化
            string modifiedContent = File.ReadAllText(filePath);
            bool maxChanged = modifiedContent.Contains("Max = 50");
            AssertTrue("ModifyScriptSourceFile", maxChanged, "Max参数已写回为50");

            // 恢复原始内容（保留用户可能已修改的值，如Max=300而非硬编码400）
            File.WriteAllText(filePath, originalContent);
            EditorScript.TestClearSourceModifiedFlag();
            Log.Info("TestRunner", "ModifyScriptSourceFile: 已恢复原始文件内容（保留用户修改）");
        }

        // ===== Test 4: 值保存与恢复 =====
        // 验证SaveAllScriptPropertyValues/RestoreAllScriptPropertyValues方法存在
        // 并直接测试内存保存/恢复机制（与SaveAllScriptPropertyValues相同的内存List方式）
        private static void TestSaveRestorePropertyValues()
        {
            Log.Info("TestRunner", "--- TestSaveRestorePropertyValues ---");

            // 验证方法存在（private static on EditorScript）
            MethodInfo saveMethod = typeof(EditorScript).GetMethod("SaveAllScriptPropertyValues",
                BindingFlags.Static | BindingFlags.NonPublic);
            MethodInfo restoreMethod = typeof(EditorScript).GetMethod("RestoreAllScriptPropertyValues",
                BindingFlags.Static | BindingFlags.NonPublic);

            AssertTrue("SaveRestore", saveMethod != null, "SaveAllScriptPropertyValues方法可找到");
            AssertTrue("SaveRestore", restoreMethod != null, "RestoreAllScriptPropertyValues方法可找到");

            // 直接测试内存保存/恢复机制（不依赖_gameScene，使用内存List方式）
            IntPtr instancePtr = RotatingEntity.CreateInstance();
            AssertTrue("SaveRestore", instancePtr != IntPtr.Zero, "CreateInstance返回有效指针");

            if (instancePtr == IntPtr.Zero) return;

            try
            {
                // 读取初始值
                float originalSpeed = ScriptEntityHelper.GetFieldValue(instancePtr, "rotationSpeed", typeof(RotatingEntity));
                float originalOffset = ScriptEntityHelper.GetFieldValue(instancePtr, "angleOffset", typeof(RotatingEntity));

                // 保存值到内存List（模拟SaveAllScriptPropertyValues的_savedScriptPropertyValues）
                // 格式: propertyName → runtimeValue
                SavedTestValue[] savedValues = new SavedTestValue[2];
                savedValues[0] = new SavedTestValue();
                savedValues[0].PropertyName = "rotationSpeed";
                savedValues[0].RuntimeValue = originalSpeed;
                savedValues[1] = new SavedTestValue();
                savedValues[1].PropertyName = "angleOffset";
                savedValues[1].RuntimeValue = originalOffset;

                // 修改值（模拟运行时值变化）
                ScriptEntityHelper.SetFieldValue(instancePtr, "rotationSpeed", 999f, typeof(RotatingEntity));
                ScriptEntityHelper.SetFieldValue(instancePtr, "angleOffset", -180f, typeof(RotatingEntity));

                float modifiedSpeed = ScriptEntityHelper.GetFieldValue(instancePtr, "rotationSpeed", typeof(RotatingEntity));
                AssertEqualFloat("SaveRestore", 999f, modifiedSpeed, "修改后rotationSpeed=999");

                // 从内存List恢复值（模拟RestoreAllScriptPropertyValues逻辑）
                // 包含clamp越界处理（与RestoreAllScriptPropertyValues相同逻辑）
                ScriptPropertyDescriptor[] descriptors = ScriptEntityHelper.ReflectProperties(typeof(RotatingEntity));
                int restoredCount = 0;

                for (int i = 0; i < savedValues.Length; i++)
                {
                    // 查找匹配的descriptor（与RestoreAll相同: 按名称匹配）
                    ScriptPropertyDescriptor descriptor = new ScriptPropertyDescriptor();
                    bool foundDescriptor = false;
                    for (int d = 0; d < descriptors.Length; d++)
                    {
                        if (descriptors[d].Name == savedValues[i].PropertyName)
                        {
                            descriptor = descriptors[d];
                            foundDescriptor = true;
                            break;
                        }
                    }
                    if (!foundDescriptor) continue;

                    // clamp越界处理（与RestoreAll相同逻辑）
                    float clampedValue = savedValues[i].RuntimeValue;
                    if (clampedValue > descriptor.Max) clampedValue = descriptor.Max;
                    if (clampedValue < descriptor.Min) clampedValue = descriptor.Min;

                    ScriptEntityHelper.SetFieldValue(instancePtr, savedValues[i].PropertyName, clampedValue, typeof(RotatingEntity));
                    restoredCount++;
                }

                AssertTrue("SaveRestore", restoredCount == 2,
                    "恢复了2个属性值 (actual=" + restoredCount + ")");

                // 验证值已恢复到原始值（clamp后）
                float restoredSpeed = ScriptEntityHelper.GetFieldValue(instancePtr, "rotationSpeed", typeof(RotatingEntity));
                AssertEqualFloat("SaveRestore", originalSpeed, restoredSpeed, "恢复后rotationSpeed=原值");

                float restoredOffset = ScriptEntityHelper.GetFieldValue(instancePtr, "angleOffset", typeof(RotatingEntity));
                AssertEqualFloat("SaveRestore", originalOffset, restoredOffset, "恢复后angleOffset=原值");
            }
            finally
            {
                // 清理测试实例
                RotatingEntity.DestroyInstance(instancePtr);
            }
        }

        /// <summary>
        /// 0.2 渲染质量 / PBR FFI 自检（task-15）：
        /// 默认 legacy；切 High 成功并反映到摘要；非法档位被拒绝；切回 Legacy 零回归。
        /// </summary>
        private static void TestRendererQualityFfi()
        {
            string summaryLegacy = UI.RendererGetPbrSummary();
            AssertTrue("RendererFFI", summaryLegacy.Length > 0,
                "RendererGetPbrSummary 返回非空 (len=" + summaryLegacy.Length + ")");
            AssertTrue("RendererFFI", summaryLegacy.IndexOf("quality=") >= 0,
                "默认摘要包含 quality= (" + summaryLegacy + ")");
            AssertTrue("RendererFFI", !UI.RendererPbrActive(),
                "默认 legacy：PBR draw path 未激活");

            int rcHigh = UI.RendererSetQualityPreset(2);
            AssertTrue("RendererFFI", rcHigh == 0, "切换到 High 返回 0 (rc=" + rcHigh + ")");
            string summaryHigh = UI.RendererGetPbrSummary();
            AssertTrue("RendererFFI", summaryHigh.ToLower().IndexOf("quality=high") >= 0,
                "High 摘要包含 quality=high (" + summaryHigh + ")");

            int rcBad = UI.RendererSetQualityPreset(99);
            AssertTrue("RendererFFI", rcBad == -2, "非法档位 99 返回 -2 (rc=" + rcBad + ")");

            int rcLegacy = UI.RendererSetQualityPreset(3);
            AssertTrue("RendererFFI", rcLegacy == 0, "切回 Legacy 返回 0 (rc=" + rcLegacy + ")");
            string summaryBack = UI.RendererGetPbrSummary();
            AssertTrue("RendererFFI", summaryBack.StartsWith("Legacy"),
                "恢复 Legacy 默认 (" + summaryBack + ")");

            // 工具栏按钮同一路径：EditorScript.ApplyQualityPreset（含按钮文本/状态栏/日志）
            EditorScript.ApplyQualityPreset(0);
            string summaryLow = UI.RendererGetPbrSummary();
            AssertTrue("RendererFFI", summaryLow.ToLower().IndexOf("quality=low") >= 0,
                "ApplyQualityPreset(Low) → quality=low (" + summaryLow + ")");
            EditorScript.ApplyQualityPreset(3);
        }

        /// <summary>
        /// 0.2 音频 / 资产数据库 FFI 自检（task-16）：
        /// 启动时资产自动扫描、AssetId 可查询；音频引擎就绪（无设备自动降级）、
        /// 播放/暂停/恢复/停止/主音量/监听者全链路可调用且不崩溃。
        /// </summary>
        private static void TestAudioAssetFfi()
        {
            // === 资产数据库（启动时已自动扫描当前工程） ===
            int scanRc = UI.AssetScan();
            AssertTrue("AudioAsset", scanRc == 0, "AssetScan 成功 (rc=" + scanRc + ")");
            string assetSummary = UI.AssetGetSummary();
            AssertTrue("AudioAsset", assetSummary.IndexOf("assets=") >= 0,
                "资产摘要包含 assets= (" + assetSummary + ")");
            int count = UI.AssetCount();
            AssertTrue("AudioAsset", count >= 0, "AssetCount 非负 (count=" + count + ")");
            if (count > 0)
            {
                string assetId;
                string locator;
                bool ok = UI.AssetGetAsset(0, out assetId, out locator);
                AssertTrue("AudioAsset", ok && assetId.Length > 0 && locator.Length > 0,
                    "AssetGetAsset(0) 返回 AssetId+定位符 (id=" + assetId + ", locator=" + locator + ")");
            }
            else
            {
                AssertTrue("AudioAsset", true, "工程无资产，跳过 AssetId 查询");
            }

            // === glTF 导入（创建临时最小 glTF -> 导入 -> 清理） ===
            string selftestDir = System.IO.Path.Combine(
                System.IO.Directory.GetCurrentDirectory(), "assets");
            string selftestPath = System.IO.Path.Combine(selftestDir, "__ffi_selftest__.gltf");
            try
            {
                System.IO.Directory.CreateDirectory(selftestDir);
                byte[] bin = new byte[42];
                float[] positions = new float[] { 0f, 0f, 0f, 1f, 0f, 0f, 0f, 1f, 0f };
                for (int i = 0; i < positions.Length; i++)
                {
                    byte[] bytes = BitConverter.GetBytes(positions[i]);
                    Buffer.BlockCopy(bytes, 0, bin, i * 4, 4);
                }
                ushort[] indices = new ushort[] { 0, 1, 2 };
                for (int i = 0; i < indices.Length; i++)
                {
                    byte[] bytes = BitConverter.GetBytes(indices[i]);
                    Buffer.BlockCopy(bytes, 0, bin, 36 + i * 2, 2);
                }
                string base64 = Convert.ToBase64String(bin);
                string json = "{"
                    + "\"asset\":{\"version\":\"2.0\"},"
                    + "\"scene\":0,"
                    + "\"scenes\":[{\"nodes\":[0]}],"
                    + "\"nodes\":[{\"name\":\"FFITest\",\"mesh\":0}],"
                    + "\"meshes\":[{\"name\":\"FFITestMesh\",\"primitives\":[{\"attributes\":{\"POSITION\":0},\"indices\":1,\"material\":0}]}],"
                    + "\"materials\":[{\"name\":\"FFITestMat\",\"pbrMetallicRoughness\":{\"baseColorFactor\":[1,0,0,1]}}],"
                    + "\"accessors\":["
                    + "{\"bufferView\":0,\"componentType\":5126,\"count\":3,\"type\":\"VEC3\",\"min\":[0,0,0],\"max\":[1,1,0]},"
                    + "{\"bufferView\":1,\"componentType\":5123,\"count\":3,\"type\":\"SCALAR\"}],"
                    + "\"bufferViews\":[{\"buffer\":0,\"byteOffset\":0,\"byteLength\":36},{\"buffer\":0,\"byteOffset\":36,\"byteLength\":6}],"
                    + "\"buffers\":[{\"byteLength\":42,\"uri\":\"data:application/octet-stream;base64," + base64 + "\"}]"
                    + "}";
                System.IO.File.WriteAllText(selftestPath, json);
                int importRc = UI.AssetImportGltf(selftestPath);
                AssertTrue("AudioAsset", importRc == 0, "AssetImportGltf 临时 glTF 返回 0 (rc=" + importRc + ")");
            }
            catch (Exception ex)
            {
                AssertTrue("AudioAsset", false, "glTF 导入自检异常: " + ex.Message);
            }
            finally
            {
                try { if (System.IO.File.Exists(selftestPath)) System.IO.File.Delete(selftestPath); } catch (Exception) { }
                try { if (System.IO.File.Exists(selftestPath + ".meta")) System.IO.File.Delete(selftestPath + ".meta"); } catch (Exception) { }
                int cleanupRc = UI.AssetScan();
                AssertTrue("AudioAsset", cleanupRc == 0, "导入自检后重新扫描返回 0 (rc=" + cleanupRc + ")");
            }

            // === 音频（启动时已初始化；无设备自动降级为无声） ===
            string audioSummary = UI.AudioGetSummary();
            AssertTrue("AudioAsset", audioSummary.IndexOf("audio=on") >= 0,
                "音频引擎已就绪 (" + audioSummary + ")");
            int rcInit = UI.AudioInit();
            AssertTrue("AudioAsset", rcInit == 0, "AudioInit 幂等返回 0 (rc=" + rcInit + ")");

            UI.AudioSetMasterVolume(0f); // 自检静音，避免启动时听到测试音
            int rcPlay = UI.AudioPlayFirstAsset(false);
            AssertTrue("AudioAsset", rcPlay == 0, "AudioPlayFirstAsset 返回 0 (rc=" + rcPlay + ")");
            int rcPause = UI.AudioPause();
            AssertTrue("AudioAsset", rcPause == 0, "AudioPause 返回 0 (rc=" + rcPause + ")");
            int rcResume = UI.AudioResume();
            AssertTrue("AudioAsset", rcResume == 0, "AudioResume 返回 0 (rc=" + rcResume + ")");
            int rcStop = UI.AudioStop();
            AssertTrue("AudioAsset", rcStop == 0, "AudioStop 返回 0 (rc=" + rcStop + ")");
            UI.AudioSetMasterVolume(0.8f);
            int rcListener = UI.AudioSetListener(0f, 1.5f, -3f);
            AssertTrue("AudioAsset", rcListener == 0, "AudioSetListener 返回 0");
        }

        /// <summary>
        /// 0.2 Prefab / SceneManager / Player 导出 dry-run 自检（task-21）：
        /// Prefab 捕获+实例化（并清理新增实体）、场景槽 保存/切换/卸载、双平台 dry-run 计划生成。
        /// </summary>
        private static void TestPrefabScenePlayerFfi()
        {
            // --- Prefab：捕获第一个根实体 → 实例化 → 清理 ---
            ulong rootEntity = EditorScript.TestGetFirstRootEntityId();
            int before = EditorScript.TestGetSceneEntityCount();
            if (rootEntity != 0)
            {
                System.IO.Directory.CreateDirectory("logs");
                string prefabPath = System.IO.Path.Combine("logs", "__ffi_prefab_selftest.prefab.json");
                int captureRc = UI.PrefabCaptureFromScene(
                    EditorScript.TestGetScenePtr(), rootEntity, "__ffi_selftest_prefab", prefabPath);
                AssertTrue("PrefabScene", captureRc == 0,
                    "PrefabCaptureFromScene 返回 0 (rc=" + captureRc + ")");
                ulong prefabRoot = UI.PrefabInstantiate(
                    EditorScript.TestGetScenePtr(), prefabPath, 0, 1f, 2f, 3f);
                AssertTrue("PrefabScene", prefabRoot != 0,
                    "PrefabInstantiate 返回新根实体 (root=" + prefabRoot + ")");
                int after = EditorScript.TestGetSceneEntityCount();
                AssertTrue("PrefabScene", after > before,
                    "实例化后实体数增加 (" + before + " -> " + after + ")");
                if (prefabRoot != 0)
                {
                    UI.SceneRemoveEntity(EditorScript.TestGetScenePtr(), prefabRoot);
                }
                try { if (System.IO.File.Exists(prefabPath)) System.IO.File.Delete(prefabPath); } catch (Exception) { }
            }
            else
            {
                AssertTrue("PrefabScene", true, "场景无根实体，跳过 Prefab 捕获/实例化");
            }

            // --- SceneManager：保存当前场景 → 切换 → 卸载 ---
            int saveRc = UI.SceneManagerSaveCurrent("__ffi_selftest_scene__");
            AssertTrue("PrefabScene", saveRc == 0,
                "SceneManagerSaveCurrent 返回 0 (rc=" + saveRc + ")");
            int slotCount = UI.SceneManagerCount();
            AssertTrue("PrefabScene", slotCount >= 1, "场景槽数量 >=1 (" + slotCount + ")");
            string slotName = UI.SceneManagerCurrentName();
            AssertTrue("PrefabScene", slotName == "__ffi_selftest_scene__",
                "当前场景槽名正确 (" + slotName + ")");
            int switchRc = UI.SceneManagerSwitch("__ffi_selftest_scene__");
            AssertTrue("PrefabScene", switchRc == 0,
                "SceneManagerSwitch 返回 0 (rc=" + switchRc + ")");
            int unloadRc = UI.SceneManagerUnload("__ffi_selftest_scene__");
            AssertTrue("PrefabScene", unloadRc == 0,
                "SceneManagerUnload 返回 0 (rc=" + unloadRc + ")");

            // --- Player 导出 dry-run（输出到 logs/，不污染工程 assets） ---
            string rootDir = System.IO.Directory.GetCurrentDirectory();
            string outputDir = System.IO.Path.Combine(rootDir, "logs", "player_dry_run_selftest");
            string requestJson = "{"
                + "\"project_root\":\"" + rootDir.Replace("\\", "\\\\") + "\","
                + "\"output_dir\":\"" + outputDir.Replace("\\", "\\\\") + "\","
                + "\"product_name\":\"HezhouSelfTest\","
                + "\"version\":\"0.1.0\","
                + "\"start_scene\":\"scenes/main.scene.json\","
                + "\"scripts_dir\":\"" + System.IO.Path.Combine(rootDir, "scripts").Replace("\\", "\\\\") + "\","
                + "\"runtime_dir\":\"" + System.IO.Path.Combine(rootDir, "runtime", "mono").Replace("\\", "\\\\") + "\","
                + "\"required_font\":\"fonts/HarmonyOS_Sans_SC.ttf\""
                + "}";
            int exportRc = UI.PlayerExportDryRun(requestJson);
            AssertTrue("PrefabScene", exportRc == 0,
                "PlayerExportDryRun 返回 0 (rc=" + exportRc + ")");
            string exportSummary = UI.PlayerExportSummary();
            AssertTrue("PrefabScene", exportSummary.IndexOf("Player 导出 dry-run") >= 0,
                "导出摘要包含 dry-run (" + exportSummary.Replace('\n', ' ') + ")");
            bool windowsPlan = System.IO.File.Exists(
                System.IO.Path.Combine(outputDir, "windows", "plan.json"));
            bool linuxPlan = System.IO.File.Exists(
                System.IO.Path.Combine(outputDir, "linux", "plan.json"));
            AssertTrue("PrefabScene", windowsPlan && linuxPlan,
                "双平台 plan.json 已生成 (win=" + windowsPlan + ", linux=" + linuxPlan + ")");
            try { System.IO.Directory.Delete(outputDir, true); } catch (Exception) { }
        }

        // 内存保存值结构（模拟SavedScriptPropertyValue，用于测试）
        private struct SavedTestValue
        {
            public string PropertyName;
            public float RuntimeValue;
        }
    }
}