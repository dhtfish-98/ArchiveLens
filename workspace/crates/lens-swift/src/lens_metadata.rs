use lens_macho::lens_fat::lens_select_arm64_slice;
use lens_macho::lens_reader::LensReader;
use lens_macho::LensMachOImage;

#[derive(PartialEq, Eq)]
pub enum LensSwiftKind {
    LensClass,
    LensStruct,
    LensEnum,
    LensOther,
}

pub struct LensSwiftType {
    pub lens_kind: LensSwiftKind,
    pub lens_name: String,
}

const LENS_KIND_MASK: u32 = 0x1f;
const LENS_KIND_MODULE: u32 = 0;
const LENS_KIND_PROTOCOL: u32 = 3;
const LENS_KIND_CLASS: u32 = 16;
const LENS_KIND_STRUCT: u32 = 17;
const LENS_KIND_ENUM: u32 = 18;
const LENS_MAX_PARENT_DEPTH: usize = 24;

fn lens_kind_has_name(lens_kind: u32) -> bool {
    matches!(
        lens_kind,
        LENS_KIND_MODULE | LENS_KIND_PROTOCOL | LENS_KIND_CLASS | LENS_KIND_STRUCT | LENS_KIND_ENUM
    )
}

fn lens_u32_at(lens_d: &[u8], lens_off: usize) -> Option<u32> {
    LensReader::lens_at(lens_d, lens_off).ok()?.lens_read_u32().ok()
}

fn lens_cstr_vm(lens_m: &LensMachOImage, lens_d: &[u8], lens_vm: u64) -> Option<String> {
    let lens_off = lens_m.lens_vmaddr_to_offset(lens_vm)?;
    let lens_rest = lens_d.get(lens_off..)?;
    let lens_end = lens_rest.iter().position(|&lens_c| lens_c == 0).unwrap_or(lens_rest.len());
    Some(String::from_utf8_lossy(&lens_rest[..lens_end]).into_owned())
}

fn lens_rel_indirectable(lens_m: &LensMachOImage, lens_d: &[u8], lens_field_vm: u64) -> Option<u64> {
    let lens_off = lens_m.lens_vmaddr_to_offset(lens_field_vm)?;
    let lens_r = lens_u32_at(lens_d, lens_off)? as i32;
    if lens_r == 0 || (lens_r & 1) == 1 {
        return None;
    }
    Some((lens_field_vm as i64).wrapping_add(lens_r as i64) as u64)
}

fn lens_rel_direct(lens_m: &LensMachOImage, lens_d: &[u8], lens_field_vm: u64) -> Option<u64> {
    let lens_off = lens_m.lens_vmaddr_to_offset(lens_field_vm)?;
    let lens_r = lens_u32_at(lens_d, lens_off)? as i32;
    if lens_r == 0 {
        return None;
    }
    Some((lens_field_vm as i64).wrapping_add(lens_r as i64) as u64)
}

fn lens_descriptor_name(lens_m: &LensMachOImage, lens_d: &[u8], lens_desc_vm: u64, lens_depth: usize) -> Option<String> {
    if lens_depth > LENS_MAX_PARENT_DEPTH {
        return None;
    }
    let lens_flags = lens_u32_at(lens_d, lens_m.lens_vmaddr_to_offset(lens_desc_vm)?)?;
    let lens_parent = lens_rel_indirectable(lens_m, lens_d, lens_desc_vm.wrapping_add(4))
        .and_then(|lens_pv| lens_descriptor_name(lens_m, lens_d, lens_pv, lens_depth + 1));

    if !lens_kind_has_name(lens_flags & LENS_KIND_MASK) {
        return lens_parent;
    }
    let lens_name = lens_rel_direct(lens_m, lens_d, lens_desc_vm.wrapping_add(8)).and_then(|lens_nv| lens_cstr_vm(lens_m, lens_d, lens_nv));
    match (lens_parent, lens_name) {
        (Some(lens_p), Some(lens_n)) => Some(format!("{lens_p}.{lens_n}")),
        (None, Some(lens_n)) => Some(lens_n),
        (Some(lens_p), None) => Some(lens_p),
        (None, None) => None,
    }
}

