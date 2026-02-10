#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 dir;
layout(location = 2) in vec2 inst_pos;

layout(location = 0) out vec2 local_position;
layout(location = 1) out vec2 projected_position;
layout(location = 2) out vec2 instance_position;
layout(location = 3) out float thickness;
layout(location = 4) out vec2 direction;

const float THICKNESS = 0.004;

layout(push_constant) uniform PushConstants {
    vec3 transform;
} push;

void main() {
    vec2 n = vec2(-dir.y, dir.x) / length(dir);
    vec2 apos = pos.y * dir + pos.x * n * THICKNESS;
    vec2 world_pos = apos + inst_pos;

    // Apply transform: push.transform = (offset_x, offset_y, scale)
    vec2 scaled_pos = world_pos * push.transform.z;
    vec2 final_pos = scaled_pos + push.transform.xy;
    gl_Position = vec4(final_pos, 0.0, 1.0);

    local_position = pos;
    projected_position = vec2(world_pos.x, world_pos.y);
    instance_position = inst_pos;
    direction = dir;
    thickness = THICKNESS;
}
