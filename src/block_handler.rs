//! A selection of utility functions surrounding the scale of the cursor/block

use Config;
use BLOCK_WIDTH;

pub struct BlockHandler {
    pub block_width: usize,
    pub block_height: usize,
    pub default_block_width: usize,
    pub default_block_height: usize,
}

impl BlockHandler {
    pub fn new(block_width: usize, block_height: usize) -> Self {
        BlockHandler {
            block_width,
            block_height,
            default_block_width: block_width,
            default_block_height: block_height,
        }
    }
}

impl BlockHandler {
    pub fn get(&self) -> (usize, usize) {
        (self.block_width, self.block_height)
    }

    pub fn get_block_from_coordinate(&self, x: usize, y: usize) -> (u16, u16) {
        let x = (x / self.block_width) + 1;
        let y = (y / self.block_height) + 1;

        (x as u16, y as u16)
    }

    pub fn get_pixels_from_block(&self, x: usize, y: usize) -> (usize, usize) {
        let x = x * self.block_width;
        let y = y * self.block_height;

        (x, y)
    }

    pub fn how_many_blocks_fit(&self, window_width: usize, window_height: usize) -> (usize, usize) {
        let width = window_width / self.block_width;
        let height = window_height / self.block_height;

        (width, height)
    }

    pub fn increase_block_size(&mut self, size: isize) {
        self.block_width = (self.block_width as isize + size) as usize;
        self.set_block_size(self.block_width);
    }

    pub fn reset_to_default(&mut self) {
        self.block_width = self.default_block_width;
        self.block_height = self.default_block_height;
    }

    pub fn set_block_size(&mut self, block_width: usize) {
        self.block_width = if block_width < 4 {
            4
        } else if block_width > 48 {
            48
        } else {
            block_width
        };
        self.block_height = self.block_width * 2;

        let scale = self.block_width as f32 / BLOCK_WIDTH as f32;
        Config::set_initial_scale(scale).unwrap();
    }
}
