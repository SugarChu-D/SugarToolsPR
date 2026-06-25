use super::charge_stone_tile::*;
use rng_core::lcg::Lcg;

#[cfg(test)]
mod tests {
    use crate::plasma_skip::charge_stone_tile;
    use super::*;

    #[test]
    fn test_cloud_impl() {
        let mut lcg = Lcg::new(0x6b0a03111b06fa29);
        lcg.advance(97);
        let cloud_tile = lcg.get_cloud(&[48, 34], charge_stone_tile::CHARGE_STONE_B1F_VALID_TILES);
        println!("cloud_tile: {:?}", cloud_tile);
        assert_eq!(cloud_tile, [50, 33]);
    }
}
