using System;
using System.Reflection;
using Hezhou;

namespace HezhouScripts
{
    /// <summary>
    /// 脚本实体辅助工具类 — 为per-entity脚本实例系统提供反射和实例查找的通用工具。
    /// 
    /// 由于mcs编译器（Mono）不支持C# 8.0的静态接口方法，
    /// 我们无法定义正式的IScriptEntity接口。
    /// 取而代之，每个脚本实体类必须遵循以下约定（CONVENTION）：
    /// 
    /// ━━━ 约定方法（每个脚本实体类必须实现这些静态方法）━━━
    /// 
    ///   1. static ScriptPropertyDescriptor[] GetPropertyDescriptors()
    ///      — 返回属性描述符数组，通常调用 ScriptEntityHelper.ReflectProperties(typeof(X))
    ///      — Rust侧调用此方法发现[Expose]字段，用于编辑器UI渲染
    /// 
    ///   2. static float GetPropertyValue(IntPtr instancePtr, string propertyName)
    ///      — 读取实例属性值，通常调用 ScriptEntityHelper.GetFieldValue(instancePtr, name, typeof(X))
    ///      — instancePtr是字典key（long），不是原始MonoObject指针
    /// 
    ///   3. static void SetPropertyValue(IntPtr instancePtr, string propertyName, float value)
    ///      — 写入实例属性值，通常调用 ScriptEntityHelper.SetFieldValue(instancePtr, name, value, typeof(X))
    ///      — 编辑器UI修改属性时，Rust侧调用此方法
    /// 
    ///   4. static void UpdateInstance(IntPtr instancePtr, float deltaTime, ulong entityId)
    ///      — 每帧更新逻辑，Rust侧每帧调用
    ///      — instancePtr用于从_instances字典查找实例
    ///      — entityId用于FFI调用（如SceneRotateEntity）
    /// 
    /// ━━━ 约定字段（每个脚本实体类必须声明）━━━
    /// 
    ///   - static Dictionary<long, T> _instances — 实例存储字典（GC安全）
    ///   - static long _nextInstanceId — 下一个实例ID计数器（从1开始）
    /// 
    /// ━━━ 约定生命周期方法（可选但推荐）━━━
    /// 
    ///   - static IntPtr CreateInstance() — 创建新实例，返回key
    ///   - static void DestroyInstance(IntPtr instancePtr) — 销毁实例
    ///   - static void ResetAll() — 热重载时清除所有实例
    /// 
    /// ━━━ 新增约定方法（方向7: GamePlay System调度 + 生命周期扩展）━━━
    /// 
    ///   5. static void OnStart(IntPtr instancePtr, ulong entityId)
    ///      — 首帧调用，脚本实例创建后第一次Update前调用
    ///      — 用于初始化逻辑（如查找其他Entity、设置初始状态）
    ///      — Rust侧通过on_start_thunk在ScriptSystem首帧调用
    /// 
    ///   6. static void OnCollisionStart(ulong entityId, ulong otherEntityId)
    ///      — 碰撞开始时调用，entityId是当前实体，otherEntityId是碰撞对象
    ///      — Rust侧通过on_collision_start_thunk在EventFlushSystem中调用
    ///      — 注意：双向通知，两个碰撞实体都会收到回调
    /// 
    ///   7. static void OnCollisionStop(ulong entityId, ulong otherEntityId)
    ///      — 碰撞结束时调用
    ///      — Rust侧通过on_collision_stop_thunk在EventFlushSystem中调用
    /// 
    ///   8. static void OnInputEvent(uint keyCode, uint eventType)
    ///      — 键盘/鼠标输入事件，eventType: 0=Press, 1=Release, 2=Repeat
    ///      — Rust侧通过on_input_event_thunk调用
    ///      — keyCode定义: Left=45, Right=46, Up=47, Down=48, ESC=39
    /// 
    /// ━━━ 设计原理 ━━━
    /// 
    ///   为什么用Dictionary而不是直接cast IntPtr→object？
    ///   → Mono运行时中，C#对象由GC管理。如果Rust侧只持有MonoObject*，
    ///     GC可能回收该对象（因为Mono不知道Rust持有引用）。
    ///   → 用C#静态Dictionary持有强引用，确保GC不会回收实例。
    ///   → Rust侧存储的是字典key（整数），通过IntPtr传递，安全且简单。
    /// 
    ///   为什么instancePtr是字典key而不是MonoObject指针？
    ///   → 避免Rust侧直接操作Mono GC堆（unsafe且易出错）
    ///   → Dictionary查找是O(1)，性能足够
    ///   → key是简单整数，跨FFI传递无marshalling开销
    /// </summary>
    public class ScriptEntityHelper
    {
        /// <summary>
        /// 通过System.Reflection扫描脚本类型上带[Expose]属性的字段，
        /// 生成ScriptPropertyDescriptor数组。
        /// 
        /// mcs兼容：使用手动循环而非LINQ，使用GetCustomAttributes(Type, bool)而非泛型版本。
        /// </summary>
        /// <param name="scriptType">脚本实体类的Type（如typeof(RotatingEntity)）</param>
        /// <returns>属性描述符数组，每个[Expose]字段对应一个描述符</returns>
        public static ScriptPropertyDescriptor[] ReflectProperties(Type scriptType)
        {
            // 获取所有实例字段（包括private，因为[Expose]常标记private字段）
            FieldInfo[] fields = scriptType.GetFields(
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic
            );

            // 先计数有多少[Expose]字段（避免动态数组，mcs兼容）
            int exposeCount = 0;
            for (int i = 0; i < fields.Length; i++)
            {
                if (fields[i].IsDefined(typeof(ExposeAttribute), false))
                {
                    exposeCount++;
                }
            }

            // 创建固定大小数组
            ScriptPropertyDescriptor[] descriptors = new ScriptPropertyDescriptor[exposeCount];
            int index = 0;

            for (int i = 0; i < fields.Length; i++)
            {
                FieldInfo field = fields[i];
                object[] attrs = field.GetCustomAttributes(typeof(ExposeAttribute), false);

                if (attrs != null && attrs.Length > 0)
                {
                    ExposeAttribute attr = (ExposeAttribute)attrs[0];

                    descriptors[index] = new ScriptPropertyDescriptor();
                    descriptors[index].Name = field.Name;
                    descriptors[index].DisplayName = attr.DisplayName;
                    descriptors[index].Type = MapFieldTypeToPropertyType(field.FieldType);
                    descriptors[index].Category = attr.Category;
                    descriptors[index].ReadOnly = attr.ReadOnly;
                    descriptors[index].Min = attr.Min;
                    descriptors[index].Max = attr.Max;
                    descriptors[index].Step = attr.Step;
                    descriptors[index].Initial = attr.Initial;
                    descriptors[index].Widget = attr.Widget;

                    index++;
                }
            }

            return descriptors;
        }

