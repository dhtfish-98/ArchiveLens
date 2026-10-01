use crate::lens_reader::LensReader;
use crate::lens_segment::LensSegment;

#[derive(Clone, PartialEq, Eq)]
pub struct LensBind {
    pub lens_address: u64,
    pub lens_symbol: String,
}

const LENS_BIND_OPCODE_MASK: u8 = 0xF0;
const LENS_BIND_IMMEDIATE_MASK: u8 = 0x0F;
const LENS_BIND_OPCODE_DONE: u8 = 0x00;
const LENS_BIND_OPCODE_SET_DYLIB_ORDINAL_IMM: u8 = 0x10;
const LENS_BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB: u8 = 0x20;
const LENS_BIND_OPCODE_SET_DYLIB_SPECIAL_IMM: u8 = 0x30;
const LENS_BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM: u8 = 0x40;
const LENS_BIND_OPCODE_SET_TYPE_IMM: u8 = 0x50;
const LENS_BIND_OPCODE_SET_ADDEND_SLEB: u8 = 0x60;
const LENS_BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB: u8 = 0x70;
const LENS_BIND_OPCODE_ADD_ADDR_ULEB: u8 = 0x80;
const LENS_BIND_OPCODE_DO_BIND: u8 = 0x90;
const LENS_BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB: u8 = 0xA0;
const LENS_BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED: u8 = 0xB0;
const LENS_BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB: u8 = 0xC0;

const LENS_PTR: u64 = 8;
const LENS_MAX_BINDS: usize = 4_000_000;

pub fn lens_parse_dyld_info_binds(lens_file: &[u8], lens_body: &[u8], lens_segments: &[LensSegment]) -> Vec<LensBind> {
    let mut lens_out = Vec::new();
    let mut lens_r = LensReader::lens_new(lens_body);
    let mut lens_u32s = [0u32; 10];
    for lens_slot in lens_u32s.iter_mut() {
        match lens_r.lens_read_u32() {
            Ok(lens_v) => *lens_slot = lens_v,
            Err(_) => return lens_out,
        }
    }
    let lens_streams = [(lens_u32s[2], lens_u32s[3]), (lens_u32s[4], lens_u32s[5]), (lens_u32s[6], lens_u32s[7])];
    for (lens_off, lens_size) in lens_streams {
        let lens_off = lens_off as usize;
        let lens_size = lens_size as usize;
        if lens_size == 0 {
            continue;
        }
        let lens_end = match lens_off.checked_add(lens_size) {
            Some(lens_e) if lens_e <= lens_file.len() => lens_e,
            _ => continue,
        };
        lens_run_bind_stream(&lens_file[lens_off..lens_end], lens_segments, &mut lens_out);
    }
    lens_out
}

fn lens_seg_addr(lens_segments: &[LensSegment], lens_idx: usize, lens_offset: u64) -> Option<u64> {
    lens_segments.get(lens_idx).map(|lens_s| lens_s.lens_vmaddr.wrapping_add(lens_offset))
}

fn lens_read_symbol(lens_r: &mut LensReader) -> String {
    let mut lens_bytes = Vec::new();
    while let Ok(lens_b) = lens_r.lens_read_u8() {
        if lens_b == 0 {
            break;
        }
        lens_bytes.push(lens_b);
    }
    String::from_utf8_lossy(&lens_bytes).into_owned()
}

