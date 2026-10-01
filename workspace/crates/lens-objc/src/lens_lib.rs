pub mod lens_type_encoding;

use lens_image::{lens_extract_string_section, LensFoundString};
use lens_macho::lens_chained_fixups::LensFixupTarget;
use lens_macho::lens_fat::lens_select_arm64_slice;
use lens_macho::lens_reader::LensReader;
use lens_macho::LensMachOImage;
use std::collections::HashMap;

pub struct LensObjcStrings {
    pub lens_selectors: Vec<LensFoundString>,
    pub lens_class_names: Vec<LensFoundString>,
    pub lens_method_types: Vec<LensFoundString>,
}

impl LensObjcStrings {
    fn lens_empty() -> LensObjcStrings {
        LensObjcStrings {
            lens_selectors: Vec::new(),
            lens_class_names: Vec::new(),
            lens_method_types: Vec::new(),
        }
    }
}

pub fn lens_parse_objc_strings(lens_buf: &[u8]) -> lens_macho::LensResult<LensObjcStrings> {
    let lens_macho = LensMachOImage::lens_parse(lens_buf)?;
    let lens_slice = lens_select_arm64_slice(lens_buf)?;
    let lens_sdata = lens_slice.lens_data;
    let mut lens_out = LensObjcStrings::lens_empty();

    for lens_seg in &lens_macho.lens_segments {
        for lens_sect in &lens_seg.lens_sections {
            match lens_sect.lens_sectname.as_str() {
                "__objc_methname" => lens_out.lens_selectors.extend(lens_extract_string_section(lens_sdata, lens_sect)),
                "__objc_classname" => lens_out.lens_class_names.extend(lens_extract_string_section(lens_sdata, lens_sect)),
                "__objc_methtype" => lens_out.lens_method_types.extend(lens_extract_string_section(lens_sdata, lens_sect)),
                _ => {}
            }
        }
    }
    Ok(lens_out)
}

pub struct LensObjcMethod {
    pub lens_name: String,
    pub lens_types: String,
    pub lens_imp: u64,
}

pub struct LensObjcIvar {
    pub lens_name: String,
    pub lens_type_enc: String,
    pub lens_offset: u32,
}

pub struct LensObjcClass {
    pub lens_address: u64,
    pub lens_name: String,
    pub lens_superclass: Option<String>,
    pub lens_instance_methods: Vec<LensObjcMethod>,
    pub lens_class_methods: Vec<LensObjcMethod>,
    pub lens_ivars: Vec<LensObjcIvar>,
    pub lens_protocols: Vec<String>,
}

pub struct LensObjcCategory {
    pub lens_name: String,
    pub lens_class_name: Option<String>,
    pub lens_instance_methods: Vec<LensObjcMethod>,
    pub lens_class_methods: Vec<LensObjcMethod>,
    pub lens_protocols: Vec<String>,
}

const LENS_OBJC_CLASS_SUPER_OFF: usize = 8;
const LENS_OBJC_CLASS_BITS_OFF: usize = 32;
const LENS_FAST_DATA_MASK: u64 = 0x0000_7fff_ffff_fff8;
const LENS_CLASS_RO_NAME_OFF: usize = 24;
const LENS_CLASS_RO_METHODS_OFF: usize = 32;
const LENS_CLASS_RO_PROTOCOLS_OFF: usize = 40;
const LENS_CLASS_RO_IVARS_OFF: usize = 48;
const LENS_PROTOCOL_NAME_OFF: usize = 8;
const LENS_SMALL_METHOD_LIST_FLAG: u32 = 0x8000_0000;
const LENS_CATEGORY_CLS_OFF: usize = 8;
const LENS_CATEGORY_INST_METHODS_OFF: usize = 16;
const LENS_CATEGORY_CLASS_METHODS_OFF: usize = 24;
const LENS_CATEGORY_PROTOCOLS_OFF: usize = 32;

fn lens_read_cstr(lens_data: &[u8], lens_offset: usize) -> Option<String> {
    let lens_rest = lens_data.get(lens_offset..)?;
    let lens_end = lens_rest.iter().position(|&lens_c| lens_c == 0).unwrap_or(lens_rest.len());
    Some(String::from_utf8_lossy(&lens_rest[..lens_end]).into_owned())
}

fn lens_u32_at(lens_data: &[u8], lens_offset: usize) -> Option<u32> {
    LensReader::lens_at(lens_data, lens_offset).ok()?.lens_read_u32().ok()
}

fn lens_u32_vm(lens_m: &LensMachOImage, lens_d: &[u8], lens_vm: u64) -> Option<u32> {
    lens_u32_at(lens_d, lens_m.lens_vmaddr_to_offset(lens_vm)?)
}

fn lens_cstr_vm(lens_m: &LensMachOImage, lens_d: &[u8], lens_vm: u64) -> Option<String> {
    lens_read_cstr(lens_d, lens_m.lens_vmaddr_to_offset(lens_vm)?)
}

fn lens_rel(lens_base_vm: u64, lens_off: i32) -> u64 {
    (lens_base_vm as i64).wrapping_add(lens_off as i64) as u64
}

