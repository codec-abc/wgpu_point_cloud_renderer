#version 450

layout(location = 0) in vec3 a_Pos;
layout(location = 1) in vec4 a_Color;

layout(location = 0) out vec4 f_Color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    //gl_Position = vec4(a_Pos, 0.0, 1.0);
    vec4 pos = u_Transform * vec4(a_Pos, 1.0);

    gl_Position = pos;

    //gl_Position = a_Pos;
    //gl_Position = a_Pos;
    f_Color = a_Color;
}
