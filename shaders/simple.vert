#version 430 core

in vec3 position;

const vec2 corners[4] = vec2[](vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
const uint indices[6] = { 0, 2, 3, 0, 3, 1 };

out vec2 local;

void main() {
    gl_Position = vec4(position * vec3(-1.0, -1.0, 1.0), 1.0f);
    local = corners[gl_VertexID];
}