struct LensResolver {
    lens_chained: HashMap<u64, LensFixupTarget>,
    lens_binds: HashMap<u64, String>,
}

impl LensResolver {
    fn lens_new(lens_m: &LensMachOImage) -> LensResolver {
        let mut lens_chained = HashMap::new();
        let mut lens_binds: HashMap<u64, String> = HashMap::new();
        for lens_b in &lens_m.lens_binds {
            lens_binds.insert(lens_b.lens_address, lens_b.lens_symbol.clone());
        }
        for lens_f in &lens_m.lens_chained_fixups {
            if let LensFixupTarget::LensBind(lens_sym) = &lens_f.lens_target {
                lens_binds.insert(lens_f.lens_address, lens_sym.clone());
            }
            lens_chained.insert(lens_f.lens_address, lens_f.lens_target.clone());
        }
        LensResolver { lens_chained: lens_chained, lens_binds: lens_binds }
    }

    fn lens_follow(&self, lens_m: &LensMachOImage, lens_d: &[u8], lens_field_vm: u64) -> Option<u64> {
        match self.lens_chained.get(&lens_field_vm) {
            Some(LensFixupTarget::LensRebase(lens_t)) => Some(*lens_t),
            Some(LensFixupTarget::LensBind(_)) => None,
            None => match lens_m.lens_read_u64_at(lens_d, lens_m.lens_vmaddr_to_offset(lens_field_vm)?)? {
                0 => None,
                lens_v => Some(lens_v),
            },
        }
    }

    fn lens_bind_class_name(&self, lens_field_vm: u64) -> Option<String> {
        let lens_sym = self.lens_binds.get(&lens_field_vm)?;
        Some(
            lens_sym.strip_prefix("_OBJC_CLASS_$_")
                .unwrap_or(lens_sym)
                .to_string(),
        )
    }
}

fn lens_class_name_at(lens_r: &LensResolver, lens_m: &LensMachOImage, lens_d: &[u8], lens_class_vm: u64) -> Option<String> {
    let lens_ro_vm = lens_r.lens_follow(lens_m, lens_d, lens_class_vm.wrapping_add(LENS_OBJC_CLASS_BITS_OFF as u64))? & LENS_FAST_DATA_MASK;
    let lens_name_vm = lens_r.lens_follow(lens_m, lens_d, lens_ro_vm.wrapping_add(LENS_CLASS_RO_NAME_OFF as u64))?;
    lens_cstr_vm(lens_m, lens_d, lens_name_vm)
}

fn lens_read_method_list(lens_r: &LensResolver, lens_m: &LensMachOImage, lens_d: &[u8], lens_list_vm: u64) -> Vec<LensObjcMethod> {
    let mut lens_out = Vec::new();
    if lens_list_vm == 0 {
        return lens_out;
    }
    let lens_list_off = match lens_m.lens_vmaddr_to_offset(lens_list_vm) {
        Some(lens_o) => lens_o,
        None => return lens_out,
    };
    let lens_eaf = match lens_u32_at(lens_d, lens_list_off) {
        Some(lens_v) => lens_v,
        None => return lens_out,
    };
    let lens_count = match lens_u32_at(lens_d, lens_list_off + 4) {
        Some(lens_v) => lens_v as usize,
        None => return lens_out,
    };
    let lens_is_small = lens_eaf & LENS_SMALL_METHOD_LIST_FLAG != 0;
    let lens_entsize: u64 = if lens_is_small { 12 } else { 24 };
    let lens_avail = lens_d.len().saturating_sub(lens_list_off + 8) as u64;
    let lens_count = lens_count.min((lens_avail / lens_entsize) as usize);

    for lens_i in 0..lens_count {
        let lens_e_vm = lens_list_vm
            .wrapping_add(8)
            .wrapping_add((lens_i as u64).wrapping_mul(lens_entsize));
        if lens_is_small {
            let lens_name = lens_u32_vm(lens_m, lens_d, lens_e_vm).and_then(|lens_off| {
                let lens_selref_vm = lens_rel(lens_e_vm, lens_off as i32);
                let lens_sel_str_vm = lens_r.lens_follow(lens_m, lens_d, lens_selref_vm)?;
                lens_cstr_vm(lens_m, lens_d, lens_sel_str_vm)
            });
            let lens_types = lens_u32_vm(lens_m, lens_d, lens_e_vm.wrapping_add(4))
                .and_then(|lens_off| lens_cstr_vm(lens_m, lens_d, lens_rel(lens_e_vm.wrapping_add(4), lens_off as i32)));
            let lens_imp = lens_u32_vm(lens_m, lens_d, lens_e_vm.wrapping_add(8))
                .map(|lens_off| lens_rel(lens_e_vm.wrapping_add(8), lens_off as i32))
                .unwrap_or(0);
            if let Some(lens_name) = lens_name {
                lens_out.push(LensObjcMethod {
                    lens_name: lens_name,
                    lens_types: lens_types.unwrap_or_default(),
                    lens_imp: lens_imp,
                });
            }
        } else {
            let lens_name = lens_r.lens_follow(lens_m, lens_d, lens_e_vm).and_then(|lens_p| lens_cstr_vm(lens_m, lens_d, lens_p));
            let lens_types = lens_r
                .lens_follow(lens_m, lens_d, lens_e_vm.wrapping_add(8))
                .and_then(|lens_p| lens_cstr_vm(lens_m, lens_d, lens_p));
            let lens_imp = lens_r.lens_follow(lens_m, lens_d, lens_e_vm.wrapping_add(16)).unwrap_or(0);
            if let Some(lens_name) = lens_name {
                lens_out.push(LensObjcMethod {
                    lens_name: lens_name,
                    lens_types: lens_types.unwrap_or_default(),
                    lens_imp: lens_imp,
                });
            }
        }
    }
    lens_out
}

