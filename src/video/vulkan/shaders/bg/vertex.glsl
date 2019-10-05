#version 450

// Constants
const float VIEW_WIDTH = 1.0;
const float VIEW_HEIGHT = 1.0;

const uint TEX_ROW_SIZE = 16;

// Tex side enum
const uint TEX_LEFT     = 0 << 16;
const uint TEX_RIGHT    = 1 << 16;

// Vertex side enum
const uint VERTEX_LEFT  = 0 << 21;
const uint VERTEX_RIGHT = 1 << 21;

// Functions
vec2 calc_vertex_wraparound(vec2, uint, uint);
vec2 calc_vertex_compare(vec2, uint, uint);
vec2 calc_tex_coords(uint, uint);
vec2 get_tex_offset(uint, uint);

// Input
layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(push_constant) uniform PushConstants {
    vec2 tex_size;
    vec2 atlas_size;
    vec2 tile_size;
    vec2 map_size;
    vec2 vertex_offset;
    vec2 depth;
    uint palette_offset;
    uint palette_size;
    float tex_pixel_height;
} push_constants;

// Output
layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

void main() {
    // Vertex position offset with scroll / position
    vec2 vertex_position = position + push_constants.vertex_offset;

    // Calculate wraparound.
    uint tex_y = (data >> 17) % 16;
    uint vertex_side = data & VERTEX_RIGHT;
    vertex_position = calc_vertex_wraparound(vertex_position, vertex_side, tex_y);

    uint priority = (data >> 13) & 1;
    gl_Position = vec4(vertex_position, push_constants.depth[priority], 1.0);

    texCoordOut = calc_tex_coords(data, tex_y);

    paletteNumOut = (data >> 10) & 7;
}

// Wraparound vertex if they overflow.
vec2 calc_vertex_wraparound(vec2 vertex_coords, uint side, uint y) {
    vec2 compare = calc_vertex_compare(vertex_coords, side, y);
    vec2 result = vertex_coords;

    if (compare.x < (VIEW_WIDTH - push_constants.map_size.x)) {
        result.x += push_constants.map_size.x;
        if (compare.x < (VIEW_WIDTH - (push_constants.map_size.x * 2.0))) {
            result.x += push_constants.map_size.x;
            if (compare.x < (VIEW_WIDTH - (push_constants.map_size.x * 3.0))) {
                result.x += push_constants.map_size.x;
            }
        }
    }
    if (compare.y < (VIEW_HEIGHT - push_constants.map_size.y)) {
        result.y += push_constants.map_size.y;
        if (compare.y < (VIEW_HEIGHT - (push_constants.map_size.y * 2.0))) {
            result.y += push_constants.map_size.y;
            if (compare.y < (VIEW_HEIGHT - (push_constants.map_size.y * 3.0))) {
                result.y += push_constants.map_size.y;
            }
        }
    }

    return result;
}

// Get top-left position of tile.
vec2 calc_vertex_compare(vec2 vertex_coords, uint side, uint y) {
    float y_offset = float(y) * push_constants.tile_size.y;  // Y = 0-15
    return vertex_coords - (side == VERTEX_LEFT ? vec2(0.0, y_offset) : vec2(push_constants.tile_size.x, y_offset));
}

// Get texture coordinates from tex number and x and y pos of tile.
vec2 calc_tex_coords(uint tex_data, uint tex_y) {
// Unpack texture information
    uint tex_num = tex_data & 0x3FF;
    uint tex_side = tex_data & TEX_RIGHT;
// Convert to 2D coords
    float x = float(tex_num % TEX_ROW_SIZE) / push_constants.atlas_size.x;
    float y = float(tex_num / TEX_ROW_SIZE) / push_constants.atlas_size.y;
    
    return vec2(x, y) + get_tex_offset(tex_side, tex_y);
}

// Get texture position based on vertex position.
vec2 get_tex_offset(uint side, uint y) {
    float y_offset = (float(y) / push_constants.tex_pixel_height) * push_constants.tex_size.y;
    return side == TEX_LEFT ? vec2(0.0, y_offset) : vec2(push_constants.tex_size.x, y_offset);
}