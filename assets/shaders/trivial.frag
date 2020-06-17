#version 450

layout(location = 0) out vec4 outColor;

#define RED vec3(1.0, 0.0, 0.0)

void main()
{
    outColor = vec4(RED, 1.0);
}