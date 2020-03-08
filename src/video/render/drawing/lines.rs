// Line-based rendering.

use bitflags::bitflags;

use super::Renderer;

use crate::video::{
    BG,
    VideoMem,
    render::patternmem::PatternMem
};

// Sub-map size (32x32 tiles)
const SUB_MAP_LEN: usize = 32;
const SUB_MAP_SIZE: usize = SUB_MAP_LEN * SUB_MAP_LEN * 2;
const SUB_MAP_A_OFFSET: usize = 0;
const SUB_MAP_B_OFFSET: usize = SUB_MAP_SIZE;
const SUB_MAP_C_OFFSET: usize = SUB_MAP_SIZE * 2;
const SUB_MAP_D_OFFSET: usize = SUB_MAP_SIZE * 3;

// Tile sizes
const SMALL_TILE_SHIFT: usize = 3;
const SMALL_TILE_MASK: usize = (bit!(SMALL_TILE_SHIFT) - 1) as usize;
const LARGE_TILE_SHIFT: usize = 4;
const LARGE_TILE_MASK: usize = (bit!(LARGE_TILE_SHIFT) - 1) as usize;

bitflags!{
    #[derive(Default)]
    pub struct TileAttributes: u8 {
        const Y_FLIP    = bit!(7);
        const X_FLIP    = bit!(6);
        const PRIORITY  = bit!(5);
        const PALETTE   = bits![4, 3, 2];
        const TILE_NUM  = bits![1, 0];
    }
}

#[derive(Clone, Copy, Default)]
pub struct BGData {
    pub texel: u8,
    pub attrs: TileAttributes
}

impl Renderer {
    // Get a line (256 texels) of a background.
    pub fn get_row(&self, tiles: &PatternMem, mem: &VideoMem, bg: BG, y: usize, row: &mut [BGData]) {
        let regs = mem.get_bg_registers();
        // The mask determines which parts of the coords are used to index into the tile.
        // The shift determines which parts of the coords are used to look for the tile num in VRAM.
        let (tile_mask, tile_shift) = if regs.bg_large_tiles(bg) {(LARGE_TILE_MASK, LARGE_TILE_SHIFT)} else {(SMALL_TILE_MASK, SMALL_TILE_SHIFT)};  // TODO: wide tiles
        let wide_map = regs.bg_wide_map(bg);

        let start_addr = regs.bg_map_addr(bg) as usize;

        let (mask_x, _) = regs.bg_size_mask(bg);
        let tile_y = y & tile_mask;                         // Y index into tile.
        let map_y = y >> tile_shift;                        // Y index into VRAM.
        let hi_submap = map_y >= 32;                            // Hi submap: B, C or D.
        let submap_y = map_y % 32;                              // Index into submap

        let scroll_x = regs.get_bg_scroll_x(bg);
        
        for (x, data) in row.iter_mut().enumerate() {
            let pix_x = (x + scroll_x) & mask_x;
            let tile_x = pix_x & tile_mask;
            let map_x = pix_x >> tile_shift;
            let right_submap = map_x >= 32;
            let submap_x = map_x % 32;

            // Find memory address of the tile we want.
            // Offset based on the 32x32 tile submap.
            let submap_offset = match (right_submap, hi_submap, wide_map) {
                (false, false, _) => SUB_MAP_A_OFFSET,
                (true, false, _) => SUB_MAP_B_OFFSET,
                (false, true, false) => SUB_MAP_B_OFFSET,
                (false, true, true) => SUB_MAP_C_OFFSET,
                (true, true, _) => SUB_MAP_D_OFFSET,
            };
            let inner_offset = (submap_y * SUB_MAP_LEN * 2) + (submap_x * 2);
            let addr = start_addr + submap_offset + inner_offset;

            let tile_num_lo = mem.get_vram()[addr];
            let tile_attrs = TileAttributes::from_bits_truncate(mem.get_vram()[addr + 1]);

            // Use tile data to find correct texel.
            let mut tile_num = make16!((tile_attrs & TileAttributes::TILE_NUM).bits(), tile_num_lo) as usize;
            let mut real_tile_x = if tile_attrs.contains(TileAttributes::X_FLIP) {tile_mask - tile_x} else {tile_x};
            let mut real_tile_y = if tile_attrs.contains(TileAttributes::Y_FLIP) {tile_mask - tile_y} else {tile_y};

            if real_tile_x >= 8 {
                tile_num += 1;
                real_tile_x -= 8;
            }
            if real_tile_y >= 8 {
                tile_num += 16;
                real_tile_y -= 8;
            }

            let tile = tiles.ref_tile(tile_num as usize);
            *data = BGData{texel: tile.get_texel(real_tile_x, real_tile_y), attrs: tile_attrs};
        }
    }
}