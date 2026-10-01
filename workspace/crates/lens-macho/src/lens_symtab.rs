use crate::lens_reader::LensReader;
use crate::{LensError, LensResult};

#[derive(PartialEq, Eq)]
pub struct LensSymbol {
    pub lens_name: String,
    pub lens_value: u64,
    pub lens_n_type: u8,
    pub lens_n_sect: u8,
}

fn lens_read_c_str(lens_file: &[u8], lens_stroff: usize, lens_strsize: usize, lens_n_strx: u32) -> LensResult<String> {
    let lens_start = lens_stroff
        .checked_add(lens_n_strx as usize)
        .ok_or(LensError::LensEof(lens_stroff))?;
    let lens_table_end = lens_stroff.checked_add(lens_strsize).ok_or(LensError::LensEof(lens_stroff))?;
    if lens_start > lens_table_end || lens_table_end > lens_file.len() {
        return Err(LensError::LensMalformed("string index out of range"));
    }
    let lens_bytes = &lens_file[lens_start..lens_table_end];
    let lens_end = lens_bytes.iter().position(|&lens_c| lens_c == 0).unwrap_or(lens_bytes.len());
    Ok(String::from_utf8_lossy(&lens_bytes[..lens_end]).into_owned())
}

pub fn lens_parse_symtab(lens_file: &[u8], lens_body: &[u8]) -> LensResult<Vec<LensSymbol>> {
    let mut lens_r = LensReader::lens_new(lens_body);
    let lens_symoff = lens_r.lens_read_u32()? as usize;
    let lens_nsyms = lens_r.lens_read_u32()? as usize;
    let lens_stroff = lens_r.lens_read_u32()? as usize;
    let lens_strsize = lens_r.lens_read_u32()? as usize;

    let mut lens_out = Vec::new();
    let mut lens_sr = LensReader::lens_at(lens_file, lens_symoff)?;
    for _ in 0..lens_nsyms {
        let lens_n_strx = lens_sr.lens_read_u32()?;
        let lens_n_type = lens_sr.lens_read_u8()?;
        let lens_n_sect = lens_sr.lens_read_u8()?;
        let lens__n_desc = lens_sr.lens_read_u16()?;
        let lens_n_value = lens_sr.lens_read_u64()?;
        let lens_name = lens_read_c_str(lens_file, lens_stroff, lens_strsize, lens_n_strx)?;
        lens_out.push(LensSymbol {
            lens_name: lens_name,
            lens_value: lens_n_value,
            lens_n_type: lens_n_type,
            lens_n_sect: lens_n_sect,
        });
    }
    Ok(lens_out)
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    #[test]
    fn lens_parses_one_symbol() {
        let lens_strtab = b"\0_main\0";
        let mut lens_file = Vec::new();
        let lens_stroff = 0usize;
        lens_file.extend_from_slice(lens_strtab);
        while lens_file.len() % 4 != 0 {
            lens_file.push(0);
        }
        let lens_symoff = lens_file.len();
        lens_file.extend_from_slice(&1u32.to_le_bytes());
        lens_file.push(0x0f);
        lens_file.push(0x01);
        lens_file.extend_from_slice(&0u16.to_le_bytes());
        lens_file.extend_from_slice(&0x4000u64.to_le_bytes());

        let mut lens_body = Vec::new();
        lens_body.extend_from_slice(&(lens_symoff as u32).to_le_bytes());
        lens_body.extend_from_slice(&1u32.to_le_bytes());
        lens_body.extend_from_slice(&(lens_stroff as u32).to_le_bytes());
        lens_body.extend_from_slice(&(lens_strtab.len() as u32).to_le_bytes());

        let lens_syms = lens_parse_symtab(&lens_file, &lens_body).unwrap();
        assert_eq!(lens_syms.len(), 1);
        assert_eq!(lens_syms[0].lens_name, "_main");
        assert_eq!(lens_syms[0].lens_value, 0x4000);
        assert_eq!(lens_syms[0].lens_n_type, 0x0f);
    }

    #[test]
    fn lens_huge_nsyms_with_tiny_buffer_errors_fast() {
        let lens_file = vec![0u8; 16];
        let mut lens_body = Vec::new();
        lens_body.extend_from_slice(&8u32.to_le_bytes());
        lens_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        lens_body.extend_from_slice(&0u32.to_le_bytes());
        lens_body.extend_from_slice(&0u32.to_le_bytes());
        assert!(lens_parse_symtab(&lens_file, &lens_body).is_err());
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensSymbol { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("Symbol");lens_debug.field("name", &self.lens_name);lens_debug.field("value", &self.lens_value);lens_debug.field("n_type", &self.lens_n_type);lens_debug.field("n_sect", &self.lens_n_sect);lens_debug.finish() } }
