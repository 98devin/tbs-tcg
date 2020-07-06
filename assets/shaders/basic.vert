
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
layout(location = 2) in  vec3 a_Normal;

layout(location = 0) out vec3 v_Position;
layout(location = 1) out vec2 v_Texcoord;
layout(location = 2) out vec3 v_Normal;

out gl_PerVertex {
    vec4 gl_Position;
}; 

void main()
{
    vec4 v_Wposition = camera.view * vec4(a_Position, 1.0);
    v_Position = v_Wposition.xyz;
    gl_Position = proj * v_Wposition;
    v_Texcoord = a_Texcoord;
    v_Normal   = a_Normal;
}