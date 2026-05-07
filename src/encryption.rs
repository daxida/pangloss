use ripemd::{Digest, Ripemd128};

// Apparently I already have adler2 as a dep, we could skip rewriting it
pub fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
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
