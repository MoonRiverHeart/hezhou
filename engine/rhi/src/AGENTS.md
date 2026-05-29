# RHI Abstraction Layer

Abstract rendering hardware interface traits. Vulkan implementation in rhi-vulkan/.

## Overview
Defines Device, CommandBuffer, Pipeline, RenderPass, Buffer, Texture, SwapChain traits. rhi-vulkan crate implements these for Vulkan API. 13 modules covering all GPU resource types. RhiResult<T> = Result<T, RhiError>.

## Where To Look
| Task | File | Notes |
|------|------|-------|
| Add GPU resource type | Appropriate module file | e.g. texture.rs for TextureDesc/TextureFormat |
| Fix pipeline abstraction | `pipeline.rs` | 315 lines — PipelineDesc, BlendState, DepthStencilState, ShaderStage |
| Fix render pass | `pass.rs` | AttachmentDesc, LoadOp/StoreOp, ImageLayout |
| Fix command buffer | `command.rs` | ClearValue, CommandBuffer trait, IndexType |
| Fix UI rendering trait | `ui.rs` | UIVertex, UIDrawData, UIPipelineDesc, UIRenderTarget, UIRenderer trait |
| Add new trait method | trait file + rhi-vulkan impl | Must implement in rhi-vulkan counterpart |

## Key Types
- `Device` trait: GPU device abstraction, creates resources
- `CommandBuffer` trait: render commands (draw, bind pipeline, set viewport)
- `PipelineDesc`: full pipeline state (blend, depth, raster, shader stages, layout)
- `TextureDesc/TextureFormat/TextureUsage`: GPU texture abstraction
- `BufferDesc/BufferType/BufferUsage/MemoryLocation`: GPU buffer abstraction
- `UIRenderer` trait: special UI rendering interface (draw UI vertices)

## Conventions
- All traits in rhi/, all implementations in rhi-vulkan/
- Desc structs: *Desc pattern (PipelineDesc, BufferDesc, TextureDesc, RenderPassDesc, etc.)
- Handle types in handle.rs: typed wrappers around raw GPU handles
- Error type: RhiError, result alias: RhiResult<T>
- UI module has specialized types (UIVertex, UIDrawData) not in main pipeline flow

## Anti-Patterns
- NEVER implement RHI traits outside rhi-vulkan/ (only one backend currently)
- NEVER use raw Vulkan types directly when RHI abstraction exists — use trait methods
- NEVER mismatch PipelineDesc blend/depth state with actual Vulkan pipeline creation