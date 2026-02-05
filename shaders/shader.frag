#version 450

layout(location = 0) in vec2 local_position;
layout(location = 1) in vec2 projected_position;
layout(location = 2) in vec2 instance_position;
layout(location = 3) in float thickness;
layout(location = 4) in vec2 direction;

layout(location = 0) out vec4 outColor;

layout(push_constant) uniform PushConstants {
    vec3 transform;
} push;

const float aaborder = 0.00445;

float line_segment(in vec2 p, in vec2 a, in vec2 b) {
    vec2 ba = b - a;
    vec2 pa = p - a;
    float h = clamp(dot(pa, ba) / dot(ba, ba), 0., 1.);
    return length(pa - h * ba);
}

void main() {
    vec2 a = instance_position - direction / 2.;
    vec2 b = instance_position + direction / 2.;
    float d = line_segment(projected_position, a, b) - thickness;
    // Use scale component (z) for anti-aliasing border
    float scaled_border = aaborder / push.transform.z;
    float edge1 = -scaled_border;
    float edge2 = 0.;

    if (d < 0.) {
        float alpha = 1.;

        if (d > edge1) {
            alpha = 1. - smoothstep(edge1, edge2, d);
        }
        vec4 color = vec4(1, 1, 1, alpha);
        outColor = color;
    } else {
        outColor = vec4(1, 1, 1, 0.0);
    }
}
