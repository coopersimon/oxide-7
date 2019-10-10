#version 450

// Constants
const float VIEW_WIDTH = 2.0;
const float VIEW_HEIGHT = 2.0;

const float ATLAS_SIZE_X = 16.0;
const float ATLAS_SIZE_Y = 16.0;
const uint TEX_ROW_SIZE = 16;
const float LINE_HEIGHT = 1.0 / (ATLAS_SIZE_Y * 8.0);

// Side enum
const uint LEFT     = 0 << 16;
const uint RIGHT    = 1 << 16;

// Size enum
const uint SMALL    = 0 << 23;
const uint LARGE    = 1 << 23;

// Functions
vec2 calc_tex_coords(uint);
vec2 get_tex_offset(uint, uint, vec2);

// Input
layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(push_constant) uniform PushConstants {
    vec4 depth;
    vec2 small_tex_size;
    vec2 large_tex_size;
} push_constants;

// Output
layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

void main() {
    uint priority = (data >> 12) & 3;
    gl_Position = vec4(position, push_constants.depth[priority], 1.0);

    texCoordOut = calc_tex_coords(data);

    paletteNumOut = (data >> 8) & 0xF;
}

// Get texture coordinates from tex number and x and y pos of tile.
vec2 calc_tex_coords(uint tex_data) {
// Unpack texture information
    uint tex_num = tex_data & 0xFF;
    uint side = tex_data & 0x10000;
    uint tex_y = (tex_data >> 17) % 64;
    uint large_tex = tex_data & LARGE;

    vec2 tex_size = large_tex == SMALL ? push_constants.small_tex_size : push_constants.large_tex_size;

// Convert to 2D coords
    float x = float(tex_num % TEX_ROW_SIZE) / ATLAS_SIZE_X;
    float y = float(tex_num / TEX_ROW_SIZE) / ATLAS_SIZE_Y;
    
    return vec2(x, y) + get_tex_offset(side, tex_y, tex_size);
}

// Get texture position based on vertex position.
vec2 get_tex_offset(uint side, uint y, vec2 tex_size) {
    float y_offset = float(y) * LINE_HEIGHT;
    return side == LEFT ? vec2(0.0, y_offset) : vec2(tex_size.x, y_offset);
}