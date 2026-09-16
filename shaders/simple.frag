#version 430 core

out vec4 color;

uniform vec2 u_resolution;

in vec2 f_local;
in vec4 f_color;

void main() {
    color = f_color;
    /* color = vec4(vec3(step(length(local - 0.5), 0.5)), 1.0); */
}
