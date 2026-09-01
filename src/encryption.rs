use ripemd::{Digest, Ripemd128};

/// The checksum every mdict block carries.
///
/// simd-adler32 rather than a loop of our own: it was already a dependency by
/// way of zip, and it is an order of magnitude quicker over the megabytes a
/// large dictionary checksums.
pub fn adler32(data: &[u8]) -> u32 {
    simd_adler32::adler32(&data)
}

pub fn ripemd128(data: &[u8]) -> [u8; 16] {
    Ripemd128::digest(data).into()
}

pub fn fast_decrypt(data: &mut [u8], key: &[u8]) {
    let mut prev: u8 = 0x36;
    for (i, byte) in data.iter_mut().enumerate() {
        let current = *byte;
        let t = current.rotate_left(4);
        let t = t ^ prev ^ (i as u8) ^ key[i % key.len()];
        prev = current;
        *byte = t;
    }
}
