#version 450

// use https://alexaltea.github.io/glslang.js/ if you are lazy to convert shaders

layout(location = 0) in vec3 a_Pos;
layout(location = 1) in vec4 a_Color;

layout(location = 0) out vec4 f_Color;

layout(set=0, binding=0)
uniform Uniforms {
    mat4x4 u_view_proj;
};

void main() {
    vec4 column = u_view_proj[3];
    f_Color = a_Color;
    gl_Position = u_view_proj * vec4(a_Pos, 1.0);
}