fn lens_run_bind_stream(lens_s: &[u8], lens_segments: &[LensSegment], lens_out: &mut Vec<LensBind>) {
    let mut lens_r = LensReader::lens_new(lens_s);
    let mut lens_seg_index = 0usize;
    let mut lens_seg_offset = 0u64;
    let mut lens_symbol = String::new();
    let lens_cap = lens_s.len() as u64 + 1;

    let lens_record = |lens_seg_index: usize, lens_seg_offset: u64, lens_symbol: &str, lens_out: &mut Vec<LensBind>| {
        if !lens_symbol.is_empty() {
            if let Some(lens_a) = lens_seg_addr(lens_segments, lens_seg_index, lens_seg_offset) {
                lens_out.push(LensBind {
                    lens_address: lens_a,
                    lens_symbol: lens_symbol.to_string(),
                });
            }
        }
    };

    while let Ok(lens_byte) = lens_r.lens_read_u8() {
        if lens_out.len() >= LENS_MAX_BINDS {
            break;
        }
        let lens_opcode = lens_byte & LENS_BIND_OPCODE_MASK;
        let lens_imm = lens_byte & LENS_BIND_IMMEDIATE_MASK;
        match lens_opcode {
            LENS_BIND_OPCODE_DONE => {}
            LENS_BIND_OPCODE_SET_DYLIB_ORDINAL_IMM => {}
            LENS_BIND_OPCODE_SET_DYLIB_ORDINAL_ULEB => {
                let _ = lens_r.lens_read_uleb128();
            }
            LENS_BIND_OPCODE_SET_DYLIB_SPECIAL_IMM => {}
            LENS_BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM => {
                lens_symbol = lens_read_symbol(&mut lens_r);
            }
            LENS_BIND_OPCODE_SET_TYPE_IMM => {}
            LENS_BIND_OPCODE_SET_ADDEND_SLEB => {
                let _ = lens_r.lens_read_uleb128();
            }
            LENS_BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB => {
                lens_seg_index = lens_imm as usize;
                lens_seg_offset = lens_r.lens_read_uleb128().unwrap_or(0);
            }
            LENS_BIND_OPCODE_ADD_ADDR_ULEB => {
                lens_seg_offset = lens_seg_offset.wrapping_add(lens_r.lens_read_uleb128().unwrap_or(0));
            }
            LENS_BIND_OPCODE_DO_BIND => {
                lens_record(lens_seg_index, lens_seg_offset, &lens_symbol, lens_out);
                lens_seg_offset = lens_seg_offset.wrapping_add(LENS_PTR);
            }
            LENS_BIND_OPCODE_DO_BIND_ADD_ADDR_ULEB => {
                lens_record(lens_seg_index, lens_seg_offset, &lens_symbol, lens_out);
                let lens_ext = lens_r.lens_read_uleb128().unwrap_or(0);
                lens_seg_offset = lens_seg_offset.wrapping_add(LENS_PTR).wrapping_add(lens_ext);
            }
            LENS_BIND_OPCODE_DO_BIND_ADD_ADDR_IMM_SCALED => {
                lens_record(lens_seg_index, lens_seg_offset, &lens_symbol, lens_out);
                lens_seg_offset = lens_seg_offset
                    .wrapping_add(LENS_PTR)
                    .wrapping_add((lens_imm as u64).wrapping_mul(LENS_PTR));
            }
            LENS_BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB => {
                let lens_count = lens_r.lens_read_uleb128().unwrap_or(0).min(lens_cap);
                let lens_skip = lens_r.lens_read_uleb128().unwrap_or(0);
                for _ in 0..lens_count {
                    if lens_out.len() >= LENS_MAX_BINDS {
                        break;
                    }
                    lens_record(lens_seg_index, lens_seg_offset, &lens_symbol, lens_out);
                    lens_seg_offset = lens_seg_offset.wrapping_add(LENS_PTR).wrapping_add(lens_skip);
                }
            }
            _ => break,
        }
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_seg(lens_vmaddr: u64) -> LensSegment {
        LensSegment {
            lens_segname: "__DATA".to_string(),
            lens_vmaddr: lens_vmaddr,
            lens_vmsize: 0x1000,
            lens_fileoff: 0,
            lens_filesize: 0x1000,
            lens_sections: Vec::new(),
        }
    }

    fn lens_build(lens_stream: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let lens_body_len = 40usize;
        let lens_bind_off = lens_body_len;
        let mut lens_body = Vec::new();
        lens_body.extend_from_slice(&0u32.to_le_bytes());
        lens_body.extend_from_slice(&0u32.to_le_bytes());
        lens_body.extend_from_slice(&(lens_bind_off as u32).to_le_bytes());
        lens_body.extend_from_slice(&(lens_stream.len() as u32).to_le_bytes());
        for _ in 0..6 {
            lens_body.extend_from_slice(&0u32.to_le_bytes());
        }
        let mut lens_file = lens_body.clone();
        lens_file.extend_from_slice(lens_stream);
        (lens_file, lens_body)
    }

    #[test]
    fn lens_resolves_a_do_bind() {
        let mut lens_s = Vec::new();
        lens_s.push(LENS_BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB);
        lens_s.push(0x10);
        lens_s.push(LENS_BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM);
        lens_s.extend_from_slice(b"_OBJC_CLASS_$_NSObject\0");
        lens_s.push(LENS_BIND_OPCODE_DO_BIND);
        let (lens_file, lens_body) = lens_build(&lens_s);
        let lens_binds = lens_parse_dyld_info_binds(&lens_file, &lens_body, &[lens_seg(0x4000)]);
        assert_eq!(lens_binds.len(), 1);
        assert_eq!(lens_binds[0].lens_address, 0x4010);
        assert_eq!(lens_binds[0].lens_symbol, "_OBJC_CLASS_$_NSObject");
    }

    #[test]
    fn lens_huge_times_skipping_is_capped() {
        let mut lens_s = Vec::new();
        lens_s.push(LENS_BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB);
        lens_s.push(0x00);
        lens_s.push(LENS_BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM);
        lens_s.extend_from_slice(b"_sym\0");
        lens_s.push(LENS_BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB);
        lens_s.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x0f]);
        lens_s.push(0x00);
        let (lens_file, lens_body) = lens_build(&lens_s);
        let lens_binds = lens_parse_dyld_info_binds(&lens_file, &lens_body, &[lens_seg(0x4000)]);
        assert!(lens_binds.len() <= lens_s.len() + 1);
    }

    #[test]
    fn lens_stacked_huge_times_skipping_stay_bounded() {
        let mut lens_s = Vec::new();
        lens_s.push(LENS_BIND_OPCODE_SET_SEGMENT_AND_OFFSET_ULEB);
        lens_s.push(0x00);
        lens_s.push(LENS_BIND_OPCODE_SET_SYMBOL_TRAILING_FLAGS_IMM);
        lens_s.extend_from_slice(b"_s\0");
        for _ in 0..50 {
            lens_s.push(LENS_BIND_OPCODE_DO_BIND_ULEB_TIMES_SKIPPING_ULEB);
            lens_s.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x0f]);
            lens_s.push(0x00);
        }
        let (lens_file, lens_body) = lens_build(&lens_s);
        let lens_binds = lens_parse_dyld_info_binds(&lens_file, &lens_body, &[lens_seg(0x4000)]);
        assert!(lens_binds.len() <= LENS_MAX_BINDS);
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensBind { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("Bind");lens_debug.field("address", &self.lens_address);lens_debug.field("symbol", &self.lens_symbol);lens_debug.finish() } }
