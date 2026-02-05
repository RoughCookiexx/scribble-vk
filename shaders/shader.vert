#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 dir;
layout(location = 2) in vec2 inst_pos;

layout(location = 0) out vec2 local_position;
layout(location = 1) out vec2 projected_position;
layout(location = 2) out vec2 instance_position;
layout(location = 3) out float thickness;
layout(location = 4) out vec2 direction;

const float THICKNESS = 5;

layout(binding = 0) uniform UniformBufferObject {
    mat4 mvp;
} ubo;
void main() {
    vec2 n = vec2(-dir.y, dir.x) / length(dir);
    vec2 apos = pos.y * dir + pos.x * n * THICKNESS;
    vec4 new_pos = vec4(apos + inst_pos, 0.0, 1.0);
    vec4 res_pos = ubo.mvp * new_pos;
    gl_Position = res_pos;

    local_position = pos;
    projected_position = vec2(new_pos.x, new_pos.y);
    instance_position = inst_pos;
    direction = dir;
    thickness = THICKNESS;
}
