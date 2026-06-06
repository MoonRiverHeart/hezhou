#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec4 inColor;
layout(location = 2) in vec2 inUV;
layout(location = 3) in vec4 inBorderRadius;

layout(push_constant) uniform PushConstants {
    float screenWidth;
    float screenHeight;
    float offsetX;
    float textureId;
    float rectX;
    float rectY;
    float rectW;
    float rectH;
} pc;

layout(location = 0) out vec4 fragColor;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec4 fragBorderRadius;
layout(location = 3) out vec2 fragPos;
layout(location = 4) out vec4 fragRect;
layout(location = 5) out float fragTextureId;

void main() {
    float x = (inPosition.x / pc.screenWidth) * 2.0 - 1.0;
    float y = (inPosition.y / pc.screenHeight) * 2.0 - 1.0;
    
    gl_Position = vec4(x, y, 0.0, 1.0);
    fragColor = inColor;
    fragUV = inUV;
    fragBorderRadius = inBorderRadius;
    fragPos = inPosition;
    fragRect = vec4(pc.rectX, pc.rectY, pc.rectW, pc.rectH);
    fragTextureId = pc.textureId;
}