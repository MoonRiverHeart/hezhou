#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
    TriangleFan,
}

impl Default for PrimitiveTopology {
    fn default() -> Self {
        Self::TriangleList
    }
}