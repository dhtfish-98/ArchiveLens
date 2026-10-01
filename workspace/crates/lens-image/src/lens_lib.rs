use lens_macho::lens_fat::lens_select_arm64_slice;
use lens_macho::LensMachOImage;

pub struct LensFoundString {
    pub lens_addr: u64,
    pub lens_value: String,
}

pub struct LensImage {
    pub lens_macho: LensMachOImage,
    pub lens_strings: Vec<LensFoundString>,
}

pub fn lens_extract_string_section(
    lens_sdata: &[u8],
    lens_sect: &lens_macho::lens_segment::LensSection,
) -> Vec<LensFoundString> {
    let mut lens_out = Vec::new();
    let lens_start = lens_sect.lens_offset as usize;
    let lens_size = lens_sect.lens_size as usize;
    let lens_end = match lens_start.checked_add(lens_size) {
        Some(lens_e) if lens_e <= lens_sdata.len() => lens_e,
        _ => return lens_out,
    };
    let lens_data = &lens_sdata[lens_start..lens_end];
    let mut lens_pos = 0usize;
    while lens_pos < lens_data.len() {
        let lens_rest = &lens_data[lens_pos..];
        let lens_term = lens_rest.iter().position(|&lens_c| lens_c == 0).unwrap_or(lens_rest.len());
        if lens_term > 0 {
            match lens_sect.lens_addr.checked_add(lens_pos as u64) {
                Some(lens_addr) => lens_out.push(LensFoundString {
                    lens_addr: lens_addr,
                    lens_value: String::from_utf8_lossy(&lens_rest[..lens_term]).into_owned(),
                }),
                None => break,
            }
        }
        lens_pos += lens_term + 1;
    }
    lens_out
}

impl LensImage {
    pub fn lens_load(lens_buf: &[u8]) -> lens_macho::LensResult<LensImage> {
        let lens_macho = LensMachOImage::lens_parse(lens_buf)?;
        let lens_slice = lens_select_arm64_slice(lens_buf)?;
        let lens_sdata = lens_slice.lens_data;
        let mut lens_strings = Vec::new();

        for lens_seg in &lens_macho.lens_segments {
            for lens_sect in &lens_seg.lens_sections {
                if lens_sect.lens_sectname == "__cstring" {
                    lens_strings.extend(lens_extract_string_section(lens_sdata, lens_sect));
                }
            }
        }

        Ok(LensImage { lens_macho: lens_macho, lens_strings: lens_strings })
    }