fn lens_read_ivar_list(lens_r: &LensResolver, lens_m: &LensMachOImage, lens_d: &[u8], lens_list_vm: u64) -> Vec<LensObjcIvar> {
    let mut lens_out = Vec::new();
    if lens_list_vm == 0 {
        return lens_out;
    }
    let lens_list_off = match lens_m.lens_vmaddr_to_offset(lens_list_vm) {
        Some(lens_o) => lens_o,
        None => return lens_out,
    };
    let lens_entsize = match lens_u32_at(lens_d, lens_list_off) {
        Some(lens_v) if lens_v != 0 => lens_v as u64,
        _ => return lens_out,
    };
    let lens_count = match lens_u32_at(lens_d, lens_list_off + 4) {
        Some(lens_v) => lens_v as usize,
        None => return lens_out,
    };
    let lens_avail = lens_d.len().saturating_sub(lens_list_off + 8) as u64;
    let lens_count = lens_count.min((lens_avail / lens_entsize) as usize);

    for lens_i in 0..lens_count {
        let lens_iv_vm = lens_list_vm
            .wrapping_add(8)
            .wrapping_add((lens_i as u64).wrapping_mul(lens_entsize));
        let lens_offset = lens_r
            .lens_follow(lens_m, lens_d, lens_iv_vm)
            .and_then(|lens_p| lens_u32_vm(lens_m, lens_d, lens_p))
            .unwrap_or(0);
        let lens_name = lens_r
            .lens_follow(lens_m, lens_d, lens_iv_vm.wrapping_add(8))
            .and_then(|lens_p| lens_cstr_vm(lens_m, lens_d, lens_p))
            .unwrap_or_default();
        let lens_type_enc = lens_r
            .lens_follow(lens_m, lens_d, lens_iv_vm.wrapping_add(16))
            .and_then(|lens_p| lens_cstr_vm(lens_m, lens_d, lens_p))
            .unwrap_or_default();
        lens_out.push(LensObjcIvar {
            lens_name: lens_name,
            lens_type_enc: lens_type_enc,
            lens_offset: lens_offset,
        });
    }
    lens_out
}

fn lens_read_class_methods(lens_r: &LensResolver, lens_m: &LensMachOImage, lens_d: &[u8], lens_class_vm: u64) -> Vec<LensObjcMethod> {
    let lens_meta_vm = match lens_r.lens_follow(lens_m, lens_d, lens_class_vm) {
        Some(lens_v) => lens_v,
        None => return Vec::new(),
    };
    let lens_meta_ro_vm = match lens_r.lens_follow(lens_m, lens_d, lens_meta_vm.wrapping_add(LENS_OBJC_CLASS_BITS_OFF as u64)) {
        Some(lens_bits) => lens_bits & LENS_FAST_DATA_MASK,
        None => return Vec::new(),
    };
    let lens_methods_vm = lens_r
        .lens_follow(lens_m, lens_d, lens_meta_ro_vm.wrapping_add(LENS_CLASS_RO_METHODS_OFF as u64))
        .unwrap_or(0);
    lens_read_method_list(lens_r, lens_m, lens_d, lens_methods_vm)
}

fn lens_read_protocol_list(lens_r: &LensResolver, lens_m: &LensMachOImage, lens_d: &[u8], lens_list_vm: u64) -> Vec<String> {
    let mut lens_out = Vec::new();
    if lens_list_vm == 0 {
        return lens_out;
    }
    let lens_list_off = match lens_m.lens_vmaddr_to_offset(lens_list_vm) {
        Some(lens_o) => lens_o,
        None => return lens_out,
    };
    let lens_count = match lens_m.lens_read_u64_at(lens_d, lens_list_off) {
        Some(lens_v) => lens_v as usize,
        None => return lens_out,
    };
    let lens_avail = lens_d.len().saturating_sub(lens_list_off + 8);
    let lens_count = lens_count.min(lens_avail / 8);
    for lens_i in 0..lens_count {
        let lens_proto_field_vm = lens_list_vm
            .wrapping_add(8)
            .wrapping_add((lens_i as u64).wrapping_mul(8));
        let lens_proto_vm = match lens_r.lens_follow(lens_m, lens_d, lens_proto_field_vm) {
            Some(lens_v) => lens_v,
            None => continue,
        };
        let lens_name = lens_r
            .lens_follow(lens_m, lens_d, lens_proto_vm.wrapping_add(LENS_PROTOCOL_NAME_OFF as u64))
            .and_then(|lens_nv| lens_cstr_vm(lens_m, lens_d, lens_nv));
        if let Some(lens_n) = lens_name {
            lens_out.push(lens_n);
        }
    }
    lens_out
}

