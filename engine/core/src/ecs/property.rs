use crate::math::Vec3;
use crate::ecs::{Entity, Scene};

/// Property value types for the reflection system.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum PropertyType {
    Float = 0,
    Float3 = 1,
    String = 2,
    Bool = 3,
    Int = 4,
    Enum = 5,
}

/// A property value that can be transferred across FFI.
#[repr(C)]
#[derive(Clone, Debug)]
pub enum PropertyValue {
    Float(f32),
    Float3 { x: f32, y: f32, z: f32 },
    String(String),
    Bool(bool),
    Int(i32),
    Enum { value: i32, names: Vec<String> },
}

/// Describes a property on an entity that can be inspected/modified.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct PropertyDescriptor {
    pub name: String,
    pub display_name: String,
    pub property_type: PropertyType,
    pub category: String,
    pub read_only: bool,
}

/// Get all property descriptors for an entity.
/// Returns the list of properties that can be inspected on this entity.
pub fn get_entity_property_descriptors() -> Vec<PropertyDescriptor> {
    vec![
        PropertyDescriptor {
            name: "name".to_string(),
            display_name: "Name".to_string(),
            property_type: PropertyType::String,
            category: "Identity".to_string(),
            read_only: false,
        },
        PropertyDescriptor {
            name: "position".to_string(),
            display_name: "Position".to_string(),
            property_type: PropertyType::Float3,
            category: "Transform".to_string(),
            read_only: false,
        },
        PropertyDescriptor {
            name: "rotation".to_string(),
            display_name: "Rotation".to_string(),
            property_type: PropertyType::Float3,
            category: "Transform".to_string(),
            read_only: false,
        },
        PropertyDescriptor {
            name: "scale".to_string(),
            display_name: "Scale".to_string(),
            property_type: PropertyType::Float3,
            category: "Transform".to_string(),
            read_only: false,
        },
        PropertyDescriptor {
            name: "id".to_string(),
            display_name: "Entity ID".to_string(),
            property_type: PropertyType::Int,
            category: "Identity".to_string(),
            read_only: true,
        },
    ]
}

/// Get a property value from an entity.
pub fn get_entity_property_value(scene: &Scene, entity: Entity, property_name: &str) -> Option<PropertyValue> {
    match property_name {
        "name" => {
            scene.get_entity_name(entity).map(|n| PropertyValue::String(n.clone()))
        }
        "position" => {
            scene.get_entity_position(entity).map(|p| PropertyValue::Float3 { x: p.x, y: p.y, z: p.z })
        }
        "rotation" => {
            scene.get_entity_rotation(entity).map(|q| {
                // Convert quaternion to Euler angles (degrees)
                let euler = q.to_euler_degrees();
                PropertyValue::Float3 {
                    x: euler[0],
                    y: euler[1],
                    z: euler[2],
                }
            })
        }
        "scale" => {
            scene.get_entity_scale(entity).map(|s| PropertyValue::Float3 { x: s.x, y: s.y, z: s.z })
        }
        "id" => {
            Some(PropertyValue::Int(entity.id as i32))
        }
        _ => None,
    }
}

/// Set a property value on an entity.
pub fn set_entity_property_value(scene: &mut Scene, entity: Entity, property_name: &str, value: &PropertyValue) -> bool {
    if !scene.world.entity_exists(entity) {
        return false;
    }

    match property_name {
        "name" => {
            if let PropertyValue::String(s) = value {
                scene.set_entity_name(entity, s.clone());
                true
            } else {
                false
            }
        }
        "position" => {
            if let PropertyValue::Float3 { x, y, z } = value {
                scene.set_entity_position(entity, Vec3::new(*x, *y, *z));
                true
            } else {
                false
            }
        }
        "rotation" => {
            if let PropertyValue::Float3 { x, y, z } = value {
                // Convert Euler degrees back to quaternion
                let q = crate::math::Quaternion::from_euler_degrees(*x, *y, *z);
                scene.set_entity_rotation(entity, q);
                true
            } else {
                false
            }
        }
        "scale" => {
            if let PropertyValue::Float3 { x, y, z } = value {
                scene.set_entity_scale(entity, Vec3::new(*x, *y, *z));
                true
            } else {
                false
            }
        }
        // id and other read-only properties cannot be set
        _ => false,
    }
}