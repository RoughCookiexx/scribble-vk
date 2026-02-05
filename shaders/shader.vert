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

layout(push_constant) uniform PushConstants {
    mat3 transform;
} push;

void main() {
    vec2 n = vec2(-dir.y, dir.x) / length(dir);
    vec2 apos = pos.y * dir + pos.x * n * THICKNESS;
    vec2 world_pos = apos + inst_pos;
    vec3 transformed = push.transform * vec3(world_pos, 1.0);
    gl_Position = vec4(transformed.xy, 0.0, 1.0);

    local_position = pos;
    projected_position = vec2(world_pos.x, world_pos.y);
    instance_position = inst_pos;
    direction = dir;
    thickness = THICKNESS;
}
