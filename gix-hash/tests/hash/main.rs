use gix_hash::ObjectId;

mod change_id;
mod hasher;
mod kind;
mod object_id;
mod oid;
mod prefix;

fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}
