pub(crate) fn u16_to_hex(value: u16) -> [u8; 4] {
    let mut buf = [0u8; 4];
    faster_hex::hex_encode(&value.to_be_bytes(), &mut buf).expect("two bytes to 4 hex chars never fails");
    buf
}
