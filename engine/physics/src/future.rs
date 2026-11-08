//! 后续版本预留接口。
//!
//! 0.3 已落地：动态刚体、接触冲量/摩擦/位置修正、基础 CCD（静态 + 动态-动态）、
//! 距离关节与固定关节、uniform grid 空间索引宽相（可切回 brute force）、
//! 旋转盒保守 AABB 接触。以下能力仍在后续版本规划中：
//!
//! TODO(0.4)：铰链、弹簧、角度限制、相对旋转约束等完整约束求解；
//! TODO(0.4)：布娃娃（ragdoll）与骨骼关节链；
//! TODO(0.4)：子步进（sub-stepping）、旋转盒 CCD、连续多步 CCD；
//! TODO(0.4)：旋转碰撞响应（角冲量/惯性张量）与精确 OBB/分离轴接触；
//! TODO(0.4)：多级/动态 BVH 或并行宽相（当前为 uniform grid + brute force 可切换）。

/// 完整关节/约束求解器预留接口（铰链、弹簧、角度限制等）。
pub trait JointSolver {
    /// 以固定步长求解一次约束。
    fn solve(&mut self, dt: f32);
}

/// 高级连续碰撞检测预留接口（子步进 / 旋转盒 CCD）。
pub trait ContinuousCollisionDetector {
    /// 以固定步长做一次 CCD 扫掠；返回是否发生了子步进。
    fn sweep_ccd(&mut self, dt: f32) -> bool;
}

/// 布娃娃（ragdoll）预留接口。
pub trait Ragdoll {
    /// 从骨骼链构建布娃娃刚体。
    fn build(&mut self);
}
