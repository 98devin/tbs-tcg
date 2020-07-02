
#version 450

layout(set = 0, binding = 0) uniform texture2D u_Texture;
layout(set = 0, binding = 1) uniform sampler   u_Sampler;

layout(location = 0) in  vec2 v_Texcoord;


layout(location = 0) out vec4 f_Color;


void main()
{
    // identity post-process (with linear scaling)
    f_Color = mix(
        vec4(v_Texcoord, 0.0, 1.0),
        1.0 - texture(sampler2D(u_Texture, u_Sampler), v_Texcoord),
        0.5
    );
    // f_Color = ;
}