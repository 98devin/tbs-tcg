
#version 450

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    vec3 pos;
    vec3 dir;
    vec3 top;
} camera;

layout(set = 0, binding = 1) uniform Projection {
    mat4 proj;
};

layout(location = 0) in  vec3 a_Position;
layout(location = 1) in  vec2 a_Texcoord;

layout(location = 0) out vec2 v_Texcoord;


out gl_PerVertex {
    vec4 gl_Position;
}; 

void main()
{
    gl_Position = proj * camera.view * vec4(a_Position, 1.0);
    v_Texcoord  = a_Texcoord;
}