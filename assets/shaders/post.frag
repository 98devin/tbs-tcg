
#version 450

layout(set = 0, binding = 0) uniform texture2D u_Texture;
layout(set = 0, binding = 1) uniform sampler   u_Sampler;

layout(location = 0) in  vec2 v_Texcoord;


layout(location = 0) out vec4 f_Color;



vec3 hdr_correct(vec3 color)
{
    return color / (color + 1.0);
}


vec3 gamma_correct(vec3 color)
{
    const float gamma = 2.2;
    return pow(color, vec3(1.0 / gamma));
}



void main()
{
    vec3 color = texture(sampler2D(u_Texture, u_Sampler), v_Texcoord).rgb;

    color = hdr_correct(color);

    // TODO: Work in linear color space so this applies
    // color = gamma_correct(color);


    f_Color = vec4(color, 1.0);    
}