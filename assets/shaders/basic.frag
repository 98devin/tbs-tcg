
#version 450

layout(set = 1, binding = 0) uniform texture2D u_Texture;
layout(set = 1, binding = 1) uniform sampler u_Sampler;

layout(location = 0) in  vec2 v_Texcoord;

layout(location = 0) out vec4 f_Color;

void main()
{
    f_Color = texture(sampler2D(u_Texture, u_Sampler), v_Texcoord);
}