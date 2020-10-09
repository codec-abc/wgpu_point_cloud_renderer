#version 450

layout(location = 0) in vec3 a_Pos;
layout(location = 1) in vec4 a_Color;

layout(location = 0) out vec4 f_Color;

layout(set=0, binding=0) // 1.
uniform Uniforms {
    //mat4 u_view_proj; // 2.
    vec4 uniform_color;
};


void main() {
    f_Color = uniform_color;//a_Color;
    mat4 matrix = mat4(1.0);
    gl_Position = matrix * vec4(a_Pos, 1.0);
}
