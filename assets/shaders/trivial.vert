#version 450

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec4 v_color;

const vec3[3] v_colors = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

const vec2[3] v_positions = vec2[](
    vec2(-1.0, -1.0),
    vec2( 0.0,  1.0),
    vec2( 1.0, -1.0)
);

void main()
{
    gl_Position = vec4(v_positions[gl_VertexIndex], 0.0, 1.0);
    v_color = vec4(v_colors[gl_VertexIndex], 1.0);
}