pub fn lens_parse_objc_classes(lens_buf: &[u8]) -> lens_macho::LensResult<Vec<LensObjcClass>> {
    let lens_macho = LensMachOImage::lens_parse(lens_buf)?;
    let lens_slice = lens_select_arm64_slice(lens_buf)?;
    let lens_sdata = lens_slice.lens_data;

    let mut lens_out = Vec::new();
    let lens_classlist = match lens_macho.lens_section_by_name("__objc_classlist") {
        Some(lens_s) => lens_s,
        None => return Ok(lens_out),
    };

    let lens_resolver = LensResolver::lens_new(&lens_macho);
    let lens_list_addr = lens_classlist.lens_addr;
    let lens_base = lens_classlist.lens_offset as usize;
    let lens_max_by_buf = lens_sdata.len().saturating_sub(lens_base) / 8;
    let lens_count = ((lens_classlist.lens_size / 8) as usize).min(lens_max_by_buf);
    for lens_i in 0..lens_count {
        let lens_entry_vm = lens_list_addr.wrapping_add((lens_i as u64).wrapping_mul(8));
        let lens_class_vm = match lens_resolver.lens_follow(&lens_macho, lens_sdata, lens_entry_vm) {
            Some(lens_v) => lens_v,
            None => continue,
        };
        let lens_ro_vm = match lens_resolver.lens_follow(
            &lens_macho,
            lens_sdata,
            lens_class_vm.wrapping_add(LENS_OBJC_CLASS_BITS_OFF as u64),
        ) {
            Some(lens_bits) => lens_bits & LENS_FAST_DATA_MASK,
            None => continue,
        };
        let lens_name = match lens_resolver
            .lens_follow(&lens_macho, lens_sdata, lens_ro_vm.wrapping_add(LENS_CLASS_RO_NAME_OFF as u64))
            .and_then(|lens_nv| lens_cstr_vm(&lens_macho, lens_sdata, lens_nv))
        {
            Some(lens_n) => lens_n,
            None => continue,
        };

        let lens_super_field = lens_class_vm.wrapping_add(LENS_OBJC_CLASS_SUPER_OFF as u64);
        let lens_superclass = match lens_resolver.lens_follow(&lens_macho, lens_sdata, lens_super_field) {
            Some(lens_v) => lens_class_name_at(&lens_resolver, &lens_macho, lens_sdata, lens_v),
            None => lens_resolver.lens_bind_class_name(lens_super_field),
        };
        let lens_methods_vm = lens_resolver
            .lens_follow(
                &lens_macho,
                lens_sdata,
                lens_ro_vm.wrapping_add(LENS_CLASS_RO_METHODS_OFF as u64),
            )
            .unwrap_or(0);
        let lens_instance_methods = lens_read_method_list(&lens_resolver, &lens_macho, lens_sdata, lens_methods_vm);
        let lens_class_methods = lens_read_class_methods(&lens_resolver, &lens_macho, lens_sdata, lens_class_vm);
        let lens_ivars_vm = lens_resolver
            .lens_follow(&lens_macho, lens_sdata, lens_ro_vm.wrapping_add(LENS_CLASS_RO_IVARS_OFF as u64))
            .unwrap_or(0);
        let lens_ivars = lens_read_ivar_list(&lens_resolver, &lens_macho, lens_sdata, lens_ivars_vm);
        let lens_protos_vm = lens_resolver
            .lens_follow(
                &lens_macho,
                lens_sdata,
                lens_ro_vm.wrapping_add(LENS_CLASS_RO_PROTOCOLS_OFF as u64),
            )
            .unwrap_or(0);
        let lens_protocols = lens_read_protocol_list(&lens_resolver, &lens_macho, lens_sdata, lens_protos_vm);

        lens_out.push(LensObjcClass {
            lens_address: lens_class_vm,
            lens_name: lens_name,
            lens_superclass: lens_superclass,
            lens_instance_methods: lens_instance_methods,
            lens_class_methods: lens_class_methods,
            lens_ivars: lens_ivars,
            lens_protocols: lens_protocols,
        });
    }
    Ok(lens_out)
}

