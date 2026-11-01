#version 450
// Hezhou 0.2 骨骼蒙皮顶点着色器（契约示例，需手动 glslc 编译）
//
// 用途：为 skinned mesh 提供 GPU 蒙皮。数据契约见 engine/animation/src/gpu.rs：
//   - 顶点顶点布局：position(0)/normal(1)/uv(2)/jointIndices(3,uvec4)/jointWeights(4,vec4)
//   - stride = 64 bytes，前 32 字节与现有 rotation.vert 的非蒙皮顶点一致
//   - joint palette：std430 storage buffer，set=0 binding=1，mat4[]，元素 = world * inverseBind
//   - push constant 布局与 rotation.vert 完全一致（144 bytes），可直接复用 raster 侧
//     现有 push constant 上传路径；蒙皮只作用于 model 之前的模型空间。
//
// 渲染侧接入方式（0.2 建议，由 render-owner 实施）：
//   1) 新增 pipeline：vertex shader = skinning.vert.spv，fragment 可复用 rotation.frag.spv
//   2) 顶点输入 stride 从 32 改成 64，并增加 location 3/4 两个属性
//   3) 每角色创建 joint palette storage buffer（或 UBO），descriptor set 0 binding 1
//   4) draw 前上传当前帧 palette（JointPalette::to_le_bytes）
//
// 编译：
//   glslc engine/shaders/skinning.vert -o engine/shaders/skinning.vert.spv

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;
layout(location = 3) in uvec4 inJointIndices;
layout(location = 4) in vec4 inJointWeights;

// joint palette（模型空间蒙皮矩阵：world_joint * inverse_bind）
layout(std430, set = 0, binding = 1) readonly buffer JointPalette {
    mat4 joints[];
};

// 与 rotation.vert 保持严格一致的 144 bytes push constant 布局
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
    float has_texture;           // 4 bytes — 1.0 if entity has texture
    float specular_strength;     // 4 bytes
    float ambient_strength;      // 4 bytes
    float shininess;             // 4 bytes
    // Total: 144 bytes
} pc;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out float fragOutline;
layout(location = 3) out vec3 fragPosition;
layout(location = 4) out vec2 fragUV;

mat4 viewMatrix() {
    float cy = cos(pc.camera_yaw);
    float sy = sin(pc.camera_yaw);
    float cp = cos(pc.camera_pitch);
    float sp = sin(pc.camera_pitch);
    vec3 forward = vec3(sy * cp, -sp, -cy * cp);
    vec3 right = vec3(cy, 0, sy);
    vec3 up = vec3(sy * sp, cp, -cy * sp);
    vec3 eye = pc.camera_pos;

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
        0, -f, 0, 0,
        0, 0, far / (near - far), -1,
        0, 0, near * far / (near - far), 0
    );
}

void main() {
    // CPU 侧已归一化权重；这里再做容错，避免未蒙皮顶点被零矩阵压扁。
    vec4 weights = inJointWeights;
    float totalWeight = dot(weights, vec4(1.0));

    mat4 skin = mat4(0.0);
    if (totalWeight > 1e-6) {
        skin = weights.x * joints[inJointIndices.x]
             + weights.y * joints[inJointIndices.y]
             + weights.z * joints[inJointIndices.z]
             + weights.w * joints[inJointIndices.w];
        if (abs(totalWeight - 1.0) > 1e-3) {
            skin = skin / totalWeight;
        }
    }

    vec4 localPosition = vec4(inPosition, 1.0);
    vec3 localNormal = inNormal;
    if (totalWeight > 1e-6) {
        localPosition = skin * localPosition;
        localNormal = mat3(skin) * localNormal;
    }

    vec4 worldPos = pc.model * localPosition;
    mat4 view = viewMatrix();
    mat4 proj = projectionMatrix(pc.viewport_size.x / pc.viewport_size.y);

    gl_Position = proj * view * worldPos;

    // 与 rotation.vert 一致的法线方向面着色（占位，PBR 由 render-owner 的管线接管）
    vec3 absN = abs(localNormal);
    if (absN.x > absN.y && absN.x > absN.z) {
        fragColor = localNormal.x > 0 ? vec3(0.8, 0.2, 0.2) : vec3(0.6, 0.15, 0.15);
    } else if (absN.y > absN.z) {
        fragColor = localNormal.y > 0 ? vec3(0.2, 0.8, 0.2) : vec3(0.15, 0.6, 0.15);
    } else {
        fragColor = localNormal.z > 0 ? vec3(0.2, 0.2, 0.8) : vec3(0.15, 0.15, 0.6);
    }

    fragNormal = mat3(pc.model) * localNormal;
    fragOutline = pc.is_selected;
    fragPosition = worldPos.xyz;
    fragUV = inUV;
}
