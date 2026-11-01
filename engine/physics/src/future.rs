//! 0.3 预留接口（本版本只保留形状，不实现）。
//!
//! 0.2 明确不做：关节/弹簧/约束、连续碰撞检测（CCD）、布娃娃（ragdoll）、
//! 完整动态刚体堆叠。这里先定义接口轮廓，方便 0.3 在同一套 `PhysicsWorld`
//! 上增量实现，而不影响 0.2 已稳定的胶囊角色/命中盒 API。
//!
//! TODO(0.3)：以下 trait 仅作接口预留，当前没有任何实现。

/// 关节/约束求解器预留接口。
///
/// TODO(0.3)：实现固定关节、球关节、弹簧约束，并在 `PhysicsWorld::step`
/// 的速度求解阶段统一迭代。
pub trait JointSolver {
    /// 以固定步长求解一次约束。
    fn solve(&mut self, dt: f32);
}

/// 连续碰撞检测（CCD）预留接口。
///
/// TODO(0.3)：对高速动态刚体做扫掠检测，避免穿模。
pub trait ContinuousCollisionDetector {
    /// 以固定步长做一次 CCD 扫掠；返回是否发生了子步进。
    fn sweep_ccd(&mut self, dt: f32) -> bool;
}

/// 布娃娃（ragdoll）预留接口。
///
/// TODO(0.3)：由骨骼关节链生成的刚体集合，在死亡/受击时切换驱动方式。
pub trait Ragdoll {
    /// 从骨骼链构建布娃娃刚体。
    fn build(&mut self);
}
