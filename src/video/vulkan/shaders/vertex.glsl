#version 450

// Constants
const float VIEW_WIDTH = 2.0;

const uint TEX_ROW_SIZE = 16;

// Corner enum
const uint LEFT     = 0 << 16;
const uint RIGHT    = 1 << 16;

// Functions
//vec2 calc_vertex_wraparound(vec2, uint, uint);
//vec2 calc_vertex_compare(vec2, uint, uint);
vec2 calc_tex_coords(uint);
vec2 get_tex_offset(uint, uint);

// Input
layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(push_constant) uniform PushConstants {
    vec2 tex_size;
    vec2 atlas_size;
    vec2 vertex_offset;
    //uint tex_offset;
    float tex_pixel_height;
    uint palette_offset;
    uint palette_size;
} push_constants;

// Output
layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

void main() {
    // Vertex position offset with scroll / position
    vec2 vertex_position = position + push_constants.vertex_offset;

    /*if ((push_constants.flags & WRAPAROUND) != 0) {
        uint side = data & 0x10000;
        uint tex_y = (data >> 17) % 64;
        vertex_position = calc_vertex_wraparound(vertex_position, side, tex_y);
    }*/

    gl_Position = vec4(vertex_position, 0.0, 1.0);

    texCoordOut = calc_tex_coords(data);

    paletteNumOut = (data >> 10) & 7;
}

vec2 calc_tex_coords(uint tex_data) {
// Unpack texture information
    uint tex_num = tex_data & 0x3FF;
    uint side = tex_data & 0x10000;
    uint tex_y = (tex_data >> 17) % 64;
// Convert to 2D coords
    float x = float(tex_num % TEX_ROW_SIZE) / push_constants.atlas_size.x;
    float y = float(tex_num / TEX_ROW_SIZE) / push_constants.atlas_size.y;
    
    return vec2(x, y) + get_tex_offset(side, tex_y);
}

vec2 get_tex_offset(uint side, uint y) {
    float y_offset = (float(y) * push_constants.tex_size.y) / push_constants.tex_pixel_height;
    switch (side) {
        case LEFT:  return vec2(0.0, y_offset);
        default:    return vec2(push_constants.tex_size.x, y_offset);
    }
}