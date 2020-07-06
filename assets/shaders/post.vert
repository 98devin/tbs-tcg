
#version 450

out gl_PerVertex {
    vec4 gl_Position;
}; 

layout(location = 0) out vec2 v_Texcoord;


const vec2[3] v_Positions = vec2[3](
    vec2(-1.0, -1.0),
    vec2(-1.0, 3.0),
    vec2(3.0, -1.0)
);

const vec2[3] v_Texcoords = vec2[3](
    vec2(0.0, 1.0),
    vec2(0.0, -1.0),
    vec2(2.0, 1.0)
);

void main()
{
    vec2 pos = v_Positions[gl_VertexIndex];
    gl_Position = vec4(pos, 0.0, 1.0);

    vec2 uv = v_Texcoords[gl_VertexIndex];
    v_Texcoord = uv;
}