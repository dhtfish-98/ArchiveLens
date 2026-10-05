use crate::lens_chained_fixups::{lens_parse_chained_fixups, LensChainedFixup};
use crate::lens_consts::*;
use crate::lens_dyld_info::{lens_parse_dyld_info_binds, LensBind};
use crate::lens_fat::lens_select_arm64_slice;
use crate::lens_header::lens_load_commands;
use crate::lens_linkedit::{lens_parse_encryption, lens_parse_function_starts, lens_parse_uuid, LensEncryption};
use crate::lens_reader::LensReader;
use crate::lens_segment::{lens_parse_segment, LensSection, LensSegment};
use crate::lens_symtab::{lens_parse_symtab, LensSymbol};
use crate::LensResult;

#[derive()]
pub struct LensMachOImage {
    pub lens_cputype: u32,
    pub lens_cpusubtype: u32,
    pub lens_filetype: u32,
    pub lens_segments: Vec<LensSegment>,
    pub lens_symbols: Vec<LensSymbol>,
    pub lens_uuid: Option<[u8; 16]>,
    pub lens_encryption: Option<LensEncryption>,
    pub lens_function_starts: Vec<u64>,
    pub lens_has_chained_fixups: bool,
    pub lens_binds: Vec<LensBind>,
    pub lens_chained_fixups: Vec<LensChainedFixup>,
}

impl LensMachOImage {
    pub fn lens_parse(lens_buf: &[u8]) -> LensResult<LensMachOImage> {
        let lens_slice = lens_select_arm64_slice(lens_buf)?;
        let lens_cmds = lens_load_commands(&lens_slice)?;

        let mut lens_segments = Vec::new();
        let mut lens_symbols = Vec::new();
        let mut lens_uuid = None;
        let mut lens_encryption = None;
        let mut lens_has_chained_fixups = false;

        for lens_lc in &lens_cmds {
            match lens_lc.lens_cmd {
                LENS_LC_SEGMENT_64 => {
                    if let Ok(lens_seg) = lens_parse_segment(lens_lc.lens_body) {
                        lens_segments.push(lens_seg);
                    }
                }
                LENS_LC_SYMTAB => {
                    if let Ok(mut lens_s) = lens_parse_symtab(lens_slice.lens_data, lens_lc.lens_body) {
                        lens_symbols.append(&mut lens_s);
                    }
                }
                LENS_LC_UUID => {
                    lens_uuid = lens_parse_uuid(lens_lc.lens_body).ok();
                }
                LENS_LC_ENCRYPTION_INFO_64 => {
                    lens_encryption = Some(lens_parse_encryption(lens_lc.lens_body)?);
                }
                LENS_LC_DYLD_CHAINED_FIXUPS => {
                    lens_has_chained_fixups = true;
                }
                _ => {}
            }
        }

        let lens_text_vmaddr = lens_segments
            .iter()
            .find(|lens_s| lens_s.lens_segname == "__TEXT")
            .map(|lens_s| lens_s.lens_vmaddr)
            .unwrap_or(0);

        let mut lens_function_starts = Vec::new();
        let mut lens_binds = Vec::new();
        let mut lens_chained_fixups = Vec::new();
        for lens_lc in &lens_cmds {
            if lens_lc.lens_cmd == LENS_LC_FUNCTION_STARTS {
                lens_function_starts = lens_parse_function_starts(lens_slice.lens_data, lens_lc.lens_body, lens_text_vmaddr)?;
            }
            if lens_lc.lens_cmd == LENS_LC_DYLD_INFO || lens_lc.lens_cmd == LENS_LC_DYLD_INFO_ONLY {
                lens_binds = lens_parse_dyld_info_binds(lens_slice.lens_data, lens_lc.lens_body, &lens_segments);
            }
            if lens_lc.lens_cmd == LENS_LC_DYLD_CHAINED_FIXUPS {
                lens_chained_fixups = lens_parse_chained_fixups(lens_slice.lens_data, lens_lc.lens_body, &lens_segments, lens_text_vmaddr);
            }
        }

        let lens_header = crate::lens_header::lens_parse_header(&lens_slice)?;
        Ok(LensMachOImage {
            lens_cputype: lens_slice.lens_cputype,
            lens_cpusubtype: lens_slice.lens_cpusubtype,
            lens_filetype: lens_header.lens_filetype,
            lens_segments: lens_segments,
            lens_symbols: lens_symbols,
            lens_uuid: lens_uuid,
            lens_encryption: lens_encryption,
            lens_function_starts: lens_function_starts,
            lens_has_chained_fixups: lens_has_chained_fixups,
            lens_binds: lens_binds,
            lens_chained_fixups: lens_chained_fixups,
        })
    }

