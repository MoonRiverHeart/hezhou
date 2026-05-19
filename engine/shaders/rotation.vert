#version 450

layout(push_constant) uniform PushConstants {
    float rotation;
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
// All faces use COUNTER_CLOCKWISE winding (looking from outside)
int vertex_indices[36] = int[](
    // Back face (Z-) - red
    0, 1, 2, 0, 2, 3,
    // Front face (Z+) - green
    4, 7, 6, 4, 6, 5,
    // Left face (X-) - blue
    0, 3, 7, 0, 7, 4,
    // Right face (X+) - yellow
    1, 5, 6, 1, 6, 2,
    // Bottom face (Y-) - cyan
    0, 1, 5, 0, 5, 4,
    // Top face (Y+) - magenta
    3, 2, 6, 3, 6, 7
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

void main() {
    int vertex_idx = vertex_indices[gl_VertexIndex];
    int face_idx = gl_VertexIndex / 6;
    
    vec3 pos = positions[vertex_idx];
    
    // Model transform: rotate around Y (cube self-rotation)
    float angle = pc.rotation;
    float cosA = cos(angle);
    float sinA = sin(angle);
    
    vec3 model_pos = vec3(
        pos.x * cosA - pos.z * sinA,
        pos.y,
        pos.x * sinA + pos.z * cosA
    );
    
    // Camera view: yaw (Y rotation) + pitch (X rotation)
    float yaw = pc.cameraYaw;
    float pitch = pc.cameraPitch;
    
    // First: pitch (rotate around X axis)
    float cosP = cos(pitch);
    float sinP = sin(pitch);
    vec3 pitched_pos = vec3(
        model_pos.x,
        model_pos.y * cosP - model_pos.z * sinP,
        model_pos.y * sinP + model_pos.z * cosP
    );
    
    // Second: yaw (rotate around Y axis)
    float cosY = cos(yaw);
    float sinY = sin(yaw);
    vec3 yawed_pos = vec3(
        pitched_pos.x * cosY + pitched_pos.z * sinY,
        pitched_pos.y,
        -pitched_pos.x * sinY + pitched_pos.z * cosY
    );
    
    // View transform: translate by camera position
    vec3 view_pos = yawed_pos - vec3(pc.cameraX, pc.cameraY, pc.cameraZ);
    
    // Perspective projection (dynamic aspect ratio)
    float aspect = pc.width / pc.height;
    mat4 proj = perspective(1.0472, aspect, 0.1, 100.0); // 60° FOV
    gl_Position = proj * vec4(view_pos, 1.0);
    
    fragColor = face_colors[face_idx];
}