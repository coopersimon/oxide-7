#version 450

const float MAX_COLOUR = float(0x1F);

layout(location = 0) in vec2 texCoord;
layout(location = 1) in flat uint paletteNum;

layout(set = 0, binding = 0) uniform usampler2D atlas;
layout(set = 1, binding = 0) uniform Palette {
    uvec4 colours[32];  // 8 colours per vector.
    //mat4 colours[24];
} PaletteTable;

layout(push_constant) uniform PushConstants {
    vec2 tex_size;
    vec2 atlas_size;
    vec2 vertex_offset;
    //uint tex_offset;
    float tex_pixel_height;
    uint palette_offset;
    uint palette_size;
} push_constants;

layout(location = 0) out vec4 outColour;

void main() {
    uint texel = texture(atlas, texCoord).x;

    if (texel == 0) {
        outColour = vec4(0.0);
    } else {
        uint palette = (paletteNum * push_constants.palette_size * 2) + push_constants.palette_offset;

        uint colour = (PaletteTable.colours[palette + (texel / 8)][texel / 2] >> (texel % 2)) & 0x7FFF;
        float red = float(colour & 0x1F) / MAX_COLOUR;
        float green = float((colour >> 5) & 0x1F) / MAX_COLOUR;
        float blue = float((colour >> 10) & 0x1F) / MAX_COLOUR;

        outColour = vec4(red, green, blue, 1.0);
    }
}