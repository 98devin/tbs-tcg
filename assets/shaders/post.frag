
#version 450

out gl_PerVertex {
    vec4 gl_Position;
}; 

layout(location = 0) out vec2 v_Texcoord;


const vec2[3] v_Coords = vec2[3](
    vec2(0.0, 0.0),
    vec2(2.0, 0.0),
    vec2(0.0, 2.0)
);

void main()
{
    vec2 coord = v_Coords[gl_VertexIndex];
    gl_Position = vec4(coord, 0.0, 1.0);
    v_Texcoord = coord;
}