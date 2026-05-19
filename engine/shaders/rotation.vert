#version 450

layout(push_constant) uniform PushConstants {
    float rotation;
} pc;

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
// Face indices: 0=back, 1=front, 2=left, 3=right, 4=bottom, 5=top
int vertex_indices[36] = int[](
    // Back face (Z-) - red
    0, 2, 1,
    0, 3, 2,
    // Front face (Z+) - green
    4, 5, 6,
    4, 6, 7,
    // Left face (X-) - blue
    0, 4, 7,
    0, 7, 3,
    // Right face (X+) - yellow
    1, 6, 5,
    1, 2, 6,
    // Bottom face (Y-) - cyan
    0, 1, 5,
    0, 5, 4,
    // Top face (Y+) - magenta
    3, 7, 6,
    3, 6, 2
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
    int face_idx = gl_VertexIndex / 6;  // Each face has 6 vertices
    
    vec3 pos = positions[vertex_idx];
    
    // Rotate around Y axis
    float angle = pc.rotation;
    float cosA = cos(angle);
    float sinA = sin(angle);
    
    vec3 rotated = vec3(
        pos.x * cosA - pos.z * sinA,
        pos.y,
        pos.x * sinA + pos.z * cosA
    );
    
    // Slight rotation around X axis for better view
    float tilt = 0.3;
    float cosT = cos(tilt);
    float sinT = sin(tilt);
    
    vec3 final_pos = vec3(
        rotated.x,
        rotated.y * cosT - rotated.z * sinT,
        rotated.y * sinT + rotated.z * cosT
    );
    
    gl_Position = vec4(final_pos, 1.0);
    fragColor = face_colors[face_idx];
}