
#version 450

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    vec3 pos;
    vec3 dir;
    vec3 top;
} camera;

layout(set = 1, binding = 0) uniform texture2D u_Texture;
layout(set = 1, binding = 1) uniform sampler   u_Sampler;

layout(location = 0) in  vec3 v_Position;
layout(location = 1) in  vec2 v_Texcoord;
layout(location = 2) in  vec3 v_Normal;


layout(location = 0) out vec4 f_Color;


const vec3 light_pos   = vec3(3.0);
const vec4 light_color = vec4(1.0);

void main()
{
    float angle_factor = max(0.0, dot(normalize(light_pos - v_Position), normalize(v_Normal)));
    vec4 texture_sample = texture(sampler2D(u_Texture, u_Sampler), v_Texcoord);
    vec4 light_value = pow(vec4(angle_factor), vec4(5.0)) * light_color;
    f_Color = clamp(vec4(0.0), vec4(1.0), texture_sample + light_value);
}