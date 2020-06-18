#version 450

const vec3 RED = vec3(1.0, 0.0, 0.0);

layout(location = 0) out vec4 outColor;

void main()
{
    outColor = vec4(RED, 1.0);
}