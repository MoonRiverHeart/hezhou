#version 450

layout(push_constant) uniform PushConstants {
    float rotation;
    float scale;
    float outline_r;
    float outline_g;
    float outline_b;
    float outline_a;
    float width;
    float height;
    float cameraYaw;
    float cameraPitch;
    float cameraX;
    float cameraY;
    float cameraZ;
} pc;

// Perspective projection matrix (Vulkan: flip Y axis)
mat4 perspective(float fov, float aspect, float near, float far) {
    float f = 1.0 / tan(fov * 0.5);
    return mat4(
        f / aspect, 0.0, 0.0, 0.0,
        0.0, -f, 0.0, 0.0,  // -f to flip Y for Vulkan
        0.0, 0.0, (far + near) / (near - far), -1.0,
        0.0, 0.0, (2.0 * far * near) / (near - far), 0.0
    );
}

// Cube vertices (8 corners)
vec3 positions[8] = vec3[](
    vec3(-0.5, -0.5, -0.5),  // 0: back-bottom-left
    vec3( 0.5, -0.5, -0.5),  // 1: back-bottom-right
    vec3( 0.5,  0.5, -0.5),  // 2: back-top-right
    vec3(-0.5,  0.5, -0.5),  // 3: back-top-left
    vec3(-0.5, -0.5,  0.5),  // 4: front-bottom-left
    vec3( 0.5, -0.5,  0.5),  // 5: front-bottom-right
    vec3( 0.5,  0.5,  0.5),  // 6: front-top-right
    vec3(-0.5,  0.5,  0.5)   // 7: front-top-left
);

// 36 vertices for 6 faces (2 triangles per face)
// Physical CCW winding (normal points outward)
int vertex_indices[36] = int[](
    // Back face (Z-, z=-0.5) - red: normal points to -Z
    0, 2, 1, 0, 3, 2,
    // Front face (Z+, z=+0.5) - green: normal points to +Z
    4, 7, 6, 4, 6, 5,
    // Left face (X-, x=-0.5) - blue: normal points to -X
    0, 7, 3, 0, 4, 7,
    // Right face (X+, x=+0.5) - yellow: normal points to +X
    1, 2, 6, 1, 6, 5,
    // Bottom face (Y-, y=-0.5) - cyan: normal points to -Y
    0, 1, 5, 0, 5, 4,
    // Top face (Y+, y=+0.5) - magenta: normal points to +Y
    3, 7, 6, 3, 6, 2
);

// Face colors
vec3 face_colors[6] = vec3[](
    vec3(1.0, 0.2, 0.2),  // back: red
    vec3(0.2, 1.0, 0.2),  // front: green
    vec3(0.2, 0.2, 1.0),  // left: blue
    vec3(1.0, 1.0, 0.2),  // right: yellow
    vec3(0.2, 1.0, 1.0),  // bottom: cyan
    vec3(1.0, 0.2, 1.0)   // top: magenta
);

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec4 outlineColor;

void main() {
    int vertex_idx = vertex_indices[gl_VertexIndex];
    int face_idx = gl_VertexIndex / 6;
    
    vec3 pos = positions[vertex_idx];
    
    // Apply scale
    pos = pos * pc.scale;
    
    // Model transform: rotate around Y (cube self-rotation)
    float angle = pc.rotation;
    float cosA = cos(angle);
    float sinA = sin(angle);
    
    vec3 model_pos = vec3(
        pos.x * cosA - pos.z * sinA,
        pos.y,
        pos.x * sinA + pos.z * cosA
    );
    
    // View transform: translate to camera space then rotate
    vec3 translated_pos = model_pos - vec3(pc.cameraX, pc.cameraY, pc.cameraZ);
    
    // Camera rotation: pitch (X axis) then yaw (Y axis)
    float yaw = pc.cameraYaw;
    float pitch = pc.cameraPitch;
    
    float cosP = cos(pitch);
    float sinP = sin(pitch);
    vec3 pitched_pos = vec3(
        translated_pos.x,
        translated_pos.y * cosP - translated_pos.z * sinP,
        translated_pos.y * sinP + translated_pos.z * cosP
    );
    
    float cosY = cos(yaw);
    float sinY = sin(yaw);
    vec3 view_pos = vec3(
        pitched_pos.x * cosY + pitched_pos.z * sinY,
        pitched_pos.y,
        -pitched_pos.x * sinY + pitched_pos.z * cosY
    );
    
    // Perspective projection (dynamic aspect ratio)
    float aspect = pc.width / pc.height;
    mat4 proj = perspective(1.0472, aspect, 0.1, 100.0); // 60° FOV
    gl_Position = proj * vec4(view_pos, 1.0);
    
    fragColor = face_colors[face_idx];
    outlineColor = vec4(pc.outline_r, pc.outline_g, pc.outline_b, pc.outline_a);
}