    pub fn lens_is_encrypted(&self) -> bool {
        matches!(&self.lens_encryption, Some(e) if e.lens_cryptid != 0)
    }

    pub fn lens_text_vmaddr(&self) -> Option<u64> {
        self.lens_segments
            .iter()
            .find(|lens_s| lens_s.lens_segname == "__TEXT")
            .map(|lens_s| lens_s.lens_vmaddr)
    }

    pub fn lens_section_by_name(&self, lens_name: &str) -> Option<&LensSection> {
        self.lens_segments
            .iter()
            .flat_map(|lens_seg| &lens_seg.lens_sections)
            .find(|lens_sect| lens_sect.lens_sectname == lens_name)
    }

    pub fn lens_vmaddr_to_offset(&self, lens_vmaddr: u64) -> Option<usize> {
        for lens_seg in &self.lens_segments {
            let lens_seg_end = lens_seg.lens_vmaddr.checked_add(lens_seg.lens_vmsize)?;
            if lens_vmaddr >= lens_seg.lens_vmaddr && lens_vmaddr < lens_seg_end {
                let lens_delta = lens_vmaddr - lens_seg.lens_vmaddr;
                if lens_delta < lens_seg.lens_filesize {
                    return usize::try_from(lens_seg.lens_fileoff.checked_add(lens_delta)?).ok();
                }
                return None;
            }
        }
        None
    }

