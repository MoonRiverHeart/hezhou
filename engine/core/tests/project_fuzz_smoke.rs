use hezhou_core::project::{ProjectEntity, ProjectEntityTransform, ProjectFile, ProjectLight};

/// 轻量可复现伪随机数：不引入额外依赖，CI 可稳定运行。
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }
}

#[test]
fn project_json_parser_never_panics_on_malformed_input() {
    let mut rng = Lcg(0xC0FFEE);
    let atoms = [
        "{",
        "}",
        "[",
        "]",
        "\"name\"",
        "\"entities\"",
        "\"version\"",
        "\"1.0\"",
        ":",
        ",",
        "null",
        "0",
        "-1",
        "1e999",
        "\"DirectionalLight\"",
        "\"None\"",
        "\"builtin://cube\"",
        "true",
        "false",
        "\"\\u0000\"",
        "💥",
        "\"created_at\":\"bad-date\"",
    ];

    for _ in 0..3000 {
        let mut input = String::new();
        let parts = rng.below(28) + 1;
        for _ in 0..parts {
            input.push_str(atoms[rng.below(atoms.len())]);
        }

        // 目标：任意畸形输入只能返回 Err，不能 panic / 越界。
        let _ = serde_json::from_str::<ProjectFile>(&input);
    }
}

#[test]
fn project_entity_serde_roundtrip_handles_unicode_and_optional_fields() {
    let mut rng = Lcg(0x12345678);
    for i in 0..200 {
        let mut entity = ProjectEntity::default();
        entity.id = i as u64 + 1;
        entity.name = format!("实体_{}_💥", i);
        entity.mesh_type = match i % 3 {
            0 => "Cube".to_string(),
            1 => "DirectionalLight".to_string(),
            _ => "None".to_string(),
        };
        entity.visible = i % 2 == 0;
        entity.transform = ProjectEntityTransform {
            position: [rng.next() as f32 / 1000.0, -1.0, 2.0],
            rotation: [0.0, (i as f32) * 15.0, 0.0],
            scale: [1.0, 1.5, 0.5],
        };
        if i % 4 == 0 {
            entity.light = Some(ProjectLight {
                direction: [0.0, -1.0, 0.0],
                color: [1.0, 0.9, 0.8],
                intensity: 2.0,
            });
        }

        let json = serde_json::to_string(&entity).expect("serialize");
        let parsed: ProjectEntity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.id, entity.id);
        assert_eq!(parsed.name, entity.name);
        assert_eq!(parsed.mesh_type, entity.mesh_type);
        assert_eq!(parsed.visible, entity.visible);
        assert_eq!(parsed.light.is_some(), entity.light.is_some());
    }
}