pub fn lens_parse_objc_categories(lens_buf: &[u8]) -> lens_macho::LensResult<Vec<LensObjcCategory>> {
    let lens_macho = LensMachOImage::lens_parse(lens_buf)?;
    let lens_slice = lens_select_arm64_slice(lens_buf)?;
    let lens_sdata = lens_slice.lens_data;

    let mut lens_out = Vec::new();
    let lens_catlist = match lens_macho.lens_section_by_name("__objc_catlist") {
        Some(lens_s) => lens_s,
        None => return Ok(lens_out),
    };
    let lens_resolver = LensResolver::lens_new(&lens_macho);
    let lens_list_addr = lens_catlist.lens_addr;
    let lens_base = lens_catlist.lens_offset as usize;
    let lens_max_by_buf = lens_sdata.len().saturating_sub(lens_base) / 8;
    let lens_count = ((lens_catlist.lens_size / 8) as usize).min(lens_max_by_buf);

    for lens_i in 0..lens_count {
        let lens_entry_vm = lens_list_addr.wrapping_add((lens_i as u64).wrapping_mul(8));
        let lens_cat_vm = match lens_resolver.lens_follow(&lens_macho, lens_sdata, lens_entry_vm) {
            Some(lens_v) => lens_v,
            None => continue,
        };
        let lens_name = match lens_resolver
            .lens_follow(&lens_macho, lens_sdata, lens_cat_vm)
            .and_then(|lens_nv| lens_cstr_vm(&lens_macho, lens_sdata, lens_nv))
        {
            Some(lens_n) => lens_n,
            None => continue,
        };
        let lens_cls_field = lens_cat_vm.wrapping_add(LENS_CATEGORY_CLS_OFF as u64);
        let lens_class_name = match lens_resolver.lens_follow(&lens_macho, lens_sdata, lens_cls_field) {
            Some(lens_v) => lens_class_name_at(&lens_resolver, &lens_macho, lens_sdata, lens_v),
            None => lens_resolver.lens_bind_class_name(lens_cls_field),
        };
        let lens_inst_vm = lens_resolver
            .lens_follow(
                &lens_macho,
                lens_sdata,
                lens_cat_vm.wrapping_add(LENS_CATEGORY_INST_METHODS_OFF as u64),
            )
            .unwrap_or(0);
        let lens_instance_methods = lens_read_method_list(&lens_resolver, &lens_macho, lens_sdata, lens_inst_vm);
        let lens_cls_vm = lens_resolver
            .lens_follow(
                &lens_macho,
                lens_sdata,
                lens_cat_vm.wrapping_add(LENS_CATEGORY_CLASS_METHODS_OFF as u64),
            )
            .unwrap_or(0);
        let lens_class_methods = lens_read_method_list(&lens_resolver, &lens_macho, lens_sdata, lens_cls_vm);
        let lens_protos_vm = lens_resolver
            .lens_follow(
                &lens_macho,
                lens_sdata,
                lens_cat_vm.wrapping_add(LENS_CATEGORY_PROTOCOLS_OFF as u64),
            )
            .unwrap_or(0);
        let lens_protocols = lens_read_protocol_list(&lens_resolver, &lens_macho, lens_sdata, lens_protos_vm);

        lens_out.push(LensObjcCategory {
            lens_name: lens_name,
            lens_class_name: lens_class_name,
            lens_instance_methods: lens_instance_methods,
            lens_class_methods: lens_class_methods,
            lens_protocols: lens_protocols,
        });
    }
    Ok(lens_out)
}

#[cfg(test)]
mod lens_tests {
    use super::*;
    use lens_macho::lens_consts::*;

    fn lens_build_with_objc_strings() -> Vec<u8> {
        let lens_methname = b"init\0dealloc\0";
        let lens_classname = b"Foo\0Bar\0";
        let lens_methtype = b"v16@0:8\0";

        fn lens_sect_bytes(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
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
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s.extend_from_slice(&0u32.to_le_bytes());
            lens_s
        }

        let lens_nsects = 3u32;
        let lens_seg_body_len = 64 + (lens_nsects as usize) * 80;
        let lens_cmdsize = 8 + lens_seg_body_len;
        let lens_data_start = 32 + lens_cmdsize;

        let lens_off_methname = lens_data_start as u32;
        let lens_off_classname = lens_off_methname + lens_methname.len() as u32;
        let lens_off_methtype = lens_off_classname + lens_classname.len() as u32;

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
        lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect_bytes(
            "__objc_methname",
            0x2000,
            lens_methname.len() as u64,
            lens_off_methname,
        ));
        lens_seg.extend_from_slice(&lens_sect_bytes(
            "__objc_classname",
            0x3000,
            lens_classname.len() as u64,
            lens_off_classname,
        ));
        lens_seg.extend_from_slice(&lens_sect_bytes(
            "__objc_methtype",
            0x3100,
            lens_methtype.len() as u64,
            lens_off_methtype,
        ));

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
        lens_v.extend_from_slice(lens_methname);
        lens_v.extend_from_slice(lens_classname);
        lens_v.extend_from_slice(lens_methtype);
        lens_v
    }

    #[test]
    fn lens_parses_all_three_objc_string_pools() {
        let lens_bytes = lens_build_with_objc_strings();
        let lens_objc = lens_parse_objc_strings(&lens_bytes).unwrap();
        let lens_sels: Vec<_> = lens_objc.lens_selectors.iter().map(|lens_s| lens_s.lens_value.as_str()).collect();
        assert_eq!(lens_sels, vec!["init", "dealloc"]);
        assert_eq!(lens_objc.lens_selectors[0].lens_addr, 0x2000);
        let lens_names: Vec<_> = lens_objc.lens_class_names.iter().map(|lens_s| lens_s.lens_value.as_str()).collect();
        assert_eq!(lens_names, vec!["Foo", "Bar"]);
        let lens_types: Vec<_> = lens_objc.lens_method_types.iter().map(|lens_s| lens_s.lens_value.as_str()).collect();
        assert_eq!(lens_types, vec!["v16@0:8"]);
    }

    fn lens_build_with_one_class() -> Vec<u8> {
        let lens_nsects = 1u32;
        let lens_seg_body_len = 64 + (lens_nsects as usize) * 80;
        let lens_cmdsize = 8 + lens_seg_body_len;
        let lens_ds = 32 + lens_cmdsize;

        let lens_addr_class = (lens_ds + 8) as u64;
        let lens_addr_ro = (lens_ds + 48) as u64;
        let lens_addr_name = (lens_ds + 80) as u64;

        let mut lens_data = vec![0u8; 88];
        lens_data[0..8].copy_from_slice(&lens_addr_class.to_le_bytes());
        lens_data[40..48].copy_from_slice(&(lens_addr_ro | 0x3).to_le_bytes());
        lens_data[72..80].copy_from_slice(&lens_addr_name.to_le_bytes());
        lens_data[80..88].copy_from_slice(b"MyClass\0");

        fn lens_sect(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
            let mut lens_s = Vec::new();
            let mut lens_sn = lens_name.as_bytes().to_vec();
            lens_sn.resize(16, 0);
            let mut lens_sg = b"__DATA".to_vec();
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
        let mut lens_segn = b"__DATA".to_vec();
        lens_segn.resize(16, 0);
        lens_seg.extend_from_slice(&lens_segn);
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&(lens_data.len() as u64).to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect("__objc_classlist", lens_ds as u64, 8, lens_ds as u32));

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
        lens_v.extend_from_slice(&lens_data);
        lens_v
    }

    #[test]
    fn lens_parse_objc_classes_resolves_class_name() {
        let lens_bytes = lens_build_with_one_class();
        let lens_classes = lens_parse_objc_classes(&lens_bytes).unwrap();
        assert_eq!(lens_classes.len(), 1);
        assert_eq!(lens_classes[0].lens_name, "MyClass");
        assert_eq!(lens_classes[0].lens_address, (32 + 152 + 8) as u64);
    }

    #[test]
    fn lens_parse_objc_classes_empty_when_no_classlist() {
        let lens_bytes = lens_build_with_objc_strings();
        let lens_classes = lens_parse_objc_classes(&lens_bytes).unwrap();
        assert!(lens_classes.is_empty());
    }

    fn lens_build_class_with_method_and_ivar() -> Vec<u8> {
        let lens_nsects = 1u32;
        let lens_cmdsize = 8 + 64 + (lens_nsects as usize) * 80;
        let lens_ds = 32 + lens_cmdsize;

        let lens_a = |lens_local: usize| (lens_ds + lens_local) as u64;
        let mut lens_d = vec![0u8; 0xCD];
        let lens_put64 =
            |lens_d: &mut [u8], lens_at: usize, lens_v: u64| lens_d[lens_at..lens_at + 8].copy_from_slice(&lens_v.to_le_bytes());
        let lens_put32 =
            |lens_d: &mut [u8], lens_at: usize, lens_v: u32| lens_d[lens_at..lens_at + 4].copy_from_slice(&lens_v.to_le_bytes());

        lens_put64(&mut lens_d, 0x00, lens_a(0x08));
        lens_put64(&mut lens_d, 0x28, lens_a(0x30));
        lens_put64(&mut lens_d, 0x48, lens_a(0xB0));
        lens_put64(&mut lens_d, 0x50, lens_a(0x68));
        lens_put64(&mut lens_d, 0x60, lens_a(0x7C));
        lens_put32(&mut lens_d, 0x68, 12 | LENS_SMALL_METHOD_LIST_FLAG);
        lens_put32(&mut lens_d, 0x6C, 1);
        lens_put32(&mut lens_d, 0x70, 0x38);
        lens_put32(&mut lens_d, 0x74, 0x4C);
        lens_put32(&mut lens_d, 0x78, 0);
        lens_put32(&mut lens_d, 0x7C, 32);
        lens_put32(&mut lens_d, 0x80, 1);
        lens_put64(&mut lens_d, 0x84, lens_a(0xA4));
        lens_put64(&mut lens_d, 0x8C, lens_a(0xC8));
        lens_put64(&mut lens_d, 0x94, lens_a(0xCB));
        lens_put32(&mut lens_d, 0x9C, 0);
        lens_put32(&mut lens_d, 0xA0, 8);
        lens_put32(&mut lens_d, 0xA4, 0x10);
        lens_put64(&mut lens_d, 0xA8, lens_a(0xB8));
        lens_d[0xB0..0xB8].copy_from_slice(b"MyClass\0");
        lens_d[0xB8..0xC0].copy_from_slice(b"doThing\0");
        lens_d[0xC0..0xC8].copy_from_slice(b"v16@0:8\0");
        lens_d[0xC8..0xCB].copy_from_slice(b"_x\0");
        lens_d[0xCB..0xCD].copy_from_slice(b"@\0");

        fn lens_sect(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
            let mut lens_s = Vec::new();
            let mut lens_sn = lens_name.as_bytes().to_vec();
            lens_sn.resize(16, 0);
            let mut lens_sg = b"__DATA".to_vec();
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
        let mut lens_segn = b"__DATA".to_vec();
        lens_segn.resize(16, 0);
        lens_seg.extend_from_slice(&lens_segn);
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&(lens_d.len() as u64).to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect("__objc_classlist", lens_ds as u64, 8, lens_ds as u32));

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
    fn lens_parse_objc_classes_reads_relative_methods_and_ivars() {
        let lens_bytes = lens_build_class_with_method_and_ivar();
        let lens_classes = lens_parse_objc_classes(&lens_bytes).unwrap();
        assert_eq!(lens_classes.len(), 1);
        let lens_c = &lens_classes[0];
        assert_eq!(lens_c.lens_name, "MyClass");
        assert_eq!(lens_c.lens_instance_methods.len(), 1);
        assert_eq!(lens_c.lens_instance_methods[0].lens_name, "doThing");
        assert_eq!(lens_c.lens_instance_methods[0].lens_types, "v16@0:8");
        assert_eq!(lens_c.lens_ivars.len(), 1);
        assert_eq!(lens_c.lens_ivars[0].lens_name, "_x");
        assert_eq!(lens_c.lens_ivars[0].lens_type_enc, "@");
        assert_eq!(lens_c.lens_ivars[0].lens_offset, 0x10);
    }

    #[test]
    fn lens_parse_objc_classes_bounds_huge_classlist_size() {
        let lens_nsects = 1u32;
        let lens_seg_body_len = 64 + (lens_nsects as usize) * 80;
        let lens_cmdsize = 8 + lens_seg_body_len;
        let lens_ds = 32 + lens_cmdsize;
        let lens_data = vec![0u8; 8];

        fn lens_sect(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
            let mut lens_s = Vec::new();
            let mut lens_sn = lens_name.as_bytes().to_vec();
            lens_sn.resize(16, 0);
            let mut lens_sg = b"__DATA".to_vec();
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
        let mut lens_segn = b"__DATA".to_vec();
        lens_segn.resize(16, 0);
        lens_seg.extend_from_slice(&lens_segn);
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&(lens_data.len() as u64).to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect(
            "__objc_classlist",
            lens_ds as u64,
            0xFFFF_FFFF_FFFF_FFF8,
            lens_ds as u32,
        ));

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
        lens_v.extend_from_slice(&lens_data);

        let lens_classes = lens_parse_objc_classes(&lens_v).unwrap();
        assert!(lens_classes.is_empty());
    }

    #[test]
    fn lens_read_method_list_no_overflow_on_near_max_list_vm() {
        use lens_macho::lens_segment::LensSegment;
        let lens_macho = LensMachOImage {
            lens_cputype: 0,
            lens_cpusubtype: 0,
            lens_filetype: 0,
            lens_segments: vec![LensSegment {
                lens_segname: "__DATA".to_string(),
                lens_vmaddr: u64::MAX - 4,
                lens_vmsize: 4,
                lens_fileoff: 0,
                lens_filesize: 4,
                lens_sections: Vec::new(),
            }],
            lens_symbols: Vec::new(),
            lens_uuid: None,
            lens_encryption: None,
            lens_function_starts: Vec::new(),
            lens_has_chained_fixups: false,
            lens_binds: Vec::new(),
            lens_chained_fixups: Vec::new(),
        };
        let mut lens_d = vec![0u8; 64];
        lens_d[0..4].copy_from_slice(&(12u32 | LENS_SMALL_METHOD_LIST_FLAG).to_le_bytes());
        lens_d[4..8].copy_from_slice(&1u32.to_le_bytes());
        let lens_r = LensResolver::lens_new(&lens_macho);
        let lens_methods = lens_read_method_list(&lens_r, &lens_macho, &lens_d, u64::MAX - 4);
        assert!(lens_methods.is_empty());
    }

    fn lens_identity_image(lens_len: u64) -> LensMachOImage {
        LensMachOImage {
            lens_cputype: 0,
            lens_cpusubtype: 0,
            lens_filetype: 0,
            lens_segments: vec![lens_macho::lens_segment::LensSegment {
                lens_segname: "__DATA".to_string(),
                lens_vmaddr: 0,
                lens_vmsize: lens_len,
                lens_fileoff: 0,
                lens_filesize: lens_len,
                lens_sections: Vec::new(),
            }],
            lens_symbols: Vec::new(),
            lens_uuid: None,
            lens_encryption: None,
            lens_function_starts: Vec::new(),
            lens_has_chained_fixups: false,
            lens_binds: Vec::new(),
            lens_chained_fixups: Vec::new(),
        }
    }

    fn lens_p64(lens_d: &mut [u8], lens_at: usize, lens_v: u64) {
        lens_d[lens_at..lens_at + 8].copy_from_slice(&lens_v.to_le_bytes());
    }
    fn lens_p32(lens_d: &mut [u8], lens_at: usize, lens_v: u32) {
        lens_d[lens_at..lens_at + 4].copy_from_slice(&lens_v.to_le_bytes());
    }

    fn lens_build_with_one_category() -> Vec<u8> {
        let lens_nsects = 1u32;
        let lens_cmdsize = 8 + 64 + (lens_nsects as usize) * 80;
        let lens_ds = 32 + lens_cmdsize;
        let lens_a = |lens_local: usize| (lens_ds + lens_local) as u64;
        let mut lens_d = vec![0u8; 0x8C];
        let lens_p64 = |lens_d: &mut [u8], lens_at: usize, lens_v: u64| lens_d[lens_at..lens_at + 8].copy_from_slice(&lens_v.to_le_bytes());
        lens_p64(&mut lens_d, 0x00, lens_a(0x08));
        lens_p64(&mut lens_d, 0x08, lens_a(0x78));
        lens_p64(&mut lens_d, 0x10, lens_a(0x30));
        lens_p64(&mut lens_d, 0x50, lens_a(0x58));
        lens_p64(&mut lens_d, 0x70, lens_a(0x80));
        lens_d[0x78..0x7E].copy_from_slice(b"MyCat\0");
        lens_d[0x80..0x8C].copy_from_slice(b"TargetClass\0");

        fn lens_sect(lens_name: &str, lens_addr: u64, lens_size: u64, lens_offset: u32) -> Vec<u8> {
            let mut lens_s = Vec::new();
            let mut lens_sn = lens_name.as_bytes().to_vec();
            lens_sn.resize(16, 0);
            let mut lens_sg = b"__DATA".to_vec();
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
        let mut lens_segn = b"__DATA".to_vec();
        lens_segn.resize(16, 0);
        lens_seg.extend_from_slice(&lens_segn);
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&0x1000u64.to_le_bytes());
        lens_seg.extend_from_slice(&(lens_ds as u64).to_le_bytes());
        lens_seg.extend_from_slice(&(lens_d.len() as u64).to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&3u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_nsects.to_le_bytes());
        lens_seg.extend_from_slice(&0u32.to_le_bytes());
        lens_seg.extend_from_slice(&lens_sect("__objc_catlist", lens_ds as u64, 8, lens_ds as u32));

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
    fn lens_parse_objc_categories_resolves_name_and_class() {
        let lens_bytes = lens_build_with_one_category();
        let lens_cats = lens_parse_objc_categories(&lens_bytes).unwrap();
        assert_eq!(lens_cats.len(), 1);
        assert_eq!(lens_cats[0].lens_name, "MyCat");
        assert_eq!(lens_cats[0].lens_class_name.as_deref(), Some("TargetClass"));
    }

    #[test]
    fn lens_read_protocol_list_resolves_names() {
        let lens_img = lens_identity_image(0x1000);
        let mut lens_d = vec![0u8; 0x100];
        lens_p64(&mut lens_d, 0x10, 1);
        lens_p64(&mut lens_d, 0x18, 0x30);
        lens_p64(&mut lens_d, 0x38, 0x50);
        lens_d[0x50..0x58].copy_from_slice(b"MyProto\0");
        let lens_r = LensResolver::lens_new(&lens_img);
        assert_eq!(
            lens_read_protocol_list(&lens_r, &lens_img, &lens_d, 0x10),
            vec!["MyProto".to_string()]
        );
    }

    #[test]
    fn lens_read_class_methods_via_metaclass() {
        let lens_img = lens_identity_image(0x1000);
        let mut lens_d = vec![0u8; 0x200];
        lens_p64(&mut lens_d, 0x100, 0x120);
        lens_p64(&mut lens_d, 0x140, 0x160);
        lens_p64(&mut lens_d, 0x180, 0x1A0);
        lens_p32(&mut lens_d, 0x1A0, 24);
        lens_p32(&mut lens_d, 0x1A4, 1);
        lens_p64(&mut lens_d, 0x1A8, 0x1D0);
        lens_p64(&mut lens_d, 0x1B0, 0x1E0);
        lens_d[0x1D0..0x1D3].copy_from_slice(b"cm\0");
        lens_d[0x1E0..0x1E8].copy_from_slice(b"v16@0:8\0");
        let lens_r = LensResolver::lens_new(&lens_img);
        let lens_ms = lens_read_class_methods(&lens_r, &lens_img, &lens_d, 0x100);
        assert_eq!(lens_ms.len(), 1);
        assert_eq!(lens_ms[0].lens_name, "cm");
        assert_eq!(lens_ms[0].lens_types, "v16@0:8");
    }

    #[test]
    fn lens_resolver_follows_chained_rebase_and_bind() {
        use lens_macho::lens_chained_fixups::{LensChainedFixup, LensFixupTarget};
        let mut lens_img = lens_identity_image(0x1000);
        lens_img.lens_chained_fixups.push(LensChainedFixup {
            lens_address: 0x100,
            lens_target: LensFixupTarget::LensRebase(0x500),
        });
        lens_img.lens_chained_fixups.push(LensChainedFixup {
            lens_address: 0x200,
            lens_target: LensFixupTarget::LensBind("_OBJC_CLASS_$_NSObject".to_string()),
        });
        let lens_r = LensResolver::lens_new(&lens_img);
        let lens_d = vec![0u8; 0x10];
        assert_eq!(lens_r.lens_follow(&lens_img, &lens_d, 0x100), Some(0x500));
        assert_eq!(lens_r.lens_follow(&lens_img, &lens_d, 0x200), None);
        assert_eq!(lens_r.lens_bind_class_name(0x200).as_deref(), Some("NSObject"));
    }
}