    pub fn lens_symbol_at(&self, lens_addr: u64) -> Option<&str> {
        self.lens_macho
            .lens_symbols
            .iter()
            .find(|lens_s| lens_s.lens_value == lens_addr && !lens_s.lens_name.is_empty())
            .map(|lens_s| lens_s.lens_name.as_str())
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;
    use lens_macho::lens_consts::*;

    fn lens_build_with_cstring() -> Vec<u8> {
        lens_build_with_cstring_at(0x2000)
    }

    fn lens_build_with_cstring_at(lens_section_addr: u64) -> Vec<u8> {
        lens_build_cstring_macho(lens_section_addr, None)
    }

    fn lens_build_cstring_macho(lens_section_addr: u64, lens_declared_size: Option<u64>) -> Vec<u8> {
        let lens_strings = b"hi\0bye\0";
        let lens_sect_size = lens_declared_size.unwrap_or(lens_strings.len() as u64);
        let mut lens_sect = Vec::new();
        let mut lens_sn = b"__cstring".to_vec();
        lens_sn.resize(16, 0);
        let mut lens_sg = b"__TEXT".to_vec();
        lens_sg.resize(16, 0);
        lens_sect.extend_from_slice(&lens_sn);
        lens_sect.extend_from_slice(&lens_sg);
        lens_sect.extend_from_slice(&lens_section_addr.to_le_bytes());
        lens_sect.extend_from_slice(&lens_sect_size.to_le_bytes());
        let lens_offset_pos = lens_sect.len();
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());
        lens_sect.extend_from_slice(&0u32.to_le_bytes());

        let mut lens_seg = Vec::new();
        let mut lens_segn = b"__TEXT".to_vec();
        lens_segn.resize(16, 0);
        lens_seg.extend_from_slice(&lens_segn);
        lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
        lens_seg.extend_from_slice(&0u64.to_le_bytes());
        lens_seg.extend_from_slice(&0x4000u64.to_le_bytes());
        lens_seg.extend_from_slice(&5u32.to_le_bytes());
        lens_seg.extend_from_slice(&5u32.to_le_bytes());
        lens_seg.extend_from_slice(&1u32.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect);

        let lens_cmdsize = 8 + lens_seg.len();
        let lens_header_and_lc = 32 + lens_cmdsize;
        let lens_string_offset = lens_header_and_lc;
        let lens_seg_prefix = 64usize;
        let lens_abs_offset_pos = lens_seg_prefix + lens_offset_pos;
        lens_seg[lens_abs_offset_pos..lens_abs_offset_pos + 4]
            .copy_from_slice(&(lens_string_offset as u32).to_le_bytes());

        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
        lens_v.extend_from_slice(&2u32.to_le_bytes());
        lens_v.extend_from_slice(&1u32.to_le_bytes());
        lens_v.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&LENS_LC_SEGMENT_64.to_le_bytes());
        lens_v.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
        lens_v.extend_from_slice(&lens_seg);
        lens_v.extend_from_slice(lens_strings);
        lens_v
    }

    #[test]
    fn lens_extract_string_section_reads_nul_separated_with_addrs() {
        use lens_macho::lens_segment::LensSection;
        let mut lens_sdata = vec![0u8; 4];
        lens_sdata.extend_from_slice(b"ab\0cd\0");
        let lens_sect = LensSection {
            lens_sectname: "__objc_methname".to_string(),
            lens_segname: "__TEXT".to_string(),
            lens_addr: 0x5000,
            lens_size: 6,
            lens_offset: 4,
            lens_flags: 0,
        };
        let lens_out = lens_extract_string_section(&lens_sdata, &lens_sect);
        let lens_pairs: Vec<_> = lens_out.iter().map(|lens_s| (lens_s.lens_addr, lens_s.lens_value.as_str())).collect();
        assert_eq!(lens_pairs, vec![(0x5000, "ab"), (0x5003, "cd")]);
    }

    #[test]
    fn lens_extracts_cstrings_with_addresses() {
        let lens_bytes = lens_build_with_cstring();
        let lens_img = LensImage::lens_load(&lens_bytes).unwrap();
        let lens_vals: Vec<_> = lens_img
            .lens_strings
            .iter()
            .map(|lens_s| (lens_s.lens_addr, lens_s.lens_value.as_str()))
            .collect();
        assert!(lens_vals.contains(&(0x2000, "hi")));
        assert!(lens_vals.contains(&(0x2003, "bye")));
    }

    fn lens_wrap_fat(lens_thin: &[u8]) -> Vec<u8> {
        use lens_macho::lens_consts::*;
        let lens_payload_offset: u32 = 4 + 4 + 20;
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_FAT_MAGIC.to_be_bytes());
        lens_v.extend_from_slice(&1u32.to_be_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_be_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_be_bytes());
        lens_v.extend_from_slice(&lens_payload_offset.to_be_bytes());
        lens_v.extend_from_slice(&(lens_thin.len() as u32).to_be_bytes());
        lens_v.extend_from_slice(&0u32.to_be_bytes());
        lens_v.extend_from_slice(lens_thin);
        lens_v
    }

    #[test]
    fn lens_extracts_cstrings_from_fat_binary() {
        let lens_thin = lens_build_with_cstring();
        let lens_fat = lens_wrap_fat(&lens_thin);
        let lens_img = LensImage::lens_load(&lens_fat).unwrap();
        let lens_vals: Vec<_> = lens_img
            .lens_strings
            .iter()
            .map(|lens_s| (lens_s.lens_addr, lens_s.lens_value.as_str()))
            .collect();
        assert!(lens_vals.contains(&(0x2000, "hi")), "got {lens_vals:?}");
        assert!(lens_vals.contains(&(0x2003, "bye")), "got {lens_vals:?}");
    }

    #[test]
    fn lens_adversarial_section_addr_does_not_panic() {
        let lens_bytes = lens_build_with_cstring_at(u64::MAX - 1);
        let lens_img = LensImage::lens_load(&lens_bytes).unwrap();
        let _ = lens_img.lens_strings.len();
    }

    #[test]
    fn lens_malformed_section_size_is_skipped() {
        let lens_bytes = lens_build_cstring_macho(0x2000, Some(0x1_0000));
        let lens_img = LensImage::lens_load(&lens_bytes).unwrap();
        assert!(lens_img.lens_strings.is_empty(), "got {:?}", lens_img.lens_strings.len());
    }
}
