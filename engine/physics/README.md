# hezhou-physics

自研动作物理模块（**不引入 rapier / PhysX**，ADR-002）。纯 Rust，无平台依赖，被
`engine/gameplay`（可选 `physics` feature）与 `engine/integration` 复用。

## 能力边界

### 0.2（保持兼容）
- 胶囊角色控制器 `CharacterMotor`：胶囊 vs 静态 Box/Sphere/Plane 的扫掠、
  穿透解决、`move_and_slide`、地面吸附、基础台阶/斜坡处理；
- 固定时间步 `FixedTimestep`（与渲染帧率解耦 + 渲染插值 alpha）；
- 碰撞层/掩码、触发器（sensor）Enter/Exit 事件、命中盒（Sphere/Box/Capsule）；
- 查询：`raycast` / `raycast_filtered` / `raycast_all` / `sweep_capsule` /
  `sphere_cast` / `overlap_sphere|capsule|box|shape` / `resolve_capsule_penetration`；
- 命中结果包含 hit point、normal、distance、body/collider id、is_sensor；
- 调试描述 `DebugShape`（碰撞体/命中盒的线框数据）；
- 原有 AABB 刚体、射线、`CharacterController` API 原样保留。

### 0.3 第一批（动态刚体）
- 动态积分：重力（`gravity_scale`）、线速度、质量/逆质量、线性/角阻尼、
  最大线速度限制；角速度做一阶旋转积分；
- 接触响应：动态 vs 静态/动态的 AABB/球/胶囊包围盒接触，
  法向冲量 + 恢复系数 + 库仑摩擦 + 位置修正（迭代求解，支持简单堆叠）；
- 无限平面 `ColliderShape::Plane` 参与动态刚体碰撞（自由落体落地）；
- CCD 基础：动态球/胶囊对静态/运动学目标的扫掠检测（可配置，默认关闭）；
- 自动睡眠（阈值/延迟可配，默认关闭）；
- 事件：碰撞对 Enter 事件、触发器 Enter/Exit 事件（与 0.2 语义一致，新增平面接触事件）。

### 0.3 第二批（关节增强 + 动态 CCD）
- **固定关节 `FixedJoint`**：锁定两刚体锚点的相对平移（“焊死/父子跟随”），
  速度层（消除三轴相对速度）+ 位置层（按逆质量修正误差）迭代，`stiffness` 可调；
  完整增删查 API（`add_fixed_joint` / `push_fixed_joint` / `fixed_joints` /
  `fixed_joint_mut` / `remove_fixed_joint` / `fixed_joint_count`）；
- **动态-动态 CCD**：对动态目标用“相对位移（自身位移 − 目标本步位移）”做 swept
  检测，命中后按逆质量分配冲量并停在接触点前，避免两个高速球/胶囊互穿；
  建议双方都启用 CCD 以获得双向防护；
- 距离关节与固定关节共享 `clear_joints` / `joint_count`。

## 关键 API

```rust
// 世界配置
world.set_gravity(Vec3::new(0.0, -9.81, 0.0));
world.set_solver_config(SolverConfig { velocity_iterations: 6, ..Default::default() });
world.set_ccd_enabled(true);          // CCD 全局开关
world.set_auto_sleep(true);           // 自动睡眠（可选）

// 动态刚体
let body = world.create_body();
body.set_mass(2.0);
body.set_linear_damping(0.1);
body.set_ccd_enabled(true);           // 单刚体 CCD 开关
let collider = world.create_collider(ColliderShape::sphere(0.5), body);
collider.set_restitution(0.4);
collider.set_friction(0.5);

// 关节
let distance = world.add_distance_joint(body_a, body_b, 2.0);
let fixed = world.add_fixed_joint(body_a, body_b);        // 以当前相对位置为约束
let custom = FixedJoint::new(body_a, body_b, Vec3::new(0.0, 1.0, 0.0))
    .with_anchors(Vec3::zero(), Vec3::zero())
    .with_stiffness(0.8);
world.push_fixed_joint(custom);

// 步进
world.step(1.0 / 60.0);
world.advance_fixed(&mut FixedTimestep::new(60.0), frame_dt);
```

`PhysicsWorld::step` 的求解流水线：

1. 速度积分（重力/阻尼/限速）；
2. 位置积分（CCD 开启时先扫掠并停在接触点前，静态与动态目标都参与）；
3. 接触检测 + 碰撞/触发器事件更新；
4. 速度冲量迭代（法向 + 摩擦，恢复系数只在首次迭代生效）；
5. 位置修正迭代（重新检测接触，按逆质量分配）；
6. 关节求解（距离关节 + 固定关节，各迭代速度层 + 位置层）；
7. 自动睡眠更新（可选）。

## 已知限制
- **碰撞形状仍是轴对齐近似**：`Box` 用 AABB 参与检测，忽略刚体旋转；
  `Capsule`/`Cylinder` 轴固定世界 Y 轴；`Cylinder` 的精确求交未实现（查询会跳过）。
- **线性动力学**：接触响应不产生角冲量，没有惯性张量计算；旋转仅一阶积分。
  旋转盒/旋转胶囊碰撞视觉上仍按 AABB 处理。
- **CCD**：只支持动态球/胶囊作为运动形状；动态-动态为“相对位移 + 顺序求解”近似，
  建议双方都启用；不支持旋转盒 CCD 与子步进；命中不额外产生碰撞事件
  （由下一帧离散接触补上）。
- **关节**：只有距离约束与固定平移约束；锚点偏移不随刚体旋转，固定关节不约束
  相对旋转；无铰链/弹簧/角度限制/完整约束矩阵求解。
- **宽相为 O(n²)**：逐个 AABB 两两检测（带 AABB 粗筛），空间索引（uniform grid/BVH）
  留后续；当前规模下测试与示例场景无压力。
- **布娃娃、旋转碰撞响应、任意朝向碰撞体、空间索引宽相** 仍为后续版本
  （见 `future` 模块）。

## 测试

```bash
cd engine
cargo test -p hezhou-physics --offline
cargo test -p hezhou-gameplay --features physics --offline
cargo test -p hezhou-integration --offline
cargo test -p hezhou-core --offline
```

覆盖：固定步/扫掠/斜坡台阶/层掩码/触发器/raycast/overlap/调试数据（0.2），
自由落体落地、两球弹性/非弹性碰撞动量、动态堆叠、CCD 防穿透（静态与动态-动态）、
距离关节/固定关节、阻尼、自动睡眠（0.3）。
