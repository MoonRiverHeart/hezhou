//! CharacterMotor：移动 / 奔跑 / 跳跃 / 重力 / 地面吸附 / 外部位移。

mod support;

use hezhou_gameplay::{
    CharacterCollisionQuery, CharacterMotor, MotorConfig, MotorInput, MotorOutput, Vec3,
};
use support::MockCharacterQuery;

const DT: f32 = 1.0 / 60.0;

fn new_motor() -> CharacterMotor {
    CharacterMotor::with_position(MotorConfig::default(), Vec3::ZERO)
}

fn settle_on_ground(
    motor: &mut CharacterMotor,
    query: &dyn CharacterCollisionQuery,
) -> MotorOutput {
    motor.step(DT, &MotorInput::default(), query)
}

fn jump_input() -> MotorInput {
    MotorInput {
        jump_pressed: true,
        jump_held: true,
        ..MotorInput::default()
    }
}

#[test]
fn walking_moves_character_forward() {
    let query = MockCharacterQuery::flat();
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);

    let input = MotorInput::with_direction(Vec3::FORWARD);
    let mut output = MotorOutput::default();
    for _ in 0..60 {
        output = motor.step(DT, &input, &query);
    }

    assert!(output.grounded, "角色应保持在地面");
    assert!(
        (output.position.z + 3.8).abs() < 0.3,
        "1 秒步行位移应约 3.8m，实际 z={}",
        output.position.z
    );
    assert!(output.speed > 3.5, "步行速度应接近配置值");
}

#[test]
fn sprint_moves_faster_than_walk() {
    let query = MockCharacterQuery::flat();
    let mut walk_motor = new_motor();
    let mut run_motor = new_motor();
    settle_on_ground(&mut walk_motor, &query);
    settle_on_ground(&mut run_motor, &query);

    let walk = MotorInput::with_direction(Vec3::FORWARD);
    let run = MotorInput {
        sprint: true,
        ..MotorInput::with_direction(Vec3::FORWARD)
    };
    for _ in 0..60 {
        walk_motor.step(DT, &walk, &query);
        run_motor.step(DT, &run, &query);
    }

    let walk_distance = walk_motor.position().z.abs();
    let run_distance = run_motor.position().z.abs();
    assert!(
        run_distance > walk_distance * 1.4,
        "奔跑距离 {run_distance} 应明显大于步行 {walk_distance}"
    );
}

#[test]
fn jump_leaves_ground_and_lands() {
    let query = MockCharacterQuery::flat();
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);

    let jump = motor.step(DT, &jump_input(), &query);
    assert!(jump.just_jumped, "应产生起跳事件");
    assert!(!jump.grounded, "起跳后应离地");
    assert!(jump.velocity.y > 0.0, "起跳后垂直速度应为正");

    let hold = MotorInput {
        jump_held: true,
        ..MotorInput::default()
    };
    let mut apex = jump.position.y;
    let mut landed = false;
    let mut elapsed = 0.0;
    for _ in 0..180 {
        let output = motor.step(DT, &hold, &query);
        apex = apex.max(output.position.y);
        elapsed += DT;
        if output.just_landed {
            landed = true;
            break;
        }
    }

    assert!(apex > 1.0, "跳跃最高点应超过 1m，实际 {apex}");
    assert!(landed, "应在有限时间内落地");
    assert!(elapsed < 1.0, "落地耗时应小于 1 秒，实际 {elapsed}");
    assert!(motor.position().y.abs() < 0.01, "落地后应回到地面");
}

#[test]
fn releasing_jump_cuts_height() {
    fn apex_after_jump(release_next_frame: bool) -> f32 {
        let query = MockCharacterQuery::flat();
        let mut motor = new_motor();
        settle_on_ground(&mut motor, &query);
        let first = motor.step(DT, &jump_input(), &query);
        let mut apex = first.position.y;
        let input = MotorInput {
            jump_held: !release_next_frame,
            ..MotorInput::default()
        };
        for _ in 0..120 {
            let output = motor.step(DT, &input, &query);
            apex = apex.max(output.position.y);
            if output.grounded {
                break;
            }
        }
        apex
    }

    let full = apex_after_jump(false);
    let cut = apex_after_jump(true);
    assert!(full > 1.0, "按住跳跃应到达正常高度");
    assert!(
        cut < full * 0.6,
        "松开跳跃应明显降低高度：full={full}, cut={cut}"
    );
}

#[test]
fn coyote_time_allows_jump_after_leaving_ground() {
    let mut query = MockCharacterQuery::flat();
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);
    assert!(motor.is_grounded());

    // 地面突然消失：角色进入空中，但土狼时间仍允许起跳。
    query.has_ground = false;
    let airborne = motor.step(DT, &MotorInput::default(), &query);
    assert!(!airborne.grounded, "离开地面后应处于空中");

    let jumped = motor.step(DT, &jump_input(), &query);
    assert!(jumped.just_jumped, "土狼时间内应允许起跳");
}

