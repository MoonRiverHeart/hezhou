#version 450

layout(push_constant) uniform PushConstants {
    float rotation;
    float width;
    float height;
} pc;

// Perspective projection matrix
mat4 perspective(float fov, float aspect, float near, float far) {
    float f = 1.0 / tan(fov * 0.5);
    return mat4(
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
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
    // Back face (Z-) - red (looking from Z- toward Z+)
    0, 1, 2, 0, 2, 3,
    // Front face (Z+) - green (looking from Z+ toward Z-)
    4, 6, 5, 4, 7, 6,
    // Left face (X-) - blue (looking from X- toward X+)
    0, 7, 4, 0, 3, 7,
    // Right face (X+) - yellow (looking from X+ toward X-)
    1, 5, 6, 1, 6, 2,
    // Bottom face (Y-) - cyan (looking from Y- toward Y+)
    0, 5, 1, 0, 4, 5,
    // Top face (Y+) - magenta (looking from Y+ toward Y-)
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
    
    // Model transform: rotate around Y then tilt around X
    float angle = pc.rotation;
    float cosA = cos(angle);
    float sinA = sin(angle);
    
    vec3 rotated_y = vec3(
        pos.x * cosA - pos.z * sinA,
        pos.y,
        pos.x * sinA + pos.z * cosA
    );
    
    float tilt = 0.5;
    float cosT = cos(tilt);
    float sinT = sin(tilt);
    
    vec3 model_pos = vec3(
        rotated_y.x,
        rotated_y.y * cosT - rotated_y.z * sinT,
        rotated_y.y * sinT + rotated_y.z * cosT
    );
    
    // View transform: push cube back so it's visible (z = -3)
    vec3 view_pos = model_pos + vec3(0.0, 0.0, -3.0);
    
    // Perspective projection (dynamic aspect ratio)
    float aspect = pc.width / pc.height;
    mat4 proj = perspective(1.0472, aspect, 0.1, 100.0); // 60° FOV
    gl_Position = proj * vec4(view_pos, 1.0);
    
    fragColor = face_colors[face_idx];
}