        /// <summary>
        /// 将C#字段类型映射到ScriptPropertyDescriptor类型码。
        /// 类型码定义：0=Float, 1=Float3, 2=String, 3=Bool, 4=Int, 5=Enum
        /// </summary>
        private static uint MapFieldTypeToPropertyType(Type fieldType)
        {
            if (fieldType == typeof(float))
                return 0; // Float
            if (fieldType == typeof(int))
                return 4; // Int
            if (fieldType == typeof(bool))
                return 3; // Bool
            if (fieldType == typeof(string))
                return 2; // String
            if (fieldType.IsEnum)
                return 5; // Enum

            // 默认：未知类型当作Float处理
            return 0;
        }

        /// <summary>
        /// 从实例字典中查找实例，并读取指定字段的float值。
        /// 
        /// instancePtr是字典key（由Rust侧传入的IntPtr），不是原始MonoObject指针。
        /// 内部流程：
        ///   1. 通过反射查找scriptType的_instances静态字段
        ///   2. 获取字典对象
        ///   3. 用instancePtr.ToInt64()作为key查找实例
        ///   4. 用反射读取实例上指定字段的值
        /// </summary>
        /// <param name="instancePtr">实例字典key（IntPtr形式）</param>
        /// <param name="fieldName">要读取的字段名</param>
        /// <param name="scriptType">脚本实体类的Type</param>
        /// <returns>字段值（float），找不到返回0.0f</returns>
        public static float GetFieldValue(IntPtr instancePtr, string fieldName, Type scriptType)
        {
            object instance = LookupInstance(instancePtr, scriptType);
            if (instance == null)
            {
                return 0.0f;
            }

            FieldInfo field = scriptType.GetField(
                fieldName,
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic
            );
            if (field == null)
            {
                return 0.0f;
            }

            object value = field.GetValue(instance);
            if (value is float)
            {
                return (float)value;
            }

            return 0.0f;
        }

