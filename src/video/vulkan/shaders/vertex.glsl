#version 450

// Input
layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}