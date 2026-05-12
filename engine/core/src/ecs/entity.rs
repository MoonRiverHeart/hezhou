pub type EntityId = u64;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Entity {
    pub id: EntityId,
    pub generation: u32,
}

impl Entity {
    pub fn new(id: EntityId) -> Self {
        Self {
            id,
            generation: 0,
        }
    }
    
    pub fn is_valid(&self) -> bool {
        self.id != 0
    }
    
    pub fn equals(&self, other: &Self) -> bool {
        self.id == other.id && self.generation == other.generation
    }
}

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}