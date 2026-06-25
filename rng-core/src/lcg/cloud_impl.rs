use super::Lcg;
use std::cmp::max;
use std::cmp::min;
use std::collections::HashSet;

static RANGES: [[i32; 4]; 4] = [
    [0, 5, -5, 5],
    [-5, 0, -5, 5],
    [-5, 5, -5, 0],
    [-5, 5, 0, 5],
];

pub trait CloudTileSource {
    fn contains_tile(&self, tile: &[i32; 2]) -> bool;
}

impl CloudTileSource for [[i32; 2]] {
    fn contains_tile(&self, tile: &[i32; 2]) -> bool {
        self.contains(tile)
    }
}

pub struct CloudTileLookup {
    tiles: HashSet<[i32; 2]>,
}

impl CloudTileLookup {
    pub fn new(map_tile: &[[i32; 2]]) -> Self {
        Self {
            tiles: map_tile.iter().copied().collect(),
        }
    }
}

impl CloudTileSource for CloudTileLookup {
    fn contains_tile(&self, tile: &[i32; 2]) -> bool {
        self.tiles.contains(tile)
    }
}

impl Lcg {
    pub fn can_get_cloud(&mut self) -> bool {
        self.next() < 0x1999_9999_9999_9999
    }

    pub fn get_cloud<T: CloudTileSource + ?Sized>(
        &mut self,
        trainer_tile: &[i32; 2],
        map_tile: &T,
    ) -> [i32; 2] {
        if self.can_get_cloud() == false {
            return [0, 0];
        }

        let range = RANGES[(self.rand(4)) as usize];

        let trainer_chunk_x = trainer_tile[0] - (trainer_tile[0] % 32);
        let trainer_chunk_y = trainer_tile[1] - (trainer_tile[1] % 32);
        let trainer_offset_x = trainer_tile[0] % 32;
        let trainer_offset_y = trainer_tile[1] % 32;

        let x_min = max(trainer_offset_x + range[0], 0) + trainer_chunk_x;
        let x_max = min(trainer_offset_x + range[1], 31) + trainer_chunk_x;
        let y_min = max(trainer_offset_y + range[2], 0) + trainer_chunk_y;
        let y_max = min(trainer_offset_y + range[3], 31) + trainer_chunk_y;

        let mut valid_tile_count = 0u32;

        for tile_y in y_min..=y_max {
            for tile_x in x_min..=x_max {
                let tile = [tile_x, tile_y];
                if tile != *trainer_tile && map_tile.contains_tile(&tile) {
                    valid_tile_count += 1;
                }
            }
        }

        if valid_tile_count == 0 {
            return [0, 0];
        }

        let target_index = self.rand(valid_tile_count as u64);
        let mut current_index = 0u32;

        for tile_y in y_min..=y_max {
            for tile_x in x_min..=x_max {
                let tile = [tile_x, tile_y];
                if tile != *trainer_tile && map_tile.contains_tile(&tile) {
                    if current_index == target_index {
                        return tile;
                    }
                    current_index += 1;
                }
            }
        }

        return [0, 0];
    }
}
