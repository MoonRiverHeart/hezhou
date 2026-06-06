/// 渲染结果
#[derive(Debug)]
pub struct RenderResult {
    pub frame_completed: bool,
}

/// 顶点数据
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

/// 矩形
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// 绘制命令
#[derive(Debug, Clone)]
pub struct DrawCommand {
    /// 顶点数据
    pub vertices: Vec<Vertex>,
    /// 索引数据（可选，为None时用顺序顶点）
    pub indices: Option<Vec<u32>>,
    /// 裁剪矩形（屏幕坐标）
    pub clip_rect: Option<Rect>,
    /// 纹理ID（0表示纯色，用顶点颜色）
    pub texture_id: u32,
}

impl DrawCommand {
    /// 创建纯色矩形绘制命令
    pub fn rect(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        DrawCommand {
            vertices: vec![
                Vertex { position: [x, y], color: [r, g, b, a], uv: [0.0, 0.0] },
                Vertex { position: [x + w, y], color: [r, g, b, a], uv: [1.0, 0.0] },
                Vertex { position: [x + w, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
                Vertex { position: [x, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
            ],
            indices: Some(vec![0, 1, 2, 2, 3, 0]),
            clip_rect: None,
            texture_id: 0,
        }
    }

    /// 创建矩形线框绘制命令（仅debug版本可用）
    #[cfg(debug_assertions)]
    pub fn rect_line(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        let t = 1.0;
        let vertices = vec![
            Vertex { position: [x, y], color: [r, g, b, a], uv: [0.0, 0.0] },
            Vertex { position: [x + w, y], color: [r, g, b, a], uv: [1.0, 0.0] },
            Vertex { position: [x + w, y + t], color: [r, g, b, a], uv: [1.0, 1.0] },
            Vertex { position: [x, y + t], color: [r, g, b, a], uv: [0.0, 1.0] },
            Vertex { position: [x, y + h - t], color: [r, g, b, a], uv: [0.0, 0.0] },
            Vertex { position: [x + w, y + h - t], color: [r, g, b, a], uv: [1.0, 0.0] },
            Vertex { position: [x + w, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
            Vertex { position: [x, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
            Vertex { position: [x, y], color: [r, g, b, a], uv: [0.0, 0.0] },
            Vertex { position: [x + t, y], color: [r, g, b, a], uv: [1.0, 0.0] },
            Vertex { position: [x + t, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
            Vertex { position: [x, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
            Vertex { position: [x + w - t, y], color: [r, g, b, a], uv: [0.0, 0.0] },
            Vertex { position: [x + w, y], color: [r, g, b, a], uv: [1.0, 0.0] },
            Vertex { position: [x + w, y + h], color: [r, g, b, a], uv: [1.0, 1.0] },
            Vertex { position: [x + w - t, y + h], color: [r, g, b, a], uv: [0.0, 1.0] },
        ];
        let indices = vec![
            0,1,2, 2,3,0,
            4,5,6, 6,7,4,
            8,9,10, 10,11,8,
            12,13,14, 14,15,12,
        ];
        DrawCommand {
            vertices,
            indices: Some(indices),
            clip_rect: None,
            texture_id: 0,
        }
    }
}

/// RHI 初始化描述
#[derive(Debug)]
pub struct RhiInitDesc {
    pub window_handle: WindowHandle,
    pub width: u32,
    pub height: u32,
}

/// 窗口句柄（平台相关）
#[derive(Debug)]
pub struct WindowHandle {
    #[cfg(target_os = "windows")]
    pub hwnd: *mut std::ffi::c_void,
    #[cfg(target_os = "linux")]
    pub x11_window: u64,
    #[cfg(target_os = "macos")]
    pub ns_view: *mut std::ffi::c_void,
}

/// RHI 接口
pub trait Rhi: Send + Sync {
    /// 初始化
    fn init(desc: &RhiInitDesc) -> Self where Self: Sized;
    
    /// 开始一帧
    fn begin_frame(&mut self);
    
    /// 提交绘制命令
    fn draw(&mut self, commands: &[DrawCommand]);
    
    /// 结束一帧
    fn end_frame(&mut self) -> RenderResult;
    
    /// 调整窗口大小
    fn resize(&mut self, width: u32, height: u32);
    
    /// 获取当前帧缓冲区大小
    fn framebuffer_size(&self) -> (u32, u32);
    
    /// 等待设备空闲
    fn wait_idle(&self);
}