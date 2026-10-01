use crate::lens_reader::LensReader;
use crate::lens_segment::LensSegment;

#[derive(Clone, PartialEq, Eq)]
pub enum LensFixupTarget {
    LensRebase(u64),
    LensBind(String),
}

#[derive(Clone, PartialEq, Eq)]
pub struct LensChainedFixup {
    pub lens_address: u64,
    pub lens_target: LensFixupTarget,
}

const LENS_DYLD_CHAINED_PTR_64: u16 = 2;
const LENS_DYLD_CHAINED_PTR_64_OFFSET: u16 = 6;

pub fn lens_parse_chained_fixups(
    lens_file: &[u8],
    lens_body: &[u8],
    lens_segments: &[LensSegment],
    lens_base: u64,
) -> Vec<LensChainedFixup> {
    let mut lens_out = Vec::new();
    let mut lens_br = LensReader::lens_new(lens_body);
    let lens_dataoff = match lens_br.lens_read_u32() {
        Ok(lens_v) => lens_v as usize,
        Err(_) => return lens_out,
    };
    let lens_datasize = match lens_br.lens_read_u32() {
        Ok(lens_v) => lens_v as usize,
        Err(_) => return lens_out,
    };
    let lens_blob_end = match lens_dataoff.checked_add(lens_datasize) {
        Some(lens_e) if lens_e <= lens_file.len() => lens_e,
        _ => return lens_out,
    };
    let lens_blob = &lens_file[lens_dataoff..lens_blob_end];

    let mut lens_hr = LensReader::lens_new(lens_blob);
    let lens__fixups_version = lens_read_u32(&mut lens_hr);
    let lens_starts_offset = lens_read_u32(&mut lens_hr) as usize;
    let lens_imports_offset = lens_read_u32(&mut lens_hr) as usize;
    let lens_symbols_offset = lens_read_u32(&mut lens_hr) as usize;
    let lens_imports_count = lens_read_u32(&mut lens_hr) as usize;
    let lens__imports_format = lens_read_u32(&mut lens_hr);
    let lens__symbols_format = lens_read_u32(&mut lens_hr);

    let lens_imports = lens_parse_imports(lens_blob, lens_imports_offset, lens_imports_count, lens_symbols_offset);

    let lens_seg_count = match lens_u32_at(lens_blob, lens_starts_offset) {
        Some(lens_v) => lens_v as usize,
        None => return lens_out,
    };
    for lens_seg_idx in 0..lens_seg_count {
        let lens_ent_off = match lens_starts_offset.checked_add(4 + lens_seg_idx * 4) {
            Some(lens_o) => lens_o,
            None => break,
        };
        let lens_seg_info_off = match lens_u32_at(lens_blob, lens_ent_off) {
            Some(0) => continue,
            Some(lens_v) => lens_v as usize,
            None => break,
        };
        let lens_start = match lens_starts_offset.checked_add(lens_seg_info_off) {
            Some(lens_o) => lens_o,
            None => continue,
        };
        lens_walk_segment(
            lens_blob, lens_file, lens_start, lens_seg_idx, lens_segments, lens_base, &lens_imports, &mut lens_out,
        );
    }
    lens_out
}

fn lens_parse_imports(
    lens_blob: &[u8],
    lens_imports_offset: usize,
    lens_imports_count: usize,
    lens_symbols_offset: usize,
) -> Vec<String> {
    let lens_cap = (lens_blob.len() / 4).saturating_add(1);
    let lens_count = lens_imports_count.min(lens_cap);
    let mut lens_out = Vec::with_capacity(lens_count.min(4096));
    for lens_i in 0..lens_count {
        let lens_entry = match lens_imports_offset
            .checked_add(lens_i * 4)
            .and_then(|lens_o| lens_u32_at(lens_blob, lens_o))
        {
            Some(lens_v) => lens_v,
            None => break,
        };
        let lens_name_offset = (lens_entry >> 9) as usize;
        let lens_name = lens_symbols_offset
            .checked_add(lens_name_offset)
            .and_then(|lens_o| lens_cstr_at(lens_blob, lens_o))
            .unwrap_or_default();
        lens_out.push(lens_name);
    }
    lens_out
}

