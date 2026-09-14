#version 430 core

out vec4 color;

uniform vec2 u_resolution;

in vec2 local;

void main() {
    color = vec4(1.0, 0.5, 0.2, 1.0);
    /* color = vec4(vec3(step(length(local - 0.5), 0.5)), 1.0); */
}
