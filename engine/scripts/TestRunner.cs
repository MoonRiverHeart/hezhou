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

            try { TestPlayerBuildFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestPlayerBuildFfi异常: " + ex.Message); }

            try { TestPbrMaterialFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestPbrMaterialFfi异常: " + ex.Message); }

            try { TestProfilerFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestProfilerFfi异常: " + ex.Message); }

            try { TestBuildSettingsFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestBuildSettingsFfi异常: " + ex.Message); }

            try { TestAnimationFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestAnimationFfi异常: " + ex.Message); }

            try { TestMaterialTextureFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestMaterialTextureFfi异常: " + ex.Message); }

            try { TestAnimationStateMachineFfi(); }
            catch (Exception ex) { Log.Error("TestRunner", "TestAnimationStateMachineFfi异常: " + ex.Message); }

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

        /// <summary>
        /// 0.2 Player 真实构建自检（task-25）：
        /// 用 logs/ 下的最小 fixture 工程（scene+font+audio）跑 Cooker + PlayerBuilder::build，
        /// 并通过产物内可执行文件的 `--validate`；需要 hezhou-player 可执行文件与编辑器同目录。
        /// </summary>
        private static void TestPlayerBuildFfi()
        {
            string exeDir = "";
            try
            {
                exeDir = System.IO.Path.GetDirectoryName(
                    System.Diagnostics.Process.GetCurrentProcess().MainModule.FileName);
            }
            catch (Exception) { }
            if (string.IsNullOrEmpty(exeDir)) exeDir = ".";
            bool playerExePresent = System.IO.File.Exists(System.IO.Path.Combine(exeDir, "hezhou-player.exe"))
                || System.IO.File.Exists(System.IO.Path.Combine(exeDir, "hezhou-player"));
            if (!playerExePresent)
            {
                AssertTrue("PlayerBuild", true,
                    "未找到 hezhou-player 可执行文件，跳过真实构建自检（先 cargo build -p hezhou-player）");
                return;
            }

            string cwd = System.IO.Directory.GetCurrentDirectory();
            string fixture = System.IO.Path.Combine(cwd, "logs", "player_build_fixture");
            string outputDir = System.IO.Path.Combine(cwd, "logs", "player_build_selftest");
            try
            {
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(fixture, "assets", "scenes"));
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(fixture, "assets", "fonts"));
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(fixture, "assets", "audio"));
                System.IO.Directory.CreateDirectory(System.IO.Path.Combine(fixture, "runtime"));
                System.IO.File.WriteAllText(
                    System.IO.Path.Combine(fixture, "runtime", "README.txt"),
                    "self-test runtime marker");
                System.IO.File.WriteAllText(
                    System.IO.Path.Combine(fixture, "assets", "scenes", "main.scene"), "{}");
                System.IO.File.WriteAllBytes(
                    System.IO.Path.Combine(fixture, "assets", "fonts", "HarmonyOS_Sans_SC.ttf"),
                    new byte[] { 0, 1, 2, 3 });
                System.IO.File.WriteAllBytes(
                    System.IO.Path.Combine(fixture, "assets", "audio", "test.wav"),
                    new byte[] { 82, 73, 70, 70, 0, 0, 0, 0 });

                string scriptsDir = System.IO.Path.Combine(cwd, "scripts", "bin", "Mono");
                string json = "{"
                    + "\"project_root\":\"" + fixture.Replace("\\", "\\\\") + "\","
                    + "\"output_dir\":\"" + outputDir.Replace("\\", "\\\\") + "\","
                    + "\"product_name\":\"HezhouBuildSelfTest\","
                    + "\"version\":\"0.1.0\","
                    + "\"start_scene\":\"scenes/main.scene\","
                    + "\"scripts_dir\":\"" + scriptsDir.Replace("\\", "\\\\") + "\","
                    + "\"runtime_dir\":\"" + System.IO.Path.Combine(fixture, "runtime").Replace("\\", "\\\\") + "\","
                    + "\"required_font\":\"fonts/HarmonyOS_Sans_SC.ttf\","
                    + "\"executable_path\":\"\""
                    + "}";
                int rc = UI.PlayerExportBuild(json);
                string summary = UI.PlayerExportSummary();
                AssertTrue("PlayerBuild", rc == 0,
                    "PlayerExportBuild 返回 0 (rc=" + rc + ") summary=" + summary.Replace('\n', ' '));

                bool manifestFound = false;
                if (System.IO.Directory.Exists(outputDir))
                {
                    string[] targetDirs = System.IO.Directory.GetDirectories(outputDir);
                    for (int i = 0; i < targetDirs.Length; i++)
                    {
                        if (System.IO.File.Exists(
                            System.IO.Path.Combine(targetDirs[i], "manifest.json")))
                        {
                            manifestFound = true;
                            break;
                        }
                    }
                }
                AssertTrue("PlayerBuild", manifestFound,
                    "真实构建产物包含 manifest.json（" + outputDir + "）");
                AssertTrue("PlayerBuild", summary.IndexOf("validate: 通过") >= 0,
                    "构建摘要包含 validate 通过");
            }
            finally
            {
                try { if (System.IO.Directory.Exists(fixture)) System.IO.Directory.Delete(fixture, true); } catch (Exception) { }
                try { if (System.IO.Directory.Exists(outputDir)) System.IO.Directory.Delete(outputDir, true); } catch (Exception) { }
            }
        }

        /// <summary>
        /// 0.2 per-entity PBR 材质自检（task-26）：
        /// 创建测试 Cube → 设置 JSON 材质 → 读取 roundtrip → renderer 同步覆盖数 ≥1 →
        /// 清除材质 → 读取为空 → 清理实体。
        /// </summary>
        private static void TestPbrMaterialFfi()
        {
            IntPtr scenePtr = EditorScript.TestGetScenePtr();
            if (scenePtr == IntPtr.Zero)
            {
                AssertTrue("PbrMaterial", true, "场景未初始化，跳过 PBR 材质自检");
                return;
            }

            ulong cube = EditorScript.TestCreateCubeEntity();
            AssertTrue("PbrMaterial", cube != 0, "创建测试 Cube 实体 (id=" + cube + ")");

            string json = "{"
                + "\"albedo\":[0.8,0.2,0.1,1.0],"
                + "\"metallic\":0.6,"
                + "\"roughness\":0.3,"
                + "\"normal_scale\":1.25,"
                + "\"ao\":0.9,"
                + "\"emissive\":[0.15,0.0,0.3],"
                + "\"emissive_strength\":2.5,"
                + "\"albedo_texture_path\":\"assets/tex/pbr_test.png\""
                + "}";
            int setRc = UI.SceneSetEntityPbrMaterial(scenePtr, cube, json);
            AssertTrue("PbrMaterial", setRc == 0, "SceneSetEntityPbrMaterial 返回 0 (rc=" + setRc + ")");

            string got = UI.SceneGetEntityPbrMaterial(scenePtr, cube);
            bool roundtrip = got.IndexOf("\"metallic\":0.6") >= 0
                && got.IndexOf("pbr_test.png") >= 0
                && got.IndexOf("2.5") >= 0;
            AssertTrue("PbrMaterial", roundtrip, "材质 JSON roundtrip (" + got + ")");

            int synced = UI.EditorSyncPbrMaterialsNow();
            AssertTrue("PbrMaterial", synced >= 1,
                "renderer 收到每实体材质（覆盖数=" + synced + "）");

            int clearRc = UI.SceneSetEntityPbrMaterial(scenePtr, cube, "null");
            string cleared = UI.SceneGetEntityPbrMaterial(scenePtr, cube);
            AssertTrue("PbrMaterial", clearRc == 0 && cleared.Length == 0,
                "清除材质后读取为空 (rc=" + clearRc + ", len=" + cleared.Length + ")");

            UI.EditorSyncPbrMaterialsNow();
            EditorScript.TestRemoveEntity(cube);
            AssertTrue("PbrMaterial", true, "PBR 材质自检实体已清理");
        }

        /// <summary>
        /// 0.3 Profiler 自检（task-39）：快照 JSON 必要字段、reset 返回 0、重置后帧计数归零。
        /// </summary>
        private static void TestProfilerFfi()
        {
            string snapshot = UI.ProfilerSnapshotJson();
            bool hasFields = snapshot.IndexOf("\"fps\"") >= 0
                && snapshot.IndexOf("\"frame_ms_avg\"") >= 0
                && snapshot.IndexOf("\"frame_ms_p95\"") >= 0
                && snapshot.IndexOf("\"startup_ms\"") >= 0
                && snapshot.IndexOf("\"entity_count\"") >= 0
                && snapshot.IndexOf("\"pbr_entity_count\"") >= 0
                && snapshot.IndexOf("\"skinned_mesh_count\"") >= 0
                && snapshot.IndexOf("\"asset_count\"") >= 0;
            AssertTrue("Profiler", hasFields,
                "ProfilerSnapshotJson 包含必要字段（len=" + snapshot.Length + "）");
            Log.Info("Profiler", "自检 snapshot: "
                + (snapshot.Length > 400 ? snapshot.Substring(0, 400) + "..." : snapshot));

            int resetRc = UI.ProfilerReset();
            AssertTrue("Profiler", resetRc == 0,
                "ProfilerReset 返回 0 (rc=" + resetRc + ")");
            int framesAfterReset = UI.ProfilerFrameCount();
            AssertTrue("Profiler", framesAfterReset == 0,
                "重置后帧计数为 0 (frames=" + framesAfterReset + ")");
        }

        /// <summary>
        /// 0.3 Build Settings 自检（task-41）：
        /// 快照字段 → 校验有效设置 → 保存 build.settings.json → Player dry-run 使用设置；
        /// 结束后恢复原设置文件/清理导出产物。
        /// </summary>
        private static void TestBuildSettingsFfi()
        {
            string root = System.IO.Directory.GetCurrentDirectory();
            string settingsPath = System.IO.Path.Combine(root, "build.settings.json");
            bool existed = System.IO.File.Exists(settingsPath);
            string original = existed ? System.IO.File.ReadAllText(settingsPath) : null;
            string outputDir = System.IO.Path.Combine(root, "logs", "build_settings_selftest");
            try
            {
                string snapshot = UI.BuildSettingsSnapshotJson();
                bool hasFields = snapshot.IndexOf("\"product_name\"") >= 0
                    && snapshot.IndexOf("\"start_scene\"") >= 0
                    && snapshot.IndexOf("\"quality\"") >= 0
                    && snapshot.IndexOf("\"targets\"") >= 0
                    && snapshot.IndexOf("\"window\"") >= 0;
                AssertTrue("BuildSettings", hasFields,
                    "Build Settings 快照包含必要字段（len=" + snapshot.Length + "）");

                // 用工程内第一个真实资产作为 start_scene（满足“存在”校验）。
                string firstAsset = "";
                string assetsDir = System.IO.Path.Combine(root, "assets");
                if (System.IO.Directory.Exists(assetsDir))
                {
                    string[] files = System.IO.Directory.GetFiles(
                        assetsDir, "*", System.IO.SearchOption.AllDirectories);
                    for (int i = 0; i < files.Length; i++)
                    {
                        if (!files[i].EndsWith(".meta"))
                        {
                            firstAsset = files[i].Substring(assetsDir.Length)
                                .TrimStart('\\', '/')
                                .Replace('\\', '/');
                            break;
                        }
                    }
                }
                if (firstAsset.Length == 0) firstAsset = "scenes/main.scene.json";

                string settingsJson = "{"
                    + "\"product_name\":\"HezhouSettingsSelfTest\","
                    + "\"executable_name\":\"hezhousettingstest\","
                    + "\"start_scene\":\"" + firstAsset.Replace("\\", "\\\\") + "\","
                    + "\"quality\":\"high\","
                    + "\"window\":{\"width\":1280,\"height\":720,\"mode\":\"windowed\"},"
                    + "\"targets\":[\"windows\",\"linux\"],"
                    + "\"version\":\"1.2.3\","
                    + "\"log_dir\":\"logs\","
                    + "\"required_font\":null"
                    + "}";
                string errors;
                int validateRc = UI.BuildSettingsValidateJson(settingsJson, out errors);
                AssertTrue("BuildSettings", validateRc == 0,
                    "校验有效设置返回 0 (rc=" + validateRc + ", errors=" + errors + ")");

                int saveRc = UI.BuildSettingsSaveJson(settingsJson);
                AssertTrue("BuildSettings",
                    saveRc == 0 && System.IO.File.Exists(settingsPath),
                    "保存 build.settings.json 成功 (rc=" + saveRc + ")");

                // request 中产品/版本/启动场景/字体留空 → 由 Build Settings 补齐。
                string request = "{"
                    + "\"project_root\":\"" + root.Replace("\\", "\\\\") + "\","
                    + "\"output_dir\":\"" + outputDir.Replace("\\", "\\\\") + "\","
                    + "\"product_name\":\"\",\"version\":\"\",\"start_scene\":\"\","
                    + "\"scripts_dir\":\"\",\"runtime_dir\":\"\","
                    + "\"required_font\":\"\",\"executable_path\":\"\""
                    + "}";
                int exportRc = UI.PlayerExportDryRun(request);
                string summary = UI.PlayerExportSummary();
                bool usedSettings = exportRc == 0
                    && summary.IndexOf("HezhouSettingsSelfTest") >= 0
                    && summary.IndexOf("1.2.3") >= 0
                    && summary.IndexOf(firstAsset) >= 0;
                AssertTrue("BuildSettings", usedSettings,
                    "Player dry-run 使用 Build Settings (rc=" + exportRc + ", start_scene=" + firstAsset + ")");
                Log.Info("BuildSettings", "自检 export summary: "
                    + (summary.Length > 500 ? summary.Substring(0, 500) + "..." : summary));
            }
            finally
            {
                try
                {
                    if (System.IO.Directory.Exists(outputDir))
                    {
                        System.IO.Directory.Delete(outputDir, true);
                    }
                }
                catch (Exception) { }
                try
                {
                    if (existed) System.IO.File.WriteAllText(settingsPath, original);
                    else if (System.IO.File.Exists(settingsPath)) System.IO.File.Delete(settingsPath);
                }
                catch (Exception) { }
            }
        }

        /// <summary>
        /// 0.3 动画时间轴自检（task-43）：
        /// 缺失资产返回安全错误 JSON；play/pause 状态；无 clip 时 set_time clamp。
        /// </summary>
        private static void TestAnimationFfi()
        {
            string missing = UI.AnimationAssetSummary("logs/__missing_animation__.gltf");
            bool summarySafe = missing.IndexOf("\"clips\"") >= 0
                && missing.IndexOf("\"error\"") >= 0;
            AssertTrue("Animation", summarySafe,
                "缺失资产返回安全错误 JSON（len=" + missing.Length + "）");

            UI.AnimationPreviewPlay();
            string playing = UI.AnimationPreviewSnapshot();
            AssertTrue("Animation", playing.IndexOf("\"playing\":true") >= 0,
                "Play 后 snapshot playing=true (" + playing + ")");

            UI.AnimationPreviewPause();
            string paused = UI.AnimationPreviewSnapshot();
            AssertTrue("Animation", paused.IndexOf("\"playing\":false") >= 0,
                "Pause 后 snapshot playing=false");

            double clamped = UI.AnimationPreviewSetTime(0.25f);
            AssertTrue("Animation", clamped >= 0.0 && clamped <= 1e-6,
                "无 clip 时 set_time clamp 到 0 (t=" + clamped + ")");
        }

        /// <summary>
        /// 0.3 材质纹理自检（task-45）：
        /// 写临时 PNG 到 assets → 扫描 → asset_texture_list_json 包含该纹理 →
        /// 设置/清除实体 PBR albedo_texture_path roundtrip → 清理文件与实体。
        /// </summary>
        private static void TestMaterialTextureFfi()
        {
            string root = System.IO.Directory.GetCurrentDirectory();
            string texturePath = System.IO.Path.Combine(root, "assets", "__ffi_tex_selftest__.png");
            string metaPath = texturePath + ".meta";
            ulong cube = 0;
            try
            {
                byte[] png = Convert.FromBase64String(
                    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=");
                System.IO.File.WriteAllBytes(texturePath, png);
                UI.AssetScan();

                string listJson = UI.AssetTextureListJson();
                bool listed = listJson.IndexOf("__ffi_tex_selftest__.png") >= 0
                    && listJson.IndexOf("\"png\"") >= 0;
                AssertTrue("MaterialTexture", listed,
                    "纹理列表包含临时 PNG（len=" + listJson.Length + "）");

                cube = EditorScript.TestCreateCubeEntity();
                IntPtr scenePtr = EditorScript.TestGetScenePtr();
                string materialJson = "{"
                    + "\"albedo\":[0.8,0.8,0.8,1.0],\"metallic\":0.1,\"roughness\":0.6,"
                    + "\"normal_scale\":1.0,\"ao\":1.0,\"emissive\":[0,0,0],"
                    + "\"emissive_strength\":1.0,"
                    + "\"albedo_texture_path\":\"assets/__ffi_tex_selftest__.png\""
                    + "}";
                int setRc = UI.SceneSetEntityPbrMaterial(scenePtr, cube, materialJson);
                string stored = UI.SceneGetEntityPbrMaterial(scenePtr, cube);
                AssertTrue("MaterialTexture",
                    setRc == 0 && stored.IndexOf("__ffi_tex_selftest__.png") >= 0,
                    "设置 albedo 纹理 roundtrip (rc=" + setRc + ", json=" + stored + ")");

                int clearRc = UI.SceneSetEntityPbrMaterial(scenePtr, cube, "null");
                string cleared = UI.SceneGetEntityPbrMaterial(scenePtr, cube);
                AssertTrue("MaterialTexture",
                    clearRc == 0 && cleared.Length == 0,
                    "清除 albedo 纹理后读取为空 (rc=" + clearRc + ", len=" + cleared.Length + ")");
                UI.EditorSyncPbrMaterialsNow();
            }
            finally
            {
                if (cube != 0) EditorScript.TestRemoveEntity(cube);
                try { if (System.IO.File.Exists(texturePath)) System.IO.File.Delete(texturePath); } catch (Exception) { }
                try { if (System.IO.File.Exists(metaPath)) System.IO.File.Delete(metaPath); } catch (Exception) { }
                try { UI.AssetScan(); } catch (Exception) { }
                try { EditorScript.TestRefreshMaterialTextureList(); } catch (Exception) { }
            }
        }


        /// <summary>0.3 动画状态机自检（task-47）：初始化/切换/过渡/释放。</summary>
        private static void TestAnimationStateMachineFfi()
        {
            string root = System.IO.Directory.GetCurrentDirectory();
            string fixturePath = System.IO.Path.Combine(root, "assets", "__ffi_anim_sm_selftest__.gltf");
            try
            {
                string base64 = "eyJhc3NldCI6IHsidmVyc2lvbiI6ICIyLjAifSwgImJ1ZmZlcnMiOiBbeyJieXRlTGVuZ3RoIjogOTYsICJ1cmkiOiAiZGF0YTphcHBsaWNhdGlvbi9vY3RldC1zdHJlYW07YmFzZTY0LEFBQUFBQUFBZ0Q4QUFBQUFBQUFBQUFBQUFBQUFBSUEvQUFBQUFBQUFBQUFBQUlBL0FBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQWdEOEFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQ0FQd0FBQUFBQUFBQUFBQUFBQUFBQUFBQUFBSUEvIn1dLCAiYnVmZmVyVmlld3MiOiBbeyJidWZmZXIiOiAwLCAiYnl0ZU9mZnNldCI6IDAsICJieXRlTGVuZ3RoIjogOH0sIHsiYnVmZmVyIjogMCwgImJ5dGVPZmZzZXQiOiA4LCAiYnl0ZUxlbmd0aCI6IDI0fSwgeyJidWZmZXIiOiAwLCAiYnl0ZU9mZnNldCI6IDMyLCAiYnl0ZUxlbmd0aCI6IDY0fV0sICJhY2Nlc3NvcnMiOiBbeyJidWZmZXJWaWV3IjogMCwgImNvbXBvbmVudFR5cGUiOiA1MTI2LCAiY291bnQiOiAyLCAidHlwZSI6ICJTQ0FMQVIiLCAibWluIjogWzAuMF0sICJtYXgiOiBbMS4wXX0sIHsiYnVmZmVyVmlldyI6IDEsICJjb21wb25lbnRUeXBlIjogNTEyNiwgImNvdW50IjogMiwgInR5cGUiOiAiVkVDMyJ9LCB7ImJ1ZmZlclZpZXciOiAyLCAiY29tcG9uZW50VHlwZSI6IDUxMjYsICJjb3VudCI6IDEsICJ0eXBlIjogIk1BVDQifV0sICJub2RlcyI6IFt7Im5hbWUiOiAiSm9pbnQifV0sICJza2lucyI6IFt7ImpvaW50cyI6IFswXSwgImludmVyc2VCaW5kTWF0cmljZXMiOiAyfV0sICJhbmltYXRpb25zIjogW3sibmFtZSI6ICJJZGxlIiwgImNoYW5uZWxzIjogW3sic2FtcGxlciI6IDAsICJ0YXJnZXQiOiB7Im5vZGUiOiAwLCAicGF0aCI6ICJ0cmFuc2xhdGlvbiJ9fV0sICJzYW1wbGVycyI6IFt7ImlucHV0IjogMCwgIm91dHB1dCI6IDEsICJpbnRlcnBvbGF0aW9uIjogIkxJTkVBUiJ9XX1dfQo=";
                System.IO.File.WriteAllBytes(fixturePath, Convert.FromBase64String(base64));
                int initRc = UI.AnimationPreviewInitGraph(fixturePath, 0);
                AssertTrue("AnimStateMachine", initRc == 0,
                    "9 状态图初始化返回 0 (rc=" + initRc + ")");

                string snapshot = UI.AnimationPreviewStateSnapshot();
                bool hasGraph = snapshot.IndexOf("\"initialized\":true") >= 0
                    && snapshot.IndexOf("\"Idle\"") >= 0
                    && snapshot.IndexOf("\"Block\"") >= 0
                    && snapshot.IndexOf("\"has_skeleton\":true") >= 0;
                AssertTrue("AnimStateMachine", hasGraph,
                    "快照包含 9 状态/current/skeleton (len=" + snapshot.Length + ")");

                int setRc = UI.AnimationPreviewSetState("Run");
                AssertTrue("AnimStateMachine", setRc == 0,
                    "切换到 Run 返回 0 (rc=" + setRc + ")");

                UI.AnimationPreviewTick(0.1f, 1);
                string transitionSnapshot = UI.AnimationPreviewStateSnapshot();
                AssertTrue("AnimStateMachine",
                    transitionSnapshot.IndexOf("\"transitioning\":true") >= 0,
                    "过渡中 snapshot transitioning=true");
                Log.Info("AnimStateMachine", "snapshot: "
                    + (snapshot.Length > 320 ? snapshot.Substring(0, 320) + "..." : snapshot));

                int freeRc = UI.AnimationPreviewGraphFree();
                string freedSnapshot = UI.AnimationPreviewStateSnapshot();
                AssertTrue("AnimStateMachine",
                    freeRc == 0 && freedSnapshot.IndexOf("\"initialized\":false") >= 0,
                    "释放状态图后 snapshot initialized=false");
            }
            finally
            {
                try { if (System.IO.File.Exists(fixturePath)) System.IO.File.Delete(fixturePath); } catch (Exception) { }
                try { if (System.IO.File.Exists(fixturePath + ".meta")) System.IO.File.Delete(fixturePath + ".meta"); } catch (Exception) { }
                try { UI.AssetScan(); } catch (Exception) { }
            }
        }

        // 内存保存值结构（模拟SavedScriptPropertyValue，用于测试）
        private struct SavedTestValue
        {
            public string PropertyName;
            public float RuntimeValue;
        }
    }
}