#[test]
fn jump_buffer_triggers_on_landing() {
    let query = MockCharacterQuery::flat();
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);
    motor.step(DT, &jump_input(), &query);

    let hold = MotorInput {
        jump_held: true,
        ..MotorInput::default()
    };
    let mut pressed_early = false;
    let mut rebounded = false;
    for _ in 0..180 {
        let mut input = hold;
        // 在即将落地时提前按跳跃，之后保持"不按"状态。
        if !pressed_early && motor.velocity().y < 0.0 && motor.position().y < 0.25 {
            input.jump_pressed = true;
            pressed_early = true;
        }
        let output = motor.step(DT, &input, &query);
        if pressed_early && output.just_jumped {
            rebounded = true;
            break;
        }
    }

    assert!(pressed_early, "应在落地前提前按下跳跃");
    assert!(rebounded, "落地后应因输入缓冲立即再次起跳");
}

#[test]
fn ground_snap_follows_downhill() {
    let flat = MockCharacterQuery::flat();
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &flat);
    assert!(motor.is_grounded());

    // move_capsule 不做地面解算，只靠 ground_probe 吸附，验证吸附逻辑。
    let slope = MockCharacterQuery {
        ground_slope: -0.5,
        clamp_in_move: false,
        ..MockCharacterQuery::default()
    };
    let input = MotorInput::with_direction(Vec3::FORWARD);
    let mut output = motor.step(DT, &input, &slope);
    for _ in 0..60 {
        output = motor.step(DT, &input, &slope);
        assert!(output.grounded, "下坡时应保持地面吸附");
    }

    let expected_height = 0.5 * output.position.z;
    assert!(
        (output.position.y - expected_height).abs() < 0.05,
        "应贴合下坡地面：y={} 期望={expected_height}",
        output.position.y
    );
    assert!(output.position.z < -2.0, "应沿坡向前移动");
}

#[test]
fn wall_contact_removes_into_wall_velocity() {
    let mut query = MockCharacterQuery::flat();
    query.wall_z = Some(-1.0);
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);

    let input = MotorInput::with_direction(Vec3::FORWARD);
    let mut output = MotorOutput::default();
    for _ in 0..60 {
        output = motor.step(DT, &input, &query);
    }

    assert!(
        output.position.z >= -1.001,
        "不应穿过墙体，实际 z={}",
        output.position.z
    );
    assert!(output.speed < 0.2, "撞墙后水平速度应被消去");
}

#[test]
fn ceiling_stops_upward_velocity() {
    let mut query = MockCharacterQuery::flat();
    query.ceiling_y = Some(3.0);
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);

    motor.step(DT, &jump_input(), &query);
    let hold = MotorInput {
        jump_held: true,
        ..MotorInput::default()
    };
    let mut max_height: f32 = 0.0;
    for _ in 0..120 {
        let output = motor.step(DT, &hold, &query);
        max_height = max_height.max(output.position.y);
        assert!(
            output.position.y + motor.config().height <= 3.001,
            "头部不应穿过天花板"
        );
    }
    // 天花板高度 3.0 - 角色高度 1.8 = 1.2。
    assert!(
        (max_height - 1.2).abs() < 0.05,
        "应在天花板处停止上升，实际最高点 {max_height}"
    );
}

#[test]
fn movement_is_frame_rate_independent() {
    fn distance_after_one_second(hz: f32) -> f32 {
        let query = MockCharacterQuery::flat();
        let mut motor = new_motor();
        let dt = 1.0 / hz;
        motor.step(dt, &MotorInput::default(), &query);
        let input = MotorInput::with_direction(Vec3::FORWARD);
        let steps = hz as usize;
        for _ in 0..steps {
            motor.step(dt, &input, &query);
        }
        motor.position().z.abs()
    }

    let at_60 = distance_after_one_second(60.0);
    let at_120 = distance_after_one_second(120.0);
    assert!(
        (at_60 - at_120).abs() < 0.05,
        "不同帧率下单秒位移应一致：60Hz={at_60}, 120Hz={at_120}"
    );
}

#[test]
fn external_velocity_adds_displacement_without_changing_stored_velocity() {
    let query = MockCharacterQuery::flat();
    let mut motor = new_motor();
    settle_on_ground(&mut motor, &query);
    assert!(motor.velocity().z.abs() < 0.01);

    // 模拟闪避：外部速度驱动位移，但不写入 motor 的持久速度。
    let dodge = MotorInput {
        external_velocity: Vec3::new(0.0, 0.0, -9.0),
        ..MotorInput::default()
    };
    for _ in 0..10 {
        motor.step(DT, &dodge, &query);
    }

    assert!(
        motor.position().z < -1.2,
        "外部速度应产生位移，实际 z={}",
        motor.position().z
    );
    assert!(motor.velocity().z.abs() < 0.01, "外部速度不应污染持久速度");
}