    pub fn lens_read_u64_at(&self, lens_data: &[u8], lens_offset: usize) -> Option<u64> {
        LensReader::lens_at(lens_data, lens_offset).ok()?.lens_read_u64().ok()
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_segment_body() -> Vec<u8> {
        let mut lens_b = Vec::new();
        let mut lens_name = b"__TEXT".to_vec();
        lens_name.resize(16, 0);
        lens_b.extend_from_slice(&lens_name);
        lens_b.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_b.extend_from_slice(&0x4000u64.to_le_bytes());
        lens_b.extend_from_slice(&0u64.to_le_bytes());
        lens_b.extend_from_slice(&0x4000u64.to_le_bytes());
        lens_b.extend_from_slice(&5u32.to_le_bytes());
        lens_b.extend_from_slice(&5u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b.extend_from_slice(&0u32.to_le_bytes());
        lens_b
    }

    fn lens_build(lens_cmds: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut lens_lc = Vec::new();
        for (lens_cmd, lens_body) in lens_cmds {
            let lens_raw = 8 + lens_body.len();
            let lens_cmdsize = (lens_raw + 7) & !7;
            lens_lc.extend_from_slice(&lens_cmd.to_le_bytes());
            lens_lc.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
            lens_lc.extend_from_slice(lens_body);
            lens_lc.resize(lens_lc.len() + (lens_cmdsize - lens_raw), 0);
        }
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64E.to_le_bytes());
        lens_v.extend_from_slice(&2u32.to_le_bytes());
        lens_v.extend_from_slice(&(lens_cmds.len() as u32).to_le_bytes());
        lens_v.extend_from_slice(&(lens_lc.len() as u32).to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&lens_lc);
        lens_v
    }

    #[test]
    fn lens_parses_full_image_with_encryption_flag() {
        let mut lens_enc = Vec::new();
        lens_enc.extend_from_slice(&0x4000u32.to_le_bytes());
        lens_enc.extend_from_slice(&0x1000u32.to_le_bytes());
        lens_enc.extend_from_slice(&1u32.to_le_bytes());
        lens_enc.extend_from_slice(&0u32.to_le_bytes());
        let lens_bytes = lens_build(&[
            (LENS_LC_SEGMENT_64, lens_segment_body()),
            (LENS_LC_UUID, vec![0x11; 16]),
            (LENS_LC_ENCRYPTION_INFO_64, lens_enc),
        ]);
        let lens_img = LensMachOImage::lens_parse(&lens_bytes).unwrap();
        assert_eq!(lens_img.lens_cpusubtype, LENS_CPU_SUBTYPE_ARM64E);
        assert_eq!(lens_img.lens_segments.len(), 1);
        assert_eq!(lens_img.lens_text_vmaddr(), Some(0x1000));
        assert_eq!(lens_img.lens_uuid, Some([0x11; 16]));
        assert!(lens_img.lens_is_encrypted());
    }

    #[test]
    fn lens_unencrypted_when_no_command() {
        let lens_bytes = lens_build(&[(LENS_LC_SEGMENT_64, lens_segment_body())]);
        let lens_img = LensMachOImage::lens_parse(&lens_bytes).unwrap();
        assert!(!lens_img.lens_is_encrypted());
    }

    #[test]
    fn lens_encryption_command_present_but_cryptid_zero_is_not_encrypted() {
        let mut lens_enc = Vec::new();
        lens_enc.extend_from_slice(&0x4000u32.to_le_bytes());
        lens_enc.extend_from_slice(&0x1000u32.to_le_bytes());
        lens_enc.extend_from_slice(&0u32.to_le_bytes());
        lens_enc.extend_from_slice(&0u32.to_le_bytes());
        let lens_bytes = lens_build(&[
            (LENS_LC_SEGMENT_64, lens_segment_body()),
            (LENS_LC_ENCRYPTION_INFO_64, lens_enc),
        ]);
        let lens_img = LensMachOImage::lens_parse(&lens_bytes).unwrap();
        assert!(lens_img.lens_encryption.is_some());
        assert!(!lens_img.lens_is_encrypted());
    }

    #[test]
    fn lens_rejects_truncated_encryption_command() {
        let lens_bytes = lens_build(&[(LENS_LC_ENCRYPTION_INFO_64, Vec::new())]);
        assert!(LensMachOImage::lens_parse(&lens_bytes).is_err());
    }

    #[test]
    fn lens_rejects_function_starts_past_declared_member() {
        let mut lens_body = Vec::new();
        lens_body.extend_from_slice(&48u32.to_le_bytes());
        lens_body.extend_from_slice(&1u32.to_le_bytes());
        let mut lens_bytes = lens_build(&[(LENS_LC_FUNCTION_STARTS, lens_body)]);
        lens_bytes.extend_from_slice(&[0x80, 0x01, 0x00]);
        assert!(LensMachOImage::lens_parse(&lens_bytes).is_err());
    }

    #[test]
    fn lens_vmaddr_to_offset_no_overflow_on_huge_fileoff() {
        let mut lens_body = Vec::new();
        let mut lens_name = b"__DATA".to_vec();
        lens_name.resize(16, 0);
        lens_body.extend_from_slice(&lens_name);
        lens_body.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_body.extend_from_slice(&0x2000u64.to_le_bytes());
        lens_body.extend_from_slice(&u64::MAX.to_le_bytes());
        lens_body.extend_from_slice(&0x2000u64.to_le_bytes());
        lens_body.extend_from_slice(&3u32.to_le_bytes());
        lens_body.extend_from_slice(&3u32.to_le_bytes());
        lens_body.extend_from_slice(&0u32.to_le_bytes());
        lens_body.extend_from_slice(&0u32.to_le_bytes());
        let lens_bytes = lens_build(&[(LENS_LC_SEGMENT_64, lens_body)]);
        let lens_img = LensMachOImage::lens_parse(&lens_bytes).unwrap();
        assert_eq!(lens_img.lens_vmaddr_to_offset(0x1100), None);
    }

    #[test]
    fn lens_vmaddr_to_offset_maps_within_segment_and_rejects_outside() {
        let lens_bytes = lens_build(&[(LENS_LC_SEGMENT_64, lens_segment_body())]);
        let lens_img = LensMachOImage::lens_parse(&lens_bytes).unwrap();
        assert_eq!(lens_img.lens_vmaddr_to_offset(0x1000), Some(0));
        assert_eq!(lens_img.lens_vmaddr_to_offset(0x1234), Some(0x234));
        assert_eq!(lens_img.lens_vmaddr_to_offset(0x0), None);
        assert_eq!(lens_img.lens_vmaddr_to_offset(0x9000), None);
        assert!(lens_img.lens_section_by_name("__nope").is_none());
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensMachOImage { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("MachOImage");lens_debug.field("cputype", &self.lens_cputype);lens_debug.field("cpusubtype", &self.lens_cpusubtype);lens_debug.field("filetype", &self.lens_filetype);lens_debug.field("segments", &self.lens_segments);lens_debug.field("symbols", &self.lens_symbols);lens_debug.field("uuid", &self.lens_uuid);lens_debug.field("encryption", &self.lens_encryption);lens_debug.field("function_starts", &self.lens_function_starts);lens_debug.field("has_chained_fixups", &self.lens_has_chained_fixups);lens_debug.field("binds", &self.lens_binds);lens_debug.field("chained_fixups", &self.lens_chained_fixups);lens_debug.finish() } }
