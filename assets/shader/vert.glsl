#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec4 inColor;
layout(location = 2) in vec2 inUV;

layout(push_constant) uniform PushConstants {
    float screenWidth;
    float screenHeight;
    float offsetX;
    float offsetY;
} pc;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 fragUV;

void main() {
    // 屏幕坐标 → NDC
    float x = (inPosition.x / pc.screenWidth) * 2.0 - 1.0;
    float y = (inPosition.y / pc.screenHeight) * 2.0 - 1.0;
    
    gl_Position = vec4(x, y, 0.0, 1.0);
    fragColor = inColor;
    fragUV = inUV;
}