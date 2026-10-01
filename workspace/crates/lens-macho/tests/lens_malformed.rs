use lens_macho::LensMachOImage;

#[test]
fn lens_never_panics_on_truncation() {
    use lens_macho::lens_consts::*;
    let mut lens_base = Vec::new();
    lens_base.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
    lens_base.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
    lens_base.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
    lens_base.extend_from_slice(&2u32.to_le_bytes());
    lens_base.extend_from_slice(&50u32.to_le_bytes());
    lens_base.extend_from_slice(&0x1000u32.to_le_bytes());
    lens_base.extend_from_slice(&0u32.to_le_bytes());
    lens_base.extend_from_slice(&0u32.to_le_bytes());

    for lens_len in 0..lens_base.len() {
        let _ = LensMachOImage::lens_parse(&lens_base[..lens_len]);
    }
    for lens_seed in 0u8..64 {
        let lens_junk: Vec<u8> = (0..256)
            .map(|lens_i| (lens_i as u8).wrapping_mul(lens_seed).wrapping_add(7))
            .collect();
        let _ = LensMachOImage::lens_parse(&lens_junk);
    }
}
