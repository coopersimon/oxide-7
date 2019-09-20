#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);

    switch (data & 3) {
        case 0: texCoordOut = vec2(0.0, 0.0); break;
        case 1: texCoordOut = vec2(1.0, 0.0); break;
        case 2: texCoordOut = vec2(0.0, 1.0); break;
        default: texCoordOut = vec2(1.0, 1.0); break;
    }

    paletteNumOut = 0;
}