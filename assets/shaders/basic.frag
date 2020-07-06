
#version 450

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    vec3 pos;
    vec3 dir;
    vec3 top;
} camera;

layout(set = 1, binding = 0) uniform texture2D u_Texture;
layout(set = 1, binding = 1) uniform sampler   u_Sampler;

layout(set = 2, binding = 0) uniform texture2D u_Normals;
layout(set = 2, binding = 1) uniform sampler   u_NormSampler;


layout(location = 0) in vec3 v_Position;
layout(location = 1) in vec2 v_Texcoord;
layout(location = 2) in vec3 v_Normal;


layout(location = 0) out vec4 f_Color;


const vec3 light_pos   = vec3(3.0);
const vec3 light_color = vec3(100.0);



// code adapted from "Normal Mapping Without Precomputed Tangents",
// http://www.thetenthplanet.de/archives/1180
mat3 cotangent_frame(vec3 N, vec3 p, vec2 uv)
{
    // get screen-space position vectors
    vec3 dPdx = dFdx(p);
    vec3 dPdy = dFdy(p);
    
    // get screen-space texture vectors
    vec2 dUVdx = dFdx(uv);
    vec2 dUVdy = dFdy(uv);

    // solve for position-texture conversion basis
    vec3 dPdx_perp = cross(N, dPdx);
    vec3 dPdy_perp = cross(dPdy, N);
    
    vec3 T = dPdy_perp * dUVdx.x + dPdx_perp * dUVdy.x;
    vec3 B = dPdy_perp * dUVdx.y + dPdx_perp * dUVdy.y;

    float inv_scale = inversesqrt( max(dot(T, T), dot(B, B)) );
    return mat3(
        T * inv_scale,
        B * inv_scale,
        N
    );
}




void main()
{
    vec4 tex_color = texture(sampler2D(u_Texture, u_Sampler), v_Texcoord);
    vec3 normal    = texture(sampler2D(u_Normals, u_NormSampler), v_Texcoord).xyz;
    normal.xy = normal.xy * 2.0 - 1.0;
    normal.y *= -1;

    mat3 perturb = cotangent_frame(normalize(v_Normal), camera.pos - v_Position, v_Texcoord);

    normal = perturb * normal;
    // normal = vec4(inverse(transpose(camera.view)) * vec4(normal, 0.0)).xyz;


    float angle_factor = pow(max(0.0, dot(normal, vec3(0.0, 0.0, 1.0))), 5.0);

    f_Color = vec4(tex_color.rgb * mix(vec3(0.1), vec3(5.0), angle_factor), 1.0);
}