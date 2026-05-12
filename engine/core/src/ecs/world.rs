use crate::ecs::*;
use std::collections::HashMap;

pub struct World {
    next_entity_id: EntityId,
    entities: HashMap<EntityId, Entity>,
    components: HashMap<ComponentTypeId, HashMap<EntityId, Vec<u8>>>,
    scheduler: SystemScheduler,
    parent_map: HashMap<EntityId, Option<EntityId>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_entity_id: 1,
            entities: HashMap::new(),
            components: HashMap::new(),
            scheduler: SystemScheduler::new(),
            parent_map: HashMap::new(),
        }
    }
    
    pub fn create_entity(&mut self) -> Entity {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        
        let entity = Entity::new(id);
        self.entities.insert(id, entity);
        self.parent_map.insert(id, None);
        
        entity
    }
    
    pub fn destroy_entity(&mut self, entity: Entity) {
        self.entities.remove(&entity.id);
        self.parent_map.remove(&entity.id);
        
        for (_, component_map) in &mut self.components {
            component_map.remove(&entity.id);
        }
    }
    
    pub fn entity_exists(&self, entity: Entity) -> bool {
        self.entities.contains_key(&entity.id)
    }
    
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = T::type_id();
        
        if !self.components.contains_key(&type_id) {
            self.components.insert(type_id, HashMap::new());
        }
        
        let component_data = unsafe {
            let ptr = &component as *const T as *const u8;
            std::slice::from_raw_parts(ptr, std::mem::size_of::<T>())
        };
        
        self.components.get_mut(&type_id)
            .unwrap()
            .insert(entity.id, component_data.to_vec());
    }
    
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<T> {
        let type_id = T::type_id();
        
        self.components.get(&type_id)
            .and_then(|map| map.get(&entity.id))
            .map(|data| {
                unsafe {
                    let ptr = data.as_ptr() as *const T;
                    std::ptr::read(ptr)
                }
            })
    }
    
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        let type_id = T::type_id();
        self.components.get(&type_id)
            .map(|map| map.contains_key(&entity.id))
            .unwrap_or(false)
    }
    
    pub fn remove_component<T: Component>(&mut self, entity: Entity) {
        let type_id = T::type_id();
        
        if let Some(component_map) = self.components.get_mut(&type_id) {
            component_map.remove(&entity.id);
        }
    }
    
    pub fn set_parent(&mut self, entity: Entity, parent: Option<Entity>) {
        if let Some(p) = parent {
            self.parent_map.insert(entity.id, Some(p.id));
        } else {
            self.parent_map.insert(entity.id, None);
        }
    }
    
    pub fn get_parent(&self, entity: Entity) -> Option<Entity> {
        self.parent_map.get(&entity.id)
            .and_then(|parent_id| parent_id.map(|id| Entity::new(id)))
    }
    
    pub fn get_children(&self, parent: Entity) -> Vec<Entity> {
        self.parent_map.iter()
            .filter_map(|(child_id, parent_id)| {
                if *parent_id == Some(parent.id) {
                    Some(Entity::new(*child_id))
                } else {
                    None
                }
            })
            .collect()
    }
    
    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.scheduler.add_system(system);
    }
    
    pub fn update(&mut self, _delta_time: f32) {
        // Note: Systems cannot access World directly in this simple implementation
        // A more sophisticated ECS would use a command buffer pattern
    }
    
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}