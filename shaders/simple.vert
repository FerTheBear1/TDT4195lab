#version 430 core

in vec3 a_position;
in vec4 a_color;

uniform mat4 u_transform;

const vec2 corners[4] = vec2[](vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
const uint indices[6] = { 0, 2, 3, 0, 3, 1 };

out vec2 f_local;
out vec4 f_color;

void main() {
    gl_Position = u_transform * vec4(a_position, 1.0f);
    f_local = corners[gl_VertexID];
    f_color = a_color;
}
