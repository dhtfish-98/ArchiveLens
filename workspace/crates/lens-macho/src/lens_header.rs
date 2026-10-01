use crate::lens_consts::*;
use crate::lens_fat::LensSlice;
use crate::lens_reader::LensReader;
use crate::{LensError, LensResult};

#[derive(PartialEq, Eq)]
pub struct LensMachHeader {
    pub lens_cputype: u32,
    pub lens_cpusubtype: u32,
    pub lens_filetype: u32,
    pub lens_ncmds: u32,
    pub lens_flags: u32,
}

pub struct LensLoadCommand<'a> {
    pub lens_cmd: u32,
    pub lens_body: &'a [u8],
}

const LENS_MACH_HEADER_64_SIZE: usize = 32;

pub fn lens_parse_header(lens_slice: &LensSlice) -> LensResult<LensMachHeader> {
    let mut lens_r = LensReader::lens_new(lens_slice.lens_data);
    let lens_magic = lens_r.lens_read_u32()?;
    if lens_magic != LENS_MH_MAGIC_64 {
        return Err(LensError::LensBadMagic(lens_magic));
    }
    let lens_cputype = lens_r.lens_read_u32()?;
    let lens_cpusubtype = lens_r.lens_read_u32()?;
    let lens_filetype = lens_r.lens_read_u32()?;
    let lens_ncmds = lens_r.lens_read_u32()?;
    let lens__sizeofcmds = lens_r.lens_read_u32()?;
    let lens_flags = lens_r.lens_read_u32()?;
    let lens__reserved = lens_r.lens_read_u32()?;
    Ok(LensMachHeader {
        lens_cputype: lens_cputype,
        lens_cpusubtype: lens_cpusubtype,
        lens_filetype: lens_filetype,
        lens_ncmds: lens_ncmds,
        lens_flags: lens_flags,
    })
}

pub fn lens_load_commands<'a>(lens_slice: &'a LensSlice) -> LensResult<Vec<LensLoadCommand<'a>>> {
    let lens_header = lens_parse_header(lens_slice)?;
    let mut lens_out = Vec::new();
    let mut lens_offset = LENS_MACH_HEADER_64_SIZE;
    for _ in 0..lens_header.lens_ncmds {
        let mut lens_r = LensReader::lens_at(lens_slice.lens_data, lens_offset)?;
        let lens_cmd = lens_r.lens_read_u32()?;
        let lens_cmdsize = lens_r.lens_read_u32()? as usize;
        if lens_cmdsize < 8 {
            return Err(LensError::LensMalformed("cmdsize < 8"));
        }
        let lens_body_len = lens_cmdsize - 8;
        let lens_body = lens_r.lens_read_bytes(lens_body_len)?;
        lens_out.push(LensLoadCommand { lens_cmd: lens_cmd, lens_body: lens_body });
        lens_offset = lens_offset.checked_add(lens_cmdsize).ok_or(LensError::LensEof(lens_offset))?;
    }
    Ok(lens_out)
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_slice_with(lens_cmds: &[(u32, Vec<u8>)]) -> Vec<u8> {
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
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
        lens_v.extend_from_slice(&2u32.to_le_bytes());
        lens_v.extend_from_slice(&(lens_cmds.len() as u32).to_le_bytes());
        lens_v.extend_from_slice(&(lens_lc.len() as u32).to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&lens_lc);
        lens_v
    }

    #[test]
    fn lens_parses_header_fields() {
        let lens_bytes = lens_slice_with(&[]);
        let lens_s = LensSlice {
            lens_cputype: LENS_CPU_TYPE_ARM64,
            lens_cpusubtype: 0,
            lens_data: &lens_bytes,
        };
        let lens_h = lens_parse_header(&lens_s).unwrap();
        assert_eq!(lens_h.lens_cputype, LENS_CPU_TYPE_ARM64);
        assert_eq!(lens_h.lens_filetype, 2);
        assert_eq!(lens_h.lens_ncmds, 0);
    }

    #[test]
    fn lens_iterates_load_commands() {
        let lens_bytes = lens_slice_with(&[(LENS_LC_UUID, vec![0xaa; 16]), (LENS_LC_SYMTAB, vec![0xbb; 16])]);
        let lens_s = LensSlice {
            lens_cputype: LENS_CPU_TYPE_ARM64,
            lens_cpusubtype: 0,
            lens_data: &lens_bytes,
        };
        let lens_cmds = lens_load_commands(&lens_s).unwrap();
        assert_eq!(lens_cmds.len(), 2);
        assert_eq!(lens_cmds[0].lens_cmd, LENS_LC_UUID);
        assert_eq!(lens_cmds[0].lens_body, &[0xaa; 16]);
        assert_eq!(lens_cmds[1].lens_cmd, LENS_LC_SYMTAB);
    }

    #[test]
    fn lens_truncated_load_command_errors() {
        let mut lens_bytes = lens_slice_with(&[(LENS_LC_UUID, vec![0xaa; 16])]);
        lens_bytes.truncate(lens_bytes.len() - 4);
        let lens_s = LensSlice {
            lens_cputype: LENS_CPU_TYPE_ARM64,
            lens_cpusubtype: 0,
            lens_data: &lens_bytes,
        };
        assert!(lens_load_commands(&lens_s).is_err());
    }

    #[test]
    fn lens_huge_ncmds_with_tiny_buffer_errors_fast() {
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
        lens_v.extend_from_slice(&2u32.to_le_bytes());
        lens_v.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        lens_v.extend_from_slice(&0x1000u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        lens_v.extend_from_slice(&0u32.to_le_bytes());
        let lens_s = LensSlice {
            lens_cputype: LENS_CPU_TYPE_ARM64,
            lens_cpusubtype: 0,
            lens_data: &lens_v,
        };
        assert!(lens_load_commands(&lens_s).is_err());
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensMachHeader { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("MachHeader");lens_debug.field("cputype", &self.lens_cputype);lens_debug.field("cpusubtype", &self.lens_cpusubtype);lens_debug.field("filetype", &self.lens_filetype);lens_debug.field("ncmds", &self.lens_ncmds);lens_debug.field("flags", &self.lens_flags);lens_debug.finish() } }
