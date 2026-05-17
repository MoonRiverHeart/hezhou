#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferType {
    Vertex,
    Index,
    Uniform,
    Storage,
    TransferSrc,
    TransferDst,
}

#[derive(Clone, Debug)]
pub struct BufferDesc {
    pub size: usize,
    pub buffer_type: BufferType,
    pub usage: BufferUsage,
    pub memory_location: MemoryLocation,
}

impl BufferDesc {
    pub fn vertex(size: usize) -> Self {
        Self {
            size,
            buffer_type: BufferType::Vertex,
            usage: BufferUsage::default(),
            memory_location: MemoryLocation::Device,
        }
    }

    pub fn index(size: usize) -> Self {
        Self {
            size,
            buffer_type: BufferType::Index,
            usage: BufferUsage::default(),
            memory_location: MemoryLocation::Device,
        }
    }

    pub fn uniform(size: usize) -> Self {
        Self {
            size,
            buffer_type: BufferType::Uniform,
            usage: BufferUsage::default(),
            memory_location: MemoryLocation::HostVisible,
        }
    }

    pub fn storage(size: usize) -> Self {
        Self {
            size,
            buffer_type: BufferType::Storage,
            usage: BufferUsage::default(),
            memory_location: MemoryLocation::Device,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BufferUsage {
    pub map_read: bool,
    pub map_write: bool,
    pub copy_src: bool,
    pub copy_dst: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryLocation {
    Device,
    HostVisible,
    HostCoherent,
}
