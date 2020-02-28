#version 450

layout(location = 0) in vec2 texCoord;

layout(set = 0, binding = 0) uniform usampler2D tex;

layout(location = 0) out vec4 outColor;

void main() {
    //outColor = vec4(texture(tex, texCoord).xyz, 1.0);
    vec3 rgb = texture(tex, texCoord).xyz;
    outColor = vec4(rgb.x / 255.0, rgb.y / 255.0, rgb.z / 255.0, 1.0);
}