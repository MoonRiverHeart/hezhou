use hezhou_core::ffi::*;
use hezhou_core::*;
use hezhou_scripting::*;
use std::ffi::CString;

fn main() {
    println!("=== Hezhou Game Engine Demo ===\n");

    println!("[1] 创建引擎实例...");
    let engine = engine_create();
    println!("    Engine ptr: {:?}\n", engine);

    println!("[2] 启动引擎...");
    engine_start(engine);
    let running = engine_is_running(engine);
    println!("    Engine running: {}\n", running);

    println!("[3] 获取 World 和 EventBus...");
    let world = engine_get_world(engine);
    let event_bus = engine_get_event_bus(engine);
    println!("    World ptr: {:?}", world);
    println!("    EventBus ptr: {:?}\n", event_bus);

    println!("[4] 创建实体...");
    let entity1 = ecs_create_entity(world);
    let entity2 = ecs_create_entity(world);
    let entity3 = ecs_create_entity(world);
    println!("    Entity 1 ID: {}", entity1);
    println!("    Entity 2 ID: {}", entity2);
    println!("    Entity 3 ID: {}", entity3);
    println!("    Entity count: {}\n", ecs_entity_count(world));

    println!("[5] 添加 TransformComponent...");
    ecs_add_transform_component(
        world, entity1, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
    );
    println!(
        "    Entity 1 has transform: {}\n",
        ecs_has_transform_component(world, entity1)
    );

    println!("[6] 添加 NameComponent...");
    let name = CString::new("Player").unwrap();
    ecs_add_name_component(world, entity1, name.as_ptr());
    println!("    Entity 1 name: 'Player'\n");

    println!("[7] 设置实体层级...");
    ecs_set_entity_parent(world, entity2, entity1);
    ecs_set_entity_parent(world, entity3, entity1);
    println!(
        "    Entity 2 parent: {}",
        ecs_get_entity_parent(world, entity2)
    );
    println!("    Entity 1 children count: 2\n");

    println!("[8] 运行帧循环...");
    for i in 0..5 {
        engine_run_frame(engine, 0.016);
        let time = engine_get_time(engine);
        let delta = engine_get_delta_time(engine);
        let frame = engine_get_frame_count(engine);
        println!(
            "    Frame {}: time={:.3}s, delta={:.3}s",
            frame, time, delta
        );
    }
    println!();

    println!("[9] 发布事件...");
    event_bus_publish(engine, 2, entity1 as usize);
    event_bus_dispatch(engine);
    println!("    EntityCreated event dispatched\n");

    println!("[10] 获取 Transform 数据...");
    let mut pos = [0.0f32; 3];
    let mut rot = [0.0f32; 4];
    let mut scale = [0.0f32; 3];
    let has_transform = ecs_get_transform_component(
        world,
        entity1,
        pos.as_mut_ptr(),
        rot.as_mut_ptr(),
        scale.as_mut_ptr(),
    );
    if has_transform {
        println!("    Position: ({}, {}, {})", pos[0], pos[1], pos[2]);
        println!(
            "    Rotation: ({}, {}, {}, {})",
            rot[0], rot[1], rot[2], rot[3]
        );
        println!("    Scale: ({}, {}, {})\n", scale[0], scale[1], scale[2]);
    }

    println!("[11] 销毁实体...");
    ecs_destroy_entity(world, entity2);
    println!(
        "    Entity count after destroy: {}\n",
        ecs_entity_count(world)
    );

    println!("[12] 测试脚本系统...");
    let script_mgr = scripting_init();
    println!("    ScriptManager ptr: {:?}", script_mgr);

    extern "C" fn test_callback(arg: ScriptValue, context: usize) -> ScriptValue {
        let multiplier = context as i32;
        if let Some(val) = arg.get_int() {
            ScriptValue::from_int(val * multiplier)
        } else {
            ScriptValue::err("Expected int")
        }
    }

    let cb_name = CString::new("multiply").unwrap();
    let cb_desc = CString::new("Multiply by context").unwrap();
    let cb_sig = CString::new("int -> int").unwrap();

    scripting_register_sync_callback(
        script_mgr,
        cb_name.as_ptr(),
        test_callback,
        cb_desc.as_ptr(),
        cb_sig.as_ptr(),
        5,
    );

    let trigger_name = CString::new("multiply").unwrap();
    let arg = ScriptValue::from_int(10);
    let result = scripting_trigger_sync(script_mgr, trigger_name.as_ptr(), arg);
    println!(
        "    Script callback: 10 * 5 = {}\n",
        result.get_int().unwrap_or(0)
    );

    scripting_shutdown(script_mgr);

    println!("[13] 停止引擎...");
    engine_stop(engine);
    println!("    Engine running: {}\n", engine_is_running(engine));

    println!("[14] 销毁引擎...");
    engine_destroy(engine);
    println!("    Engine destroyed\n");

    println!("=== Demo Complete ===");
}