pub fn lens_parse_swift_types(lens_buf: &[u8]) -> lens_macho::LensResult<Vec<LensSwiftType>> {
    let lens_macho = LensMachOImage::lens_parse(lens_buf)?;
    let lens_slice = lens_select_arm64_slice(lens_buf)?;
    let lens_sdata = lens_slice.lens_data;

    let mut lens_out = Vec::new();
    let lens_sec = match lens_macho.lens_section_by_name("__swift5_types") {
        Some(lens_s) => lens_s,
        None => return Ok(lens_out),
    };
    let lens_base = lens_sec.lens_offset as usize;
    let lens_max_by_buf = lens_sdata.len().saturating_sub(lens_base) / 4;
    let lens_count = ((lens_sec.lens_size / 4) as usize).min(lens_max_by_buf);

    for lens_i in 0..lens_count {
        let lens_entry_vm = lens_sec.lens_addr.wrapping_add((lens_i as u64).wrapping_mul(4));
        let lens_desc_vm = match lens_rel_indirectable(&lens_macho, lens_sdata, lens_entry_vm) {
            Some(lens_v) => lens_v,
            None => continue,
        };
        let lens_desc_off = match lens_macho.lens_vmaddr_to_offset(lens_desc_vm) {
            Some(lens_o) => lens_o,
            None => continue,
        };
        let lens_flags = match lens_u32_at(lens_sdata, lens_desc_off) {
            Some(lens_v) => lens_v,
            None => continue,
        };
        let lens_kind = match lens_flags & LENS_KIND_MASK {
            LENS_KIND_CLASS => LensSwiftKind::LensClass,
            LENS_KIND_STRUCT => LensSwiftKind::LensStruct,
            LENS_KIND_ENUM => LensSwiftKind::LensEnum,
            _ => LensSwiftKind::LensOther,
        };
        if let Some(lens_name) = lens_descriptor_name(&lens_macho, lens_sdata, lens_desc_vm, 0) {
            lens_out.push(LensSwiftType { lens_kind: lens_kind, lens_name: lens_name });
        }
    }
    Ok(lens_out)
}

#[cfg(test)]
mod lens_tests {
    use super::*;
    use lens_macho::lens_consts::*;

    fn lens_build_with_one_swift_type() -> Vec<u8> {
        let lens_nsects = 1u32;
        let lens_cmdsize = 8 + 64 + (lens_nsects as usize) * 80;
        let lens_ds = 32 + lens_cmdsize;

        let mut lens_d = vec![0u8; 0x40];
        let lens_put_i32 =
            |lens_d: &mut [u8], lens_at: usize, lens_v: i32| lens_d[lens_at..lens_at + 4].copy_from_slice(&lens_v.to_le_bytes());
        let lens_put_u32 =
            |lens_d: &mut [u8], lens_at: usize, lens_v: u32| lens_d[lens_at..lens_at + 4].copy_from_slice(&lens_v.to_le_bytes());

        lens_put_i32(&mut lens_d, 0x00, 8);
        lens_put_u32(&mut lens_d, 0x08, LENS_KIND_STRUCT);
        lens_put_i32(&mut lens_d, 0x0C, 0x20 - 0x0C);
        lens_put_i32(&mut lens_d, 0x10, 0x18 - 0x10);
        lens_d[0x18..0x1A].copy_from_slice(b"T\0");
        lens_put_u32(&mut lens_d, 0x20, 0);
        lens_put_i32(&mut lens_d, 0x24, 0);
        lens_put_i32(&mut lens_d, 0x28, 0x30 - 0x28);
        lens_d[0x30..0x32].copy_from_slice(b"M\0");

        fn lens_sect(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
            let mut lens_s = Vec::new();
            let mut lens_sn = lens_name.as_bytes().to_vec();
            lens_sn.resize(16, 0);
            let mut lens_sg = b"__TEXT".to_vec();
            lens_sg.resize(16, 0);
            lens_s.extend_from_slice(&lens_sn);
            lens_s.extend_from_slice(&lens_sg);
            lens_s.extend_from_slice(&lens_addr.to_le_bytes());
            lens_s.extend_from_slice(&lens_size.to_le_bytes());
            lens_s.extend_from_slice(&lens_offset.to_le_bytes());
            for _ in 0..7 {
                lens_s.extend_from_slice(&0u32.to_le_bytes());
            }
            lens_s
        }
        let mut lens_seg = Vec::new();
        let mut lens_segn = b"__TEXT".to_vec();
        lens_segn.resize(16, 0);
        lens_seg.extend_from_slice(&lens_segn);
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&(lens_d.len() as u64).to_le_bytes());
        lens_seg.extend_from_slice(&5u32.to_le_bytes());
        lens_seg.extend_from_slice(&5u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect("__swift5_types", lens_ds as u64, 4, lens_ds as u32));

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
        lens_v.extend_from_slice(&lens_d);
        lens_v
    }

    #[test]
    fn lens_parses_one_swift_struct_with_module() {
        let lens_bytes = lens_build_with_one_swift_type();
        let lens_types = lens_parse_swift_types(&lens_bytes).unwrap();
        assert_eq!(lens_types.len(), 1);
        assert_eq!(lens_types[0].lens_kind, LensSwiftKind::LensStruct);
        assert_eq!(lens_types[0].lens_name, "M.T");
    }

    #[test]
    fn lens_empty_when_no_section() {
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
        lens_v.extend_from_slice(&2u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        assert!(lens_parse_swift_types(&lens_v).unwrap().is_empty());
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensSwiftKind { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self {Self::LensClass => { lens_formatter.write_str("Class") },Self::LensStruct => { lens_formatter.write_str("Struct") },Self::LensEnum => { lens_formatter.write_str("Enum") },Self::LensOther => { lens_formatter.write_str("Other") }} } }
