//! 后续版本预留接口。
//!
//! 0.3 第一批已落地：基础 CCD（球/胶囊 vs 静态）与距离关节，见 `ccd` / `joint`。
//! 以下能力仍在后续版本规划中：
//!
//! TODO(0.4)：固定关节、铰链、弹簧、角度限制等完整约束求解；关节锚点随刚体旋转。
//! TODO(0.4)：布娃娃（ragdoll）与骨骼关节链。
//! TODO(0.4)：动态-动态 CCD、旋转盒 CCD、子步进（sub-stepping）。
//! TODO(0.4)：旋转碰撞响应（角冲量/惯性张量）与任意朝向碰撞体。

/// 完整关节/约束求解器预留接口（固定关节、铰链、弹簧等）。
pub trait JointSolver {
    /// 以固定步长求解一次约束。
    fn solve(&mut self, dt: f32);
}

/// 高级连续碰撞检测预留接口（动态-动态 CCD / 子步进）。
pub trait ContinuousCollisionDetector {
    /// 以固定步长做一次 CCD 扫掠；返回是否发生了子步进。
    fn sweep_ccd(&mut self, dt: f32) -> bool;
}

/// 布娃娃（ragdoll）预留接口。
pub trait Ragdoll {
    /// 从骨骼链构建布娃娃刚体。
    fn build(&mut self);
}
