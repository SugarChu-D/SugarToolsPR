use super::Lcg;

#[derive(Debug, Clone)]
pub struct grotto {
    is_filled: bool,
    sub_slot: u8,
    slot: u8,
}
