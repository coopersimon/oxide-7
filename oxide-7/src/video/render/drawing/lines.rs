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
const SUB_MAP_ROW_SIZE: usize = SUB_MAP_LEN * 2;
const SUB_MAP_SIZE: usize = SUB_MAP_LEN * SUB_MAP_ROW_SIZE;
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
    pub fn get_row(&self, tiles: &PatternMem, mem: &VideoMem, bg: BG, row: &mut [BGData], y: usize, offset_per_tile: bool) {
        let regs = mem.get_bg_registers();
        // The mask determines which parts of the coords are used to index into the tile.
        // The shift determines which parts of the coords are used to look for the tile num in VRAM.
        let (tile_mask_x, tile_shift_x) = if regs.bg_large_tiles(bg) || regs.use_wide_tiles() {(LARGE_TILE_MASK, LARGE_TILE_SHIFT)} else {(SMALL_TILE_MASK, SMALL_TILE_SHIFT)};
        let (tile_mask_y, tile_shift_y) = if regs.bg_large_tiles(bg) {(LARGE_TILE_MASK, LARGE_TILE_SHIFT)} else {(SMALL_TILE_MASK, SMALL_TILE_SHIFT)};
        let wide_map = regs.bg_wide_map(bg);

        let start_addr = regs.bg_map_addr(bg) as usize;

        for (x, data) in row.iter_mut().enumerate() {
            let (pix_x, pix_y) = self.calc_offsets(mem, offset_per_tile, x, y, bg);

            let tile_y = pix_y & tile_mask_y;   // Y index into tile.
            let map_y = pix_y >> tile_shift_y;  // Y index into VRAM.
            let hi_submap = map_y >= 32;        // Hi submap: B, C or D.
            let submap_y = map_y % 32;          // Index into submap

            let tile_x = pix_x & tile_mask_x;
            let map_x = pix_x >> tile_shift_x;
            let right_submap = map_x >= 32;
            let submap_x = map_x % 32;

            // Find memory address of the tile we want.
            // Offset based on the 32x32 tile submap.
            let submap_offset = match (right_submap, hi_submap, wide_map) {
                (false, false, _)       => SUB_MAP_A_OFFSET,
                (true, false, _)        => SUB_MAP_B_OFFSET,
                (false, true, false)    => SUB_MAP_B_OFFSET,
                (false, true, true)     => SUB_MAP_C_OFFSET,
                (true, true, _)         => SUB_MAP_D_OFFSET,
            };
            let inner_offset = (submap_y * SUB_MAP_LEN * 2) + (submap_x * 2);
            let addr = (start_addr + submap_offset + inner_offset) & 0xFFFF;

            let tile_num_lo = mem.get_vram()[addr];
            let tile_attrs = TileAttributes::from_bits_truncate(mem.get_vram()[(addr + 1)]);

            // Use tile data to find correct texel.
            let mut tile_num = make16!((tile_attrs & TileAttributes::TILE_NUM).bits(), tile_num_lo) as usize;
            let mut real_tile_x = if tile_attrs.contains(TileAttributes::X_FLIP) {tile_mask_x - tile_x} else {tile_x};
            let mut real_tile_y = if tile_attrs.contains(TileAttributes::Y_FLIP) {tile_mask_y - tile_y} else {tile_y};

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

        // DEBUG
        /*for (x, data) in row.iter_mut().enumerate() {
            let tile_num = ((x / 8) % 16) + ((y / 8) * 16);
            let tile = tiles.ref_tile(tile_num);
            *data = BGData{texel: tile.get_texel(x % 8, y % 8), attrs: TileAttributes::default()};
        }*/
    }

    // Find offsets for the pixel specified.
    pub fn calc_offsets(&self, mem: &VideoMem, offset_per_tile: bool, x: usize, y: usize, bg: BG) -> (usize, usize) {
        const H_TILE_MASK: usize = 1024 - 8;

        let regs = mem.get_bg_registers();
        let (mask_x, mask_y) = regs.bg_size_mask(bg);
        let scrolled_x = x + regs.get_bg_scroll_x(bg);

        let (out_x, out_y) = if offset_per_tile && scrolled_x >= 8 {
            let bg3_map_addr = regs.bg_map_addr(BG::_3) as usize;

            let bg3_tile_x = (((x - 8) + regs.get_bg_scroll_x(BG::_3)) & H_TILE_MASK) >> 3;
            let bg3_tile_y = regs.get_bg_scroll_y(BG::_3) >> 3;
            let bg3_tile_data_offset = (bg3_tile_x + (bg3_tile_y * SUB_MAP_LEN)) * 2;
            let x_tile_addr = bg3_map_addr + bg3_tile_data_offset;
            let scroll_x_val = make16!(mem.get_vram()[x_tile_addr + 1], mem.get_vram()[x_tile_addr]);

            let out_x = match bg {
                BG::_1 if test_bit!(scroll_x_val, 13) => {
                    let pix_offset = scrolled_x & 7;
                    pix_offset | ((x + (scroll_x_val as usize)) & H_TILE_MASK) // TODO: mask x and scroll_x_val individually?
                },
                BG::_2 if test_bit!(scroll_x_val, 14) => {
                    let pix_offset = scrolled_x & 7;
                    pix_offset | ((x + (scroll_x_val as usize)) & H_TILE_MASK) // TODO: mask x and scroll_x_val individually?
                },
                _ => scrolled_x
            };

            let y_tile_addr = x_tile_addr + SUB_MAP_ROW_SIZE;
            let scroll_y_val = make16!(mem.get_vram()[y_tile_addr + 1], mem.get_vram()[y_tile_addr]);

            let out_y = match bg {
                BG::_1 if test_bit!(scroll_y_val, 13) => {
                    y + (scroll_y_val as usize)
                },
                BG::_2 if test_bit!(scroll_y_val, 14) => {
                    y + (scroll_y_val as usize)
                },
                _ => y + regs.get_bg_scroll_y(bg)
            };
            (out_x, out_y)
        } else {
            (scrolled_x, y + regs.get_bg_scroll_y(bg))
        };

        (out_x & mask_x, out_y & mask_y)
    }
}