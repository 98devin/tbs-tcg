
#version 450

out gl_PerVertex {
    vec4 gl_Position;
}; 

layout(location = 0) in  vec3 a_position;
layout(location = 1) in  vec3 a_color;

layout(location = 0) out vec4 v_color;

void main()
{
    gl_Position = vec4(a_position, 1.0);
    v_color     = vec4(a_color, 1.0);
}