use crate::lens_consts::*;
use crate::lens_reader::LensReader;
use crate::{LensError, LensResult};

pub struct LensSlice<'a> {
    pub lens_cputype: u32,
    pub lens_cpusubtype: u32,
    pub lens_data: &'a [u8],
}

fn lens_is_arm64(lens_cputype: u32) -> bool {
    lens_cputype == LENS_CPU_TYPE_ARM64
}

pub fn lens_select_arm64_slice(lens_buf: &[u8]) -> LensResult<LensSlice<'_>> {
    let mut lens_r = LensReader::lens_new(lens_buf);
    let lens_magic = lens_r.lens_read_u32_be()?;
    match lens_magic {
        LENS_FAT_MAGIC | LENS_FAT_MAGIC_64 => {
            let lens_is64 = lens_magic == LENS_FAT_MAGIC_64;
            let lens_nfat = lens_r.lens_read_u32_be()?;
            for _ in 0..lens_nfat {
                let lens_cputype = lens_r.lens_read_u32_be()?;
                let lens_cpusubtype = lens_r.lens_read_u32_be()?;
                let (lens_offset, lens_size) = if lens_is64 {
                    let lens_off = lens_r.lens_read_u32_be()? as u64;
                    let _ = lens_off;
                    return Err(LensError::LensMalformed("fat64 unsupported in v1"));
                } else {
                    let lens_off = lens_r.lens_read_u32_be()? as usize;
                    let lens_size = lens_r.lens_read_u32_be()? as usize;
                    let lens__align = lens_r.lens_read_u32_be()?;
                    (lens_off, lens_size)
                };
                if lens_is_arm64(lens_cputype) {
                    let lens_end = lens_offset.checked_add(lens_size).ok_or(LensError::LensEof(lens_offset))?;
                    if lens_end > lens_buf.len() {
                        return Err(LensError::LensEof(lens_offset));
                    }
                    return Ok(LensSlice {
                        lens_cputype: lens_cputype,
                        lens_cpusubtype: lens_cpusubtype,
                        lens_data: &lens_buf[lens_offset..lens_end],
                    });
                }
            }
            Err(LensError::LensNoArm64Slice)
        }
        _ => {
            let mut lens_r2 = LensReader::lens_new(lens_buf);
            let lens_magic_le = lens_r2.lens_read_u32()?;
            if lens_magic_le != LENS_MH_MAGIC_64 {
                return Err(LensError::LensBadMagic(lens_magic_le));
            }
            let lens_cputype = lens_r2.lens_read_u32()?;
            let lens_cpusubtype = lens_r2.lens_read_u32()?;
            if !lens_is_arm64(lens_cputype) {
                return Err(LensError::LensNoArm64Slice);
            }
            Ok(LensSlice {
                lens_cputype: lens_cputype,
                lens_cpusubtype: lens_cpusubtype,
                lens_data: lens_buf,
            })
        }
    }
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_thin_arm64_header() -> Vec<u8> {
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_le_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_le_bytes());
        lens_v.resize(32, 0);
        lens_v
    }

    #[test]
    fn lens_thin_binary_returns_whole_buffer() {
        let lens_bytes = lens_thin_arm64_header();
        let lens_s = lens_select_arm64_slice(&lens_bytes).unwrap();
        assert_eq!(lens_s.lens_cputype, LENS_CPU_TYPE_ARM64);
        assert_eq!(lens_s.lens_data.len(), lens_bytes.len());
    }

    #[test]
    fn lens_fat_binary_selects_arm64_slice() {
        let lens_payload = lens_thin_arm64_header();
        let lens_payload_offset = 4 + 4 + 20;
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_FAT_MAGIC.to_be_bytes());
        lens_v.extend_from_slice(&1u32.to_be_bytes());
        lens_v.extend_from_slice(&LENS_CPU_TYPE_ARM64.to_be_bytes());
        lens_v.extend_from_slice(&LENS_CPU_SUBTYPE_ARM64_ALL.to_be_bytes());
        lens_v.extend_from_slice(&(lens_payload_offset as u32).to_be_bytes());
        lens_v.extend_from_slice(&(lens_payload.len() as u32).to_be_bytes());
        lens_v.extend_from_slice(&0u32.to_be_bytes());
        lens_v.extend_from_slice(&lens_payload);
        let lens_s = lens_select_arm64_slice(&lens_v).unwrap();
        assert_eq!(lens_s.lens_data, lens_payload.as_slice());
    }

    #[test]
    fn lens_non_arm64_thin_binary_errors() {
        let mut lens_v = Vec::new();
        lens_v.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_v.extend_from_slice(&0x0100_0007u32.to_le_bytes());
        lens_v.resize(32, 0);
        assert_eq!(lens_select_arm64_slice(&lens_v).err(), Some(LensError::LensNoArm64Slice));
    }
}
