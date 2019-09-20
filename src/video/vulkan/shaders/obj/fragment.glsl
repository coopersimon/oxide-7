#version 450

const float MAX_COLOUR = float(0x1F);

const uint PALETTE_OFFSET = 128;
const uint PALETTE_SIZE = 16;

layout(location = 0) in vec2 texCoord;
layout(location = 1) in flat uint paletteNum;

layout(set = 0, binding = 0) uniform usampler2D atlas;
layout(set = 1, binding = 0) uniform PaletteTable {
    uvec4 colours[16];  // 8 colours per vector.
} palette_table;

layout(location = 0) out vec4 outColour;

void main() {
    uint texel = texture(atlas, texCoord).x;

    if (texel == 0) {
        outColour = vec4(0.0);
    } else {
        // Colour is 0-255.
        uint colour_offset = (PALETTE_SIZE * paletteNum) + texel;

        uint table_index = colour_offset / 8;
        uint vec_index = (colour_offset / 2) % 4;
        uint shift = (colour_offset % 2) * 16;

        uint colour = (palette_table.colours[table_index][vec_index] >> shift) & 0x7FFF;
        float red = float(colour & 0x1F) / MAX_COLOUR;
        float green = float((colour >> 5) & 0x1F) / MAX_COLOUR;
        float blue = float((colour >> 10) & 0x1F) / MAX_COLOUR;

        outColour = vec4(red, green, blue, 1.0);
    }
    //outColour = vec4(1.0, 0.0, 0.0, 1.0);
}