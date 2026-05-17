#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShaderHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderPassHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FramebufferHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommandPoolHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CommandBufferHandle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SwapChainHandle(pub u64);

impl BufferHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl TextureHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl ShaderHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl PipelineHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl RenderPassHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl FramebufferHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl CommandPoolHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl CommandBufferHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl SwapChainHandle {
    pub fn null() -> Self {
        Self(0)
    }
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}