        /// <summary>
        /// 从实例字典中查找实例，并写入指定字段的float值。
        /// 
        /// 内部流程同GetFieldValue，但最后用field.SetValue写入值。
        /// 仅对float类型字段有效，其他类型忽略。
        /// </summary>
        /// <param name="instancePtr">实例字典key（IntPtr形式）</param>
        /// <param name="fieldName">要写入的字段名</param>
        /// <param name="value">要写入的float值</param>
        /// <param name="scriptType">脚本实体类的Type</param>
        public static void SetFieldValue(IntPtr instancePtr, string fieldName, float value, Type scriptType)
        {
            object instance = LookupInstance(instancePtr, scriptType);
            if (instance == null)
            {
                return;
            }

            FieldInfo field = scriptType.GetField(
                fieldName,
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic
            );
            if (field == null)
            {
                return;
            }

            // 仅对float类型字段写入（当前属性系统只支持float）
            if (field.FieldType == typeof(float))
            {
                field.SetValue(instance, value);
            }
        }

        /// <summary>
        /// 通过反射从脚本类型的_instances静态字典中查找实例。
        /// 
        /// 流程：
        ///   1. 查找名为"_instances"的静态字段
        ///   2. GetValue(null)获取字典对象（静态字段不需要实例）
        ///   3. 用ContainsKey检查key是否存在
        ///   4. 用索引器(Item属性)获取实例对象
        /// 
        /// mcs兼容：使用PropertyInfo.GetValue访问字典索引器，
        ///           使用MethodInfo.Invoke调用ContainsKey。
        /// </summary>
        /// <param name="instancePtr">实例字典key</param>
        /// <param name="scriptType">脚本实体类的Type</param>
        /// <returns>实例对象，找不到返回null</returns>
        private static object LookupInstance(IntPtr instancePtr, Type scriptType)
        {
            long key = instancePtr.ToInt64();

            // 查找_instances静态字段
            FieldInfo instancesField = scriptType.GetField(
                "_instances",
                BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic
            );
            if (instancesField == null)
            {
                return null;
            }

            // 获取字典对象
            object dictionary = instancesField.GetValue(null);
            if (dictionary == null)
            {
                return null;
            }

            // 检查key是否存在 — 用反射调用ContainsKey
            MethodInfo containsKeyMethod = dictionary.GetType().GetMethod(
                "ContainsKey", new Type[] { typeof(long) }
            );
            if (containsKeyMethod != null)
            {
                bool exists = (bool)containsKeyMethod.Invoke(dictionary, new object[] { key });
                if (!exists)
                {
                    return null;
                }
            }

            // 通过索引器获取实例 — 用反射访问Item属性
            PropertyInfo indexer = dictionary.GetType().GetProperty(
                "Item", new Type[] { typeof(long) }
            );
            if (indexer == null)
            {
                return null;
            }

            try
            {
                return indexer.GetValue(dictionary, new object[] { key });
            }
            catch (Exception)
            {
                // Key不存在或类型不匹配时返回null
                return null;
            }
        }
    }
}