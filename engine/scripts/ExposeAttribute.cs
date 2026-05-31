using System;

namespace Hezhou
{
    /// <summary>
    /// 标记脚本字段为编辑器可见属性。
    /// 用于属性反射系统 — 编辑器下拉选择脚本后，
    /// 带[Expose]的字段会以Input或Slider形式显示在UI中。
    /// 
    /// 用法示例:
    ///   [Expose(DisplayName="旋转速度", Category="运动", Min=0, Max=100, Step=1)]
    ///   private float rotationSpeed = 9.0f;
    /// </summary>
    [AttributeUsage(AttributeTargets.Field)]
    public class ExposeAttribute : Attribute
    {
        /// <summary>编辑器中显示的名称（中文/英文均可）</summary>
        public string DisplayName;
        
        /// <summary>属性分类（如"运动"、"渲染"、"物理"）</summary>
        public string Category;
        
        /// <summary>是否只读（默认false）</summary>
        public bool ReadOnly;
        
        /// <summary>Slider最小值（仅Float类型有效，默认0）</summary>
        public float Min;
        
        /// <summary>Slider最大值（仅Float类型有效，默认100）</summary>
        public float Max;
        
        /// <summary>Slider步长（仅Float类型有效，默认1）</summary>
        public float Step;
        
        /// <summary>初始值/默认值（编辑器中可修改，写回.cs源文件）</summary>
        public float Initial;
        
        /// <summary>UI控件类型: "input"(InputField) 或 "slider"(Slider)</summary>
        public string Widget;
        
        public ExposeAttribute()
        {
            DisplayName = "";
            Category = "脚本";
            ReadOnly = false;
            Min = 0f;
            Max = 100f;
            Step = 1f;
            Initial = 0f;
            Widget = "input";
        }
    }
    
    /// <summary>
    /// 脚本属性描述符 — C#反射系统发现[Expose]字段后生成的元数据。
    /// 通过FFI传递给Rust侧，再传回C#编辑器UI渲染属性控件。
    /// </summary>
    public struct ScriptPropertyDescriptor
    {
        /// <summary>字段名（用于get/set值）</summary>
        public string Name;
        
        /// <summary>显示名（来自[Expose]的DisplayName）</summary>
        public string DisplayName;
        
        /// <summary>属性类型: 0=Float, 1=Float3, 2=String, 3=Bool, 4=Int, 5=Enum</summary>
        public uint Type;
        
        /// <summary>分类（来自[Expose]的Category）</summary>
        public string Category;
        
        /// <summary>是否只读</summary>
        public bool ReadOnly;
        
        /// <summary>Slider最小值（仅Float+Slider有效）</summary>
        public float Min;
        
        /// <summary>Slider最大值</summary>
        public float Max;
        
        /// <summary>Slider步长</summary>
        public float Step;
        
        /// <summary>初始值/默认值（来自[Expose]的Initial，写回.cs源文件）</summary>
        public float Initial;
        
        /// <summary>UI控件: "input" 或 "slider"</summary>
        public string Widget;
    }
}