#[allow(clippy::too_many_arguments)]
fn lens_walk_segment(
    lens_blob: &[u8],
    lens_file: &[u8],
    lens_start: usize,
    lens_seg_idx: usize,
    lens_segments: &[LensSegment],
    lens_base: u64,
    lens_imports: &[String],
    lens_out: &mut Vec<LensChainedFixup>,
) {
    let lens__size = lens_u32_at(lens_blob, lens_start);
    let lens_page_size = match lens_u16_at(lens_blob, lens_start + 4) {
        Some(lens_v) if lens_v != 0 => lens_v as u64,
        _ => return,
    };
    let lens_pointer_format = match lens_u16_at(lens_blob, lens_start + 6) {
        Some(lens_v @ (LENS_DYLD_CHAINED_PTR_64 | LENS_DYLD_CHAINED_PTR_64_OFFSET)) => lens_v,
        _ => return,
    };
    let lens_segment_offset = match lens_u64_at(lens_blob, lens_start + 8) {
        Some(lens_v) => lens_v,
        None => return,
    };
    let lens_page_count = match lens_u16_at(lens_blob, lens_start + 20) {
        Some(lens_v) => lens_v as usize,
        None => return,
    };
    let lens_seg = match lens_segments.get(lens_seg_idx) {
        Some(lens_s) => lens_s,
        None => return,
    };

    for lens_page in 0..lens_page_count {
        let lens_ps_off = match lens_start.checked_add(22 + lens_page * 2) {
            Some(lens_o) => lens_o,
            None => break,
        };
        let lens_page_start = match lens_u16_at(lens_blob, lens_ps_off) {
            Some(lens_v) => lens_v,
            None => break,
        };
        if lens_page_start == 0xFFFF {
            continue;
        }
        let lens_chain_off = lens_segment_offset
            .wrapping_add((lens_page as u64) * lens_page_size)
            .wrapping_add(lens_page_start as u64);
        lens_walk_chain(lens_file, lens_seg, lens_chain_off, lens_pointer_format, lens_base, lens_imports, lens_out);
    }
}

fn lens_walk_chain(
    lens_file: &[u8],
    lens_seg: &LensSegment,
    lens_first_off: u64,
    lens_pointer_format: u16,
    lens_base: u64,
    lens_imports: &[String],
    lens_out: &mut Vec<LensChainedFixup>,
) {
    let mut lens_off = lens_first_off as usize;
    let lens_max_steps = lens_file.len() / 4 + 1;
    for _ in 0..lens_max_steps {
        let lens_raw = match lens_u64_at(lens_file, lens_off) {
            Some(lens_v) => lens_v,
            None => break,
        };
        let lens_field_vmaddr = lens_seg
            .lens_vmaddr
            .wrapping_add(lens_off as u64)
            .wrapping_sub(lens_seg.lens_fileoff);

        let lens_is_bind = (lens_raw >> 63) & 1 == 1;
        let lens_next = ((lens_raw >> 51) & 0xFFF) as usize;

        if lens_is_bind {
            let lens_ordinal = (lens_raw & 0x00FF_FFFF) as usize;
            if let Some(lens_sym) = lens_imports.get(lens_ordinal) {
                lens_out.push(LensChainedFixup {
                    lens_address: lens_field_vmaddr,
                    lens_target: LensFixupTarget::LensBind(lens_sym.clone()),
                });
            }
        } else {
            let lens_target = lens_raw & 0x0000_000F_FFFF_FFFF;
            let lens_high8 = (lens_raw >> 36) & 0xFF;
            let lens_unpacked = (lens_high8 << 56) | lens_target;
            let lens_vmaddr = match lens_pointer_format {
                LENS_DYLD_CHAINED_PTR_64_OFFSET => lens_base.wrapping_add(lens_unpacked),
                LENS_DYLD_CHAINED_PTR_64 => lens_unpacked,
                _ => lens_base.wrapping_add(lens_unpacked),
            };
            lens_out.push(LensChainedFixup {
                lens_address: lens_field_vmaddr,
                lens_target: LensFixupTarget::LensRebase(lens_vmaddr),
            });
        }

        if lens_next == 0 {
            break;
        }
        lens_off = lens_off.wrapping_add(lens_next * 4);
    }
}

fn lens_read_u32(lens_r: &mut LensReader) -> u32 {
    lens_r.lens_read_u32().unwrap_or(0)
}
fn lens_u16_at(lens_d: &[u8], lens_off: usize) -> Option<u16> {
    LensReader::lens_at(lens_d, lens_off).ok()?.lens_read_u16().ok()
}
fn lens_u32_at(lens_d: &[u8], lens_off: usize) -> Option<u32> {
    LensReader::lens_at(lens_d, lens_off).ok()?.lens_read_u32().ok()
}
fn lens_u64_at(lens_d: &[u8], lens_off: usize) -> Option<u64> {
    LensReader::lens_at(lens_d, lens_off).ok()?.lens_read_u64().ok()
}
fn lens_cstr_at(lens_d: &[u8], lens_off: usize) -> Option<String> {
    let lens_rest = lens_d.get(lens_off..)?;
    let lens_end = lens_rest.iter().position(|&lens_c| lens_c == 0).unwrap_or(lens_rest.len());
    Some(String::from_utf8_lossy(&lens_rest[..lens_end]).into_owned())
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_make_rebase_64offset(lens_target36: u64, lens_next: u64) -> u64 {
        (lens_target36 & 0xF_FFFF_FFFF) | ((lens_next & 0xFFF) << 51)
    }

    #[test]
    fn lens_decodes_a_rebase_offset_pointer() {
        let lens_ptr = lens_make_rebase_64offset(0x1234, 0);
        let mut lens_file = lens_ptr.to_le_bytes().to_vec();
        lens_file.resize(0x100, 0);

        let mut lens_blob = Vec::new();
        let lens_starts_off = 28u32;
        lens_blob.extend_from_slice(&1u32.to_le_bytes());
        lens_blob.extend_from_slice(&lens_starts_off.to_le_bytes());
        lens_blob.extend_from_slice(&0u32.to_le_bytes());
        lens_blob.extend_from_slice(&0u32.to_le_bytes());
        lens_blob.extend_from_slice(&0u32.to_le_bytes());
        lens_blob.extend_from_slice(&1u32.to_le_bytes());
        lens_blob.extend_from_slice(&0u32.to_le_bytes());
        let lens_seg_info_off = 8u32;
        lens_blob.extend_from_slice(&1u32.to_le_bytes());
        lens_blob.extend_from_slice(&lens_seg_info_off.to_le_bytes());
        lens_blob.extend_from_slice(&24u32.to_le_bytes());
        lens_blob.extend_from_slice(&0x4000u16.to_le_bytes());
        lens_blob.extend_from_slice(&LENS_DYLD_CHAINED_PTR_64_OFFSET.to_le_bytes());
        lens_blob.extend_from_slice(&0u64.to_le_bytes());
        lens_blob.extend_from_slice(&0u32.to_le_bytes());
        lens_blob.extend_from_slice(&1u16.to_le_bytes());
        lens_blob.extend_from_slice(&0u16.to_le_bytes());

        let lens_dataoff = lens_file.len();
        lens_file.extend_from_slice(&lens_blob);

        let mut lens_cmd = Vec::new();
        lens_cmd.extend_from_slice(&(lens_dataoff as u32).to_le_bytes());
        lens_cmd.extend_from_slice(&(lens_blob.len() as u32).to_le_bytes());

        let lens_seg = LensSegment {
            lens_segname: "__DATA".to_string(),
            lens_vmaddr: 0x4000,
            lens_vmsize: 0x4000,
            lens_fileoff: 0,
            lens_filesize: 0x100,
            lens_sections: Vec::new(),
        };
        let lens_base = 0x1_0000_0000;
        let lens_fixups = lens_parse_chained_fixups(&lens_file, &lens_cmd, &[lens_seg], lens_base);
        assert_eq!(lens_fixups.len(), 1);
        assert_eq!(lens_fixups[0].lens_address, 0x4000);
        assert_eq!(lens_fixups[0].lens_target, LensFixupTarget::LensRebase(lens_base + 0x1234));
    }

    #[test]
    fn lens_empty_or_malformed_returns_empty() {
        assert!(lens_parse_chained_fixups(&[], &[], &[], 0).is_empty());
        let mut lens_cmd = Vec::new();
        lens_cmd.extend_from_slice(&1000u32.to_le_bytes());
        lens_cmd.extend_from_slice(&1000u32.to_le_bytes());
        assert!(lens_parse_chained_fixups(&[0u8; 16], &lens_cmd, &[], 0).is_empty());
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensFixupTarget { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self {Self::LensRebase(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("Rebase");lens_debug.field(lens_slot0);lens_debug.finish() },Self::LensBind(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("Bind");lens_debug.field(lens_slot0);lens_debug.finish() }} } }
impl std::fmt::Debug for LensChainedFixup { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("ChainedFixup");lens_debug.field("address", &self.lens_address);lens_debug.field("target", &self.lens_target);lens_debug.finish() } }
