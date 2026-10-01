use lens_macho::lens_consts::*;

pub struct LensMachoBuilder {
    lens_cputype: u32,
    lens_cpusubtype: u32,
    lens_load_commands: Vec<(u32, Vec<u8>)>,
}

impl LensMachoBuilder {
    pub fn lens_new_thin_arm64() -> LensMachoBuilder {
        LensMachoBuilder {
            lens_cputype: LENS_CPU_TYPE_ARM64,
            lens_cpusubtype: LENS_CPU_SUBTYPE_ARM64_ALL,
            lens_load_commands: Vec::new(),
        }
    }

    pub fn lens_add_load_command(&mut self, lens_cmd: u32, lens_body: &[u8]) -> &mut Self {
        self.lens_load_commands.push((lens_cmd, lens_body.to_vec()));
        self
    }

    pub fn lens_build(&self) -> Vec<u8> {
        let mut lens_lc_bytes = Vec::new();
        for (lens_cmd, lens_body) in &self.lens_load_commands {
            let lens_raw = 8 + lens_body.len();
            let lens_cmdsize = (lens_raw + 7) & !7;
            lens_lc_bytes.extend_from_slice(&lens_cmd.to_le_bytes());
            lens_lc_bytes.extend_from_slice(&(lens_cmdsize as u32).to_le_bytes());
            lens_lc_bytes.extend_from_slice(lens_body);
            lens_lc_bytes.resize(lens_lc_bytes.len() + (lens_cmdsize - lens_raw), 0);
        }
        let mut lens_out = Vec::new();
        lens_out.extend_from_slice(&LENS_MH_MAGIC_64.to_le_bytes());
        lens_out.extend_from_slice(&self.lens_cputype.to_le_bytes());
        lens_out.extend_from_slice(&self.lens_cpusubtype.to_le_bytes());
        lens_out.extend_from_slice(&2u32.to_le_bytes());
        lens_out.extend_from_slice(&(self.lens_load_commands.len() as u32).to_le_bytes());
        lens_out.extend_from_slice(&(lens_lc_bytes.len() as u32).to_le_bytes());
        lens_out.extend_from_slice(&0u32.to_le_bytes());
        lens_out.extend_from_slice(&0u32.to_le_bytes());
        lens_out.extend_from_slice(&lens_lc_bytes);
        lens_out
    }
}

#[test]
fn lens_builder_emits_parseable_header_size() {
    let lens_bytes = LensMachoBuilder::lens_new_thin_arm64().lens_build();
    assert_eq!(lens_bytes.len(), 32);
    assert_eq!(&lens_bytes[0..4], &LENS_MH_MAGIC_64.to_le_bytes());
}
