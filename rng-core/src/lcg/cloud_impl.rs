use super::Lcg;
use std::cmp::max;
use std::cmp::min;

static RANGES: [[i32; 4]; 4] = [
    [0, 5, -5, 5],
    [-5, 0, -5, 5],
    [-5, 5, -5, 0],
    [-5, 5, 0, 5],
];

impl Lcg {
    pub fn can_get_cloud(&mut self) -> bool {
        self.next() < 0x1999_9999_9999_9999
    }

    pub fn get_cloud(&mut self, trainer_tile: &[i32; 2], map_tile: &[[i32; 2]]) -> [i32; 2]  {
        if self.can_get_cloud() == false {
            return [0, 0];
        }
        
        let range = RANGES[(self.rand(4)) as usize];

        let x_min_in_chunk: i32 = max((trainer_tile[0] % 32) as i32 + range[0], 0);
        let x_max_in_chunk: i32 = min((trainer_tile[0] % 32) as i32 + range[1], 31);
        let y_min_in_chunk: i32 = max((trainer_tile[1] % 32) as i32 + range[2], 0);
        let y_max_in_chunk: i32 = min((trainer_tile[1] % 32) as i32 + range[3], 31);

        let x_min = x_min_in_chunk + trainer_tile[0] - (trainer_tile[0] % 32);
        let x_max = x_max_in_chunk + trainer_tile[0] - (trainer_tile[0] % 32);
        let y_min = y_min_in_chunk + trainer_tile[1] - (trainer_tile[1] % 32);
        let y_max = y_max_in_chunk + trainer_tile[1] - (trainer_tile[1] % 32);

        let mut valid_tiles: Vec<[i32; 2]> = Vec::new();
        for tile_y in y_min..=y_max  {
            for tile_x in x_min..=x_max  {
                let tile = [tile_x, tile_y];
                if map_tile.contains(&tile) && &tile != trainer_tile {
                    valid_tiles.push(tile);
                }
            }
        }

        return valid_tiles[(self.rand(valid_tiles.len() as u64)) as usize];
    }
}
