# scripts

第三人称模板的工程脚本目录（0.4/1.0 工程脚本编译流程仍在完善，此目录先占位）。

本轮说明：
- 编辑器内可用 `Hezhou.Engine` API（见 `engine/scripts/CSHARP_ENGINE_API_0_3.md`）编写玩法；
- 第三人称相机/角色控制预计由 **PlayerGame**（1.0 Player C# 入口）生成完整角色与相机；
  当前 Player host 仍在接入中，模板里用 `CameraAnchor` 空实体 + `current_state` 相机参数
  表达第三人称构图；
- 平台输入约定：WASD 移动、Space 跳跃、鼠标视角（本轮无鼠标视角，见 README 限制）。
