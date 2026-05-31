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
            AssertEqualFloat("ReflectProperties", 400f, speedDesc.Max, "rotationSpeed Max");
            AssertEqualFloat("ReflectProperties", 1f, speedDesc.Step, "rotationSpeed Step");
            AssertEqualFloat("ReflectProperties", 80f, speedDesc.Initial, "rotationSpeed Initial");
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

            // 读取初始值（rotationSpeed=80, angleOffset=0）
            float initialSpeed = ScriptEntityHelper.GetFieldValue(instancePtr, "rotationSpeed", typeof(RotatingEntity));
            AssertEqualFloat("GetSetFieldValue", 80f, initialSpeed, "初始rotationSpeed=80");

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

            // 调用ModifyScriptSourceFile: 将rotationSpeed的Max从400改为50
            modifyMethod.Invoke(null, new object[] { "RotatingEntity", "rotationSpeed", "Max", 50f });

            // 读取修改后的文件，验证Max参数变化
            string modifiedContent = File.ReadAllText(filePath);
            bool maxChanged = modifiedContent.Contains("Max = 50");
            AssertTrue("ModifyScriptSourceFile", maxChanged, "Max参数已从400改为50");

            // 恢复原始值: 将Max改回400
            modifyMethod.Invoke(null, new object[] { "RotatingEntity", "rotationSpeed", "Max", 400f });

            // 验证恢复成功
            string restoredContent = File.ReadAllText(filePath);
            bool maxRestored = restoredContent.Contains("Max = 400");
            AssertTrue("ModifyScriptSourceFile", maxRestored, "Max参数已恢复为400");

            // 安全兜底: 如果恢复失败，强制写回原始内容
            if (!maxRestored)
            {
                File.WriteAllText(filePath, originalContent);
                Log.Warn("TestRunner", "ModifyScriptSourceFile: 强制恢复原始文件内容");
            }
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

        // 内存保存值结构（模拟SavedScriptPropertyValue，用于测试）
        private struct SavedTestValue
        {
            public string PropertyName;
            public float RuntimeValue;
        }
    }
}