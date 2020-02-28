// Stores the cached image of a single background.

use bitflags::bitflags;

use crate::video::{
    VideoMem,
    ram::{
        MapMirror,
        BGReg
    }
};

use super::patternmem::PatternMem;

// Sub-map size (32x32 tiles)
const SUB_MAP_LEN: usize = 32;
const SUB_MAP_SIZE: usize = SUB_MAP_LEN * SUB_MAP_LEN * 2;

// Tile sizes TODO: alter based on mode
const SMALL_TILE: usize = 8;
const LARGE_TILE: usize = 16;

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

pub struct BGCache {
    texels:         Vec<Vec<u8>>,
    attrs:          Vec<Vec<TileAttributes>>,

    bg_reg:         BGReg,      // Address and size as stored in register.
    large_tiles:    bool,
    start_addr:     u16,

    size_x:         usize,
    size_y:         usize,

    // dirty?
}

impl BGCache {
    // TODO: avoid complete recreation if these values change
    pub fn new(bg_reg: BGReg, large_tiles: bool) -> Self {
        let tile_size = if large_tiles {LARGE_TILE} else {SMALL_TILE};
        let size_x = if bg_reg.contains(BGReg::MIRROR_X) {SUB_MAP_LEN * 2} else {SUB_MAP_LEN} * tile_size;
        let size_y = if bg_reg.contains(BGReg::MIRROR_Y) {SUB_MAP_LEN * 2} else {SUB_MAP_LEN} * tile_size;

        BGCache {
            texels:         vec![vec![0; size_x]; size_y],
            attrs:          vec![vec![TileAttributes::default(); size_x]; size_y],

            bg_reg:         bg_reg,
            large_tiles:    large_tiles,
            start_addr:     ((bg_reg & BGReg::ADDR).bits() as u16) << 9,    // TODO: do this elsewhere

            size_x:         size_x,
            size_y:         size_y,
        }
    }

    // Returns true if the settings are still valid for this background.
    pub fn check_if_valid(&self, settings: BGReg, large_tiles: bool) -> bool {
        ((settings != self.bg_reg) || (large_tiles != self.large_tiles))
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.size_x
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.size_y
    }

    pub fn construct(&mut self, tiles: &PatternMem, mem: &VideoMem, tiles_changed: bool) {
        use MapMirror::*;
        // First A:
        if tiles_changed || mem.vram_is_dirty(self.start_addr) {
            self.construct_submap(tiles, mem, 0, 0, 0);
        }
        match self.map_mirror() {
            None => {},
            X => {
                // B
                if tiles_changed || mem.vram_is_dirty(self.start_addr + SUB_MAP_SIZE as u16) {
                    self.construct_submap(tiles, mem, SUB_MAP_SIZE, SUB_MAP_LEN, 0);
                }
            },
            Y => {
                // B
                if tiles_changed || mem.vram_is_dirty(self.start_addr + SUB_MAP_SIZE as u16) {
                    self.construct_submap(tiles, mem, SUB_MAP_SIZE, 0, SUB_MAP_LEN);
                }
            },
            Both => {
                if tiles_changed || mem.vram_is_dirty(self.start_addr + SUB_MAP_SIZE as u16) {
                    self.construct_submap(tiles, mem, SUB_MAP_SIZE, SUB_MAP_LEN, 0);  // B
                }
                if tiles_changed || mem.vram_is_dirty(self.start_addr + (SUB_MAP_SIZE * 2) as u16) {
                    self.construct_submap(tiles, mem, SUB_MAP_SIZE * 2, 0, SUB_MAP_LEN);  // C
                }
                if tiles_changed || mem.vram_is_dirty(self.start_addr + (SUB_MAP_SIZE * 3) as u16) {
                    self.construct_submap(tiles, mem, SUB_MAP_SIZE * 3, SUB_MAP_LEN, SUB_MAP_LEN);    // D
                }
            }
        }
    }

    #[inline]
    pub fn get_texel(&self, x: usize, y: usize) -> u8 {
        self.texels[y][x]
    }

    #[inline]
    pub fn get_attrs(&self, x: usize, y: usize) -> TileAttributes {
        self.attrs[y][x]
    }
}

// Private
impl BGCache {
    // Store data for a single sub-map.
    fn construct_submap(&mut self, tiles: &PatternMem, mem: &VideoMem, start_offset: usize, x_offset: usize, y_offset: usize) {
        let start_addr = (self.start_addr as usize) + start_offset;
        let tile_size = if self.large_tiles {LARGE_TILE} else {SMALL_TILE};

        for (i, data) in mem.get_vram().chunks(2).skip(start_addr / 2).take(SUB_MAP_SIZE / 2).enumerate() {
            let attr_flags = TileAttributes::from_bits_truncate(data[1]);
            let tile_num = make16!((attr_flags & TileAttributes::TILE_NUM).bits(), data[0]) as usize;
            
            let base_y = ((i / SUB_MAP_LEN) * tile_size) + y_offset;
            let base_x = ((i % SUB_MAP_LEN) * tile_size) + x_offset;

            for (y, (texel_row, attrs_row)) in self.texels.iter_mut().zip(self.attrs.iter_mut()).skip(base_y).take(tile_size).enumerate() {
                for (x, (texel, attrs)) in texel_row.iter_mut().zip(attrs_row.iter_mut()).skip(base_x).take(tile_size).enumerate() {
                    let (tex_x, tile_idx_x) = {
                        let tex_x = if attr_flags.contains(TileAttributes::X_FLIP) {tile_size - 1 - x} else {x};
                        if tex_x >= SMALL_TILE {
                            (tex_x - SMALL_TILE, tile_num + 1)
                        } else {
                            (tex_x, tile_num)
                        }
                    };
                    let (tex_y, tile_idx) = {
                        let tex_y = if attr_flags.contains(TileAttributes::Y_FLIP) {tile_size - 1 - y} else {y};
                        if tex_y >= SMALL_TILE {
                            (tex_y - SMALL_TILE, tile_idx_x + 16)
                        } else {
                            (tex_y, tile_idx_x)
                        }
                    };
                    *texel = tiles.ref_tile(tile_idx).get_texel(tex_x, tex_y);
                    *attrs = attr_flags;
                }
            }
        }
    }

    // Tile map mirror.
    fn map_mirror(&self) -> MapMirror {
        MapMirror::from(self.bg_reg)
    }
}