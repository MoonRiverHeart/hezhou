#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;

layout(push_constant) uniform PushConstants {
    mat4 model;           // 64 bytes - per-entity model matrix
    vec3 outline_color;   // 12 bytes - selection highlight color
    float is_selected;    // 4 bytes - 1.0 if selected, 0.0 if not
    vec2 viewport_size;   // 8 bytes - game viewport dimensions
    float _pad0;          // 4 bytes - padding for vec3 alignment
    float _pad1;          // 4 bytes - padding for vec3 alignment
    vec3 camera_pos;      // 12 bytes - camera position (at offset 96, 16-byte aligned)
    float camera_yaw;     // 4 bytes
    float camera_pitch;   // 4 bytes
    float _pad2;          // 4 bytes - padding to round struct size
    // Total: 128 bytes, within 128-byte limit
} pc;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out float fragOutline;

mat4 viewMatrix() {
    float cy = cos(pc.camera_yaw);
    float sy = sin(pc.camera_yaw);
    float cp = cos(pc.camera_pitch);
    float sp = sin(pc.camera_pitch);
    vec3 forward = vec3(sy * cp, -sp, -cy * cp);
    vec3 right = vec3(cy, 0, -sy);
    vec3 up = vec3(sy * sp, cp, cy * sp);
    vec3 eye = pc.camera_pos;
    
    // OpenGL/Vulkan convention: camera looks along -Z, so negate forward in view matrix
    return mat4(
        right.x, up.x, -forward.x, 0,
        right.y, up.y, -forward.y, 0,
        right.z, up.z, -forward.z, 0,
        -dot(right, eye), -dot(up, eye), dot(forward, eye), 1
    );
}

mat4 projectionMatrix(float aspect) {
     float fov = 60.0;
    float near = 0.1;
    float far = 100.0;
    float f = 1.0 / tan(fov * 3.14159265 / 360.0);
    return mat4(
        f / aspect, 0, 0, 0,
        0, f, 0, 0,
        0, 0, (far + near) / (near - far), -1,
        0, 0, 2 * far * near / (near - far), 0
    );
}

void main() {
    vec4 worldPos = pc.model * vec4(inPosition, 1.0);
    mat4 view = viewMatrix();
    mat4 proj = projectionMatrix(pc.viewport_size.x / pc.viewport_size.y);
    
    gl_Position = proj * view * worldPos;
    
    // Face colors based on normal direction
    vec3 absN = abs(inNormal);
    if (absN.x > absN.y && absN.x > absN.z) {
        fragColor = inNormal.x > 0 ? vec3(0.8, 0.2, 0.2) : vec3(0.6, 0.15, 0.15);
    } else if (absN.y > absN.z) {
        fragColor = inNormal.y > 0 ? vec3(0.2, 0.8, 0.2) : vec3(0.15, 0.6, 0.15);
    } else {
        fragColor = inNormal.z > 0 ? vec3(0.2, 0.2, 0.8) : vec3(0.15, 0.15, 0.6);
    }
    
    fragNormal = mat3(pc.model) * inNormal;
    fragOutline = pc.is_selected;
}