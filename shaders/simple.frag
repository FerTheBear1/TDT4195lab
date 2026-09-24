#version 430 core

#define PI 3.14159265358979323846264338327950288
#define TAU PI * 2.0

out vec4 color;

uniform vec2 u_resolution;

in vec2 f_local;
in vec4 f_color;
in vec3 f_normal;


void main() {
    vec3 normal = normalize(f_normal);
    vec3 light_direction = normalize(vec3(0.8, -0.5, 0.6));

    float diffuse = max(dot(-light_direction, normal), 0.0);

    color.rgb = vec3(1.0) * diffuse;
    color.a = 1.0;


    /* color = vec4(vec3(step(length(local - 0.5), 0.5)), 1.0); */
}
