pub mod lens_cfg;

#[derive(Clone, PartialEq, Eq)]
pub enum LensFlow {
    LensFallthrough,
    LensBranch(u64),
    LensCall(u64),
    LensCondBranch(u64),
    LensReturn,
    LensIndirect,
    LensIndirectCall,
}

#[derive(Clone, PartialEq, Eq)]
pub enum LensFlagKind {
    LensCmp,
    LensCmn,
    LensTst,
}

#[derive(Clone, PartialEq, Eq)]
pub struct LensFlagOp {
    pub lens_a: String,
    pub lens_b: String,
    pub lens_kind: LensFlagKind,
}

#[derive(Clone)]
pub struct LensInsn {
    pub lens_addr: u64,
    pub lens_raw: u32,
    pub lens_text: String,
    pub lens_flow: LensFlow,
    pub lens_cond: Option<u8>,
    pub lens_flags: Option<LensFlagOp>,
}

impl LensInsn {
    fn lens_with_cond(mut self, lens_c: u8) -> LensInsn {
        self.lens_cond = Some(lens_c);
        self
    }
    fn lens_with_flags(mut self, lens_a: String, lens_b: String, lens_kind: LensFlagKind) -> LensInsn {
        self.lens_flags = Some(LensFlagOp { lens_a: lens_a, lens_b: lens_b, lens_kind: lens_kind });
        self
    }
}

fn lens_reg(lens_n: u32, lens_sf: bool, lens_sp: bool) -> String {
    if lens_n == 31 {
        if lens_sp {
            "sp".to_string()
        } else if lens_sf {
            "xzr".to_string()
        } else {
            "wzr".to_string()
        }
    } else {
        format!("{}{}", if lens_sf { "x" } else { "w" }, lens_n)
    }
}

fn lens_sx(lens_n: u32, lens_sf: bool) -> String {
    lens_reg(lens_n, lens_sf, false)
}

fn lens_sext(lens_v: u32, lens_bits: u32) -> i64 {
    let lens_shift = 64 - lens_bits;
    ((lens_v as u64) << lens_shift) as i64 >> lens_shift
}

const LENS_COND: [&str; 16] = [
    "eq", "ne", "cs", "cc", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le", "al", "nv",
];

pub fn lens_decode(lens_raw: u32, lens_addr: u64) -> LensInsn {
    let lens_mk = |lens_text: String, lens_flow: LensFlow| LensInsn {
        lens_addr: lens_addr,
        lens_raw: lens_raw,
        lens_text: lens_text,
        lens_flow: lens_flow,
        lens_cond: None,
        lens_flags: None,
    };
    let lens_unk = || lens_mk(format!(".word 0x{lens_raw:08x}"), LensFlow::LensFallthrough);

    if lens_raw == 0xD503_201F {
        return lens_mk("nop".to_string(), LensFlow::LensFallthrough);
    }
    if lens_raw & 0xFFFF_FC1F == 0xD65F_0000 {
        return lens_mk("ret".to_string(), LensFlow::LensReturn);
    }
    if lens_raw & 0xFFFF_FC1F == 0xD61F_0000 {
        let lens_rn = (lens_raw >> 5) & 0x1f;
        return lens_mk(format!("br {}", lens_sx(lens_rn, true)), LensFlow::LensIndirect);
    }
    if lens_raw & 0xFFFF_FC1F == 0xD63F_0000 {
        let lens_rn = (lens_raw >> 5) & 0x1f;
        return lens_mk(format!("blr {}", lens_sx(lens_rn, true)), LensFlow::LensIndirectCall);
    }
    if lens_raw & 0x7C00_0000 == 0x1400_0000 {
        let lens_imm = lens_sext(lens_raw & 0x03FF_FFFF, 26) << 2;
        let lens_target = lens_addr.wrapping_add(lens_imm as u64);
        let lens_link = lens_raw & 0x8000_0000 != 0;
        let lens_mn = if lens_link { "bl" } else { "b" };
        let lens_flow = if lens_link {
            LensFlow::LensCall(lens_target)
        } else {
            LensFlow::LensBranch(lens_target)
        };
        return lens_mk(format!("{lens_mn} 0x{lens_target:x}"), lens_flow);
    }
    if lens_raw & 0xFF00_0010 == 0x5400_0000 {
        let lens_imm = lens_sext((lens_raw >> 5) & 0x7FFFF, 19) << 2;
        let lens_target = lens_addr.wrapping_add(lens_imm as u64);
        let lens_code = (lens_raw & 0xf) as u8;
        let lens_cond = LENS_COND[lens_code as usize];
        return lens_mk(format!("b.{lens_cond} 0x{lens_target:x}"), LensFlow::LensCondBranch(lens_target)).lens_with_cond(lens_code);
    }
    if lens_raw & 0x7E00_0000 == 0x3400_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_neg = lens_raw & 0x0100_0000 != 0;
        let lens_imm = lens_sext((lens_raw >> 5) & 0x7FFFF, 19) << 2;
        let lens_target = lens_addr.wrapping_add(lens_imm as u64);
        let lens_rt = lens_raw & 0x1f;
        let lens_mn = if lens_neg { "cbnz" } else { "cbz" };
        return lens_mk(
            format!("{lens_mn} {}, 0x{lens_target:x}", lens_sx(lens_rt, lens_sf)),
            LensFlow::LensCondBranch(lens_target),
        );
    }
    if lens_raw & 0x7E00_0000 == 0x3600_0000 {
        let lens_neg = lens_raw & 0x0100_0000 != 0;
        let lens_b5 = (lens_raw >> 31) & 1;
        let lens_b40 = (lens_raw >> 19) & 0x1f;
        let lens_bit = (lens_b5 << 5) | lens_b40;
        let lens_imm = lens_sext((lens_raw >> 5) & 0x3FFF, 14) << 2;
        let lens_target = lens_addr.wrapping_add(lens_imm as u64);
        let lens_rt = lens_raw & 0x1f;
        let lens_sf = lens_b5 == 1;
        let lens_mn = if lens_neg { "tbnz" } else { "tbz" };
        return lens_mk(
            format!("{lens_mn} {}, #{lens_bit}, 0x{lens_target:x}", lens_sx(lens_rt, lens_sf)),
            LensFlow::LensCondBranch(lens_target),
        );
    }
    if lens_raw & 0xFFE0_001F == 0xD400_0001 {
        let lens_imm = (lens_raw >> 5) & 0xffff;
        return lens_mk(format!("svc #0x{lens_imm:x}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x1F00_0000 == 0x1000_0000 {
        let lens_op = lens_raw & 0x8000_0000 != 0;
        let lens_immlo = (lens_raw >> 29) & 0x3;
        let lens_immhi = (lens_raw >> 5) & 0x7FFFF;
        let lens_imm = (lens_immhi << 2) | lens_immlo;
        let lens_rd = lens_raw & 0x1f;
        if lens_op {
            let lens_base = lens_addr & !0xFFF;
            let lens_target = lens_base.wrapping_add((lens_sext(lens_imm, 21) << 12) as u64);
            return lens_mk(
                format!("adrp {}, 0x{lens_target:x}", lens_sx(lens_rd, true)),
                LensFlow::LensFallthrough,
            );
        } else {
            let lens_target = lens_addr.wrapping_add(lens_sext(lens_imm, 21) as u64);
            return lens_mk(
                format!("adr {}, 0x{lens_target:x}", lens_sx(lens_rd, true)),
                LensFlow::LensFallthrough,
            );
        }
    }

    if lens_raw & 0x1F80_0000 == 0x1280_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_opc = (lens_raw >> 29) & 0x3;
        let lens_hw = (lens_raw >> 21) & 0x3;
        let lens_imm16 = (lens_raw >> 5) & 0xFFFF;
        let lens_rd = lens_raw & 0x1f;
        let lens_shift = lens_hw * 16;
        let lens_lsl = if lens_shift == 0 {
            String::new()
        } else {
            format!(", lsl #{lens_shift}")
        };
        return match lens_opc {
            0b00 => lens_mk(
                format!("movn {}, #0x{lens_imm16:x}{lens_lsl}", lens_sx(lens_rd, lens_sf)),
                LensFlow::LensFallthrough,
            ),
            0b10 => {
                if lens_shift == 0 {
                    lens_mk(
                        format!("mov {}, #0x{lens_imm16:x}", lens_sx(lens_rd, lens_sf)),
                        LensFlow::LensFallthrough,
                    )
                } else {
                    lens_mk(
                        format!("movz {}, #0x{lens_imm16:x}{lens_lsl}", lens_sx(lens_rd, lens_sf)),
                        LensFlow::LensFallthrough,
                    )
                }
            }
            0b11 => lens_mk(
                format!("movk {}, #0x{lens_imm16:x}{lens_lsl}", lens_sx(lens_rd, lens_sf)),
                LensFlow::LensFallthrough,
            ),
            _ => lens_unk(),
        };
    }

    if lens_raw & 0x1F00_0000 == 0x1100_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_sub = lens_raw & 0x4000_0000 != 0;
        let lens_set = lens_raw & 0x2000_0000 != 0;
        let lens_sh = lens_raw & 0x0040_0000 != 0;
        let lens_imm = (lens_raw >> 10) & 0xFFF;
        let lens_imm = if lens_sh { lens_imm << 12 } else { lens_imm };
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        if lens_set && lens_rd == 31 {
            let lens_mn = if lens_sub { "cmp" } else { "cmn" };
            let lens_kind = if lens_sub { LensFlagKind::LensCmp } else { LensFlagKind::LensCmn };
            return lens_mk(
                format!("{lens_mn} {}, #0x{lens_imm:x}", lens_reg(lens_rn, lens_sf, true)),
                LensFlow::LensFallthrough,
            )
            .lens_with_flags(lens_reg(lens_rn, lens_sf, true), format!("0x{lens_imm:x}"), lens_kind);
        }
        if !lens_sub && !lens_set && lens_imm == 0 && (lens_rd == 31 || lens_rn == 31) {
            return lens_mk(
                format!("mov {}, {}", lens_reg(lens_rd, lens_sf, true), lens_reg(lens_rn, lens_sf, true)),
                LensFlow::LensFallthrough,
            );
        }
        let lens_mn = match (lens_sub, lens_set) {
            (false, false) => "add",
            (false, true) => "adds",
            (true, false) => "sub",
            (true, true) => "subs",
        };
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, #0x{lens_imm:x}",
                lens_reg(lens_rd, lens_sf, !lens_set),
                lens_reg(lens_rn, lens_sf, true)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1F00_0000 == 0x0A00_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_opc = (lens_raw >> 29) & 0x3;
        let lens_n = lens_raw & 0x0020_0000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_imm6 = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_shtype = (lens_raw >> 22) & 0x3;
        if lens_opc == 0b01 && !lens_n && lens_rn == 31 && lens_imm6 == 0 {
            return lens_mk(
                format!("mov {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rm, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        let lens_base = match lens_opc {
            0b00 => "and",
            0b01 => "orr",
            0b10 => "eor",
            0b11 => "ands",
            _ => return lens_unk(),
        };
        let lens_mn = if lens_n {
            match lens_opc {
                0b00 => "bic",
                0b01 => "orn",
                0b10 => "eon",
                _ => "bics",
            }
        } else {
            lens_base
        };
        if lens_opc == 0b11 && !lens_n && lens_rd == 31 {
            return lens_mk(
                format!("tst {}, {}", lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf)),
                LensFlow::LensFallthrough,
            )
            .lens_with_flags(lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf), LensFlagKind::LensTst);
        }
        let lens_sh = lens_shift_str(lens_shtype, lens_imm6);
        return lens_mk(
            format!("{lens_mn} {}, {}, {}{}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf), lens_sh),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1F20_0000 == 0x0B00_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_sub = lens_raw & 0x4000_0000 != 0;
        let lens_set = lens_raw & 0x2000_0000 != 0;
        let lens_shtype = (lens_raw >> 22) & 0x3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_imm6 = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        if lens_set && lens_rd == 31 {
            let lens_mn = if lens_sub { "cmp" } else { "cmn" };
            let lens_kind = if lens_sub { LensFlagKind::LensCmp } else { LensFlagKind::LensCmn };
            let lens_b = format!("{}{}", lens_sx(lens_rm, lens_sf), lens_shift_str(lens_shtype, lens_imm6));
            return lens_mk(format!("{lens_mn} {}, {lens_b}", lens_sx(lens_rn, lens_sf)), LensFlow::LensFallthrough).lens_with_flags(
                lens_sx(lens_rn, lens_sf),
                lens_b,
                lens_kind,
            );
        }
        if lens_sub && lens_rn == 31 {
            let lens_mn = if lens_set { "negs" } else { "neg" };
            return lens_mk(
                format!(
                    "{lens_mn} {}, {}{}",
                    lens_sx(lens_rd, lens_sf),
                    lens_sx(lens_rm, lens_sf),
                    lens_shift_str(lens_shtype, lens_imm6)
                ),
                LensFlow::LensFallthrough,
            );
        }
        let lens_mn = match (lens_sub, lens_set) {
            (false, false) => "add",
            (false, true) => "adds",
            (true, false) => "sub",
            (true, true) => "subs",
        };
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, {}{}",
                lens_sx(lens_rd, lens_sf),
                lens_sx(lens_rn, lens_sf),
                lens_sx(lens_rm, lens_sf),
                lens_shift_str(lens_shtype, lens_imm6)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x3F00_0000 == 0x3900_0000 {
        let lens_size = (lens_raw >> 30) & 0x3;
        let lens_opc = (lens_raw >> 22) & 0x3;
        let lens_imm12 = (lens_raw >> 10) & 0xFFF;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        if lens_size == 3 && lens_opc == 2 {
            let lens_off = lens_imm12 << 3;
            let lens_base = lens_reg(lens_rn, true, true);
            let lens_mem = if lens_off == 0 {
                format!("[{lens_base}]")
            } else {
                format!("[{lens_base}, #0x{lens_off:x}]")
            };
            return lens_mk(format!("prfm #0x{lens_rt:x}, {lens_mem}"), LensFlow::LensFallthrough);
        }
        let (lens_mn, lens_sf, lens_scale) = match (lens_size, lens_opc) {
            (0, 0) => ("strb", false, 0),
            (0, 1) => ("ldrb", false, 0),
            (0, 2) => ("ldrsb", true, 0),
            (0, 3) => ("ldrsb", false, 0),
            (1, 0) => ("strh", false, 1),
            (1, 1) => ("ldrh", false, 1),
            (1, 2) => ("ldrsh", true, 1),
            (1, 3) => ("ldrsh", false, 1),
            (2, 0) => ("str", false, 2),
            (2, 1) => ("ldr", false, 2),
            (2, 2) => ("ldrsw", true, 2),
            (3, 0) => ("str", true, 3),
            (3, 1) => ("ldr", true, 3),
            _ => return lens_unk(),
        };
        let lens_off = lens_imm12 << lens_scale;
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_mem = if lens_off == 0 {
            format!("[{lens_base}]")
        } else {
            format!("[{lens_base}, #0x{lens_off:x}]")
        };
        return lens_mk(format!("{lens_mn} {}, {lens_mem}", lens_sx(lens_rt, lens_sf)), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x3B00_0000 == 0x1800_0000 {
        let lens_opc = (lens_raw >> 30) & 0x3;
        let lens_sf = lens_opc == 1;
        let lens_imm = lens_sext((lens_raw >> 5) & 0x7FFFF, 19) << 2;
        let lens_target = lens_addr.wrapping_add(lens_imm as u64);
        let lens_rt = lens_raw & 0x1f;
        return lens_mk(
            format!("ldr {}, 0x{lens_target:x}", lens_sx(lens_rt, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1FE0_0000 == 0x1A80_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_op = lens_raw & 0x4000_0000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_cond = (lens_raw >> 12) & 0xf;
        let lens_op2 = (lens_raw >> 10) & 0x3;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_cc = LENS_COND[lens_cond as usize];
        if !lens_op && lens_op2 == 1 && lens_rn == 31 && lens_rm == 31 && lens_cond < 0b1110 {
            let lens_inv = LENS_COND[(lens_cond ^ 1) as usize];
            return lens_mk(format!("cset {}, {lens_inv}", lens_sx(lens_rd, lens_sf)), LensFlow::LensFallthrough);
        }
        let lens_mn = match (lens_op, lens_op2) {
            (false, 0) => "csel",
            (false, 1) => "csinc",
            (true, 0) => "csinv",
            (true, 1) => "csneg",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!("{lens_mn} {}, {}, {}, {lens_cc}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1F00_0000 == 0x1B00_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_o0 = lens_raw & 0x0000_8000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_ra = (lens_raw >> 10) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_op31 = (lens_raw >> 21) & 0x7;
        if lens_op31 == 0 {
            if lens_ra == 31 {
                let lens_mn = if lens_o0 { "mneg" } else { "mul" };
                return lens_mk(
                    format!("{lens_mn} {}, {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf)),
                    LensFlow::LensFallthrough,
                );
            }
            let lens_mn = if lens_o0 { "msub" } else { "madd" };
            return lens_mk(
                format!(
                    "{lens_mn} {}, {}, {}, {}",
                    lens_sx(lens_rd, lens_sf),
                    lens_sx(lens_rn, lens_sf),
                    lens_sx(lens_rm, lens_sf),
                    lens_sx(lens_ra, lens_sf)
                ),
                LensFlow::LensFallthrough,
            );
        }
        let lens_mn = match (lens_op31, lens_o0) {
            (0b001, false) => "smaddl",
            (0b001, true) => "smsubl",
            (0b101, false) => "umaddl",
            (0b101, true) => "umsubl",
            (0b010, false) => "smulh",
            (0b110, false) => "umulh",
            _ => return lens_unk(),
        };
        if lens_mn == "smulh" || lens_mn == "umulh" {
            return lens_mk(
                format!("{lens_mn} {}, {}, {}", lens_sx(lens_rd, true), lens_sx(lens_rn, true), lens_sx(lens_rm, true)),
                LensFlow::LensFallthrough,
            );
        }
        if lens_ra == 31 && (lens_mn == "smaddl" || lens_mn == "umaddl") {
            let lens_alias = if lens_mn == "smaddl" { "smull" } else { "umull" };
            return lens_mk(
                format!(
                    "{lens_alias} {}, {}, {}",
                    lens_sx(lens_rd, true),
                    lens_sx(lens_rn, false),
                    lens_sx(lens_rm, false)
                ),
                LensFlow::LensFallthrough,
            );
        }
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, {}, {}",
                lens_sx(lens_rd, true),
                lens_sx(lens_rn, false),
                lens_sx(lens_rm, false),
                lens_sx(lens_ra, true)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x7FE0_0000 == 0x1AC0_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opc = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match lens_opc {
            0b000010 => "udiv",
            0b000011 => "sdiv",
            0b001000 => "lsl",
            0b001001 => "lsr",
            0b001010 => "asr",
            0b001011 => "ror",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!("{lens_mn} {}, {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1F80_0000 == 0x1300_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_opc = (lens_raw >> 29) & 0x3;
        let lens_immr = (lens_raw >> 16) & 0x3f;
        let lens_imms = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_width = if lens_sf { 64 } else { 32 };
        if lens_opc == 0b10 {
            if lens_imms != lens_width - 1 && lens_imms + 1 == lens_immr {
                let lens_sh = lens_width - lens_immr;
                return lens_mk(
                    format!("lsl {}, {}, #{lens_sh}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
                    LensFlow::LensFallthrough,
                );
            }
            if lens_imms == lens_width - 1 {
                return lens_mk(
                    format!("lsr {}, {}, #{lens_immr}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
                    LensFlow::LensFallthrough,
                );
            }
            if lens_immr == 0 && lens_imms == 7 {
                return lens_mk(
                    format!("uxtb {}, {}", lens_sx(lens_rd, false), lens_sx(lens_rn, false)),
                    LensFlow::LensFallthrough,
                );
            }
            if lens_immr == 0 && lens_imms == 15 {
                return lens_mk(
                    format!("uxth {}, {}", lens_sx(lens_rd, false), lens_sx(lens_rn, false)),
                    LensFlow::LensFallthrough,
                );
            }
            let lens_w = lens_imms.wrapping_sub(lens_immr).wrapping_add(1);
            return lens_mk(
                format!("ubfx {}, {}, #{lens_immr}, #{lens_w}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        if lens_opc == 0b00 {
            if lens_imms == lens_width - 1 {
                return lens_mk(
                    format!("asr {}, {}, #{lens_immr}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
                    LensFlow::LensFallthrough,
                );
            }
            if lens_immr == 0 && lens_imms == 7 {
                return lens_mk(
                    format!("sxtb {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, false)),
                    LensFlow::LensFallthrough,
                );
            }
            if lens_immr == 0 && lens_imms == 15 {
                return lens_mk(
                    format!("sxth {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, false)),
                    LensFlow::LensFallthrough,
                );
            }
            if lens_immr == 0 && lens_imms == 31 && lens_sf {
                return lens_mk(
                    format!("sxtw {}, {}", lens_sx(lens_rd, true), lens_sx(lens_rn, false)),
                    LensFlow::LensFallthrough,
                );
            }
            let lens_w = lens_imms.wrapping_sub(lens_immr).wrapping_add(1);
            return lens_mk(
                format!("sbfx {}, {}, #{lens_immr}, #{lens_w}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        return lens_mk(
            format!("bfm {}, {}, #{lens_immr}, #{lens_imms}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x3F00_0000 == 0x3800_0000 {
        let lens_size = (lens_raw >> 30) & 0x3;
        let lens_opc = (lens_raw >> 22) & 0x3;
        let lens_imm9 = lens_sext((lens_raw >> 12) & 0x1ff, 9);
        let lens_idx = (lens_raw >> 10) & 0x3;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let (lens_base_mn, lens_sf) = match (lens_size, lens_opc) {
            (0, 0) => ("strb", false),
            (0, 1) => ("ldrb", false),
            (0, 2) => ("ldrsb", true),
            (0, 3) => ("ldrsb", false),
            (1, 0) => ("strh", false),
            (1, 1) => ("ldrh", false),
            (1, 2) => ("ldrsh", true),
            (1, 3) => ("ldrsh", false),
            (2, 0) => ("str", false),
            (2, 1) => ("ldr", false),
            (2, 2) => ("ldrsw", true),
            (3, 0) => ("str", true),
            (3, 1) => ("ldr", true),
            _ => return lens_unk(),
        };
        let lens_mn = if lens_idx == 0 {
            match lens_base_mn {
                "str" => "stur",
                "ldr" => "ldur",
                "strb" => "sturb",
                "ldrb" => "ldurb",
                "strh" => "sturh",
                "ldrh" => "ldurh",
                "ldrsb" => "ldursb",
                "ldrsh" => "ldursh",
                "ldrsw" => "ldursw",
                lens_other => lens_other,
            }
        } else {
            lens_base_mn
        };
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_off = lens_simm(lens_imm9);
        let lens_mem = match lens_idx {
            0b01 => format!("[{lens_base}], #{lens_off}"),
            0b11 => format!("[{lens_base}, #{lens_off}]!"),
            _ if lens_imm9 == 0 => format!("[{lens_base}]"),
            _ => format!("[{lens_base}, #{lens_off}]"),
        };
        return lens_mk(format!("{lens_mn} {}, {lens_mem}", lens_sx(lens_rt, lens_sf)), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x3E00_0000 == 0x2800_0000 {
        let lens_opc = (lens_raw >> 30) & 0x3;
        let lens_load = lens_raw & 0x0040_0000 != 0;
        let lens_imm7 = (lens_raw >> 15) & 0x7f;
        let lens_rt2 = (lens_raw >> 10) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let lens_ldpsw = lens_opc == 1 && lens_load;
        let lens_sf = lens_opc == 2 || lens_ldpsw;
        let lens_scale = if lens_opc == 2 { 3 } else { 2 };
        let lens_off = lens_sext(lens_imm7, 7) << lens_scale;
        let lens_mn = if lens_ldpsw {
            "ldpsw"
        } else if lens_load {
            "ldp"
        } else {
            "stp"
        };
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_mem = if lens_off == 0 {
            format!("[{lens_base}]")
        } else {
            format!("[{lens_base}, #{}]", lens_simm(lens_off))
        };
        return lens_mk(
            format!("{lens_mn} {}, {}, {lens_mem}", lens_sx(lens_rt, lens_sf), lens_sx(lens_rt2, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1F80_0000 == 0x1200_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_opc = (lens_raw >> 29) & 0x3;
        let lens_n = (lens_raw >> 22) & 1;
        let lens_immr = (lens_raw >> 16) & 0x3f;
        let lens_imms = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_datasize = if lens_sf { 64 } else { 32 };
        if !lens_sf && lens_n == 1 {
            return lens_unk();
        }
        let lens_imm = match lens_decode_bitmask(lens_n, lens_imms, lens_immr, lens_datasize) {
            Some(lens_v) => lens_v,
            None => return lens_unk(),
        };
        if lens_opc == 0b01 && lens_rn == 31 {
            return lens_mk(
                format!("mov {}, #0x{lens_imm:x}", lens_reg(lens_rd, lens_sf, true)),
                LensFlow::LensFallthrough,
            );
        }
        if lens_opc == 0b11 && lens_rd == 31 {
            return lens_mk(format!("tst {}, #0x{lens_imm:x}", lens_sx(lens_rn, lens_sf)), LensFlow::LensFallthrough);
        }
        let lens_mn = match lens_opc {
            0b00 => "and",
            0b01 => "orr",
            0b10 => "eor",
            _ => "ands",
        };
        let lens_rd_s = lens_reg(lens_rd, lens_sf, lens_opc != 0b11);
        return lens_mk(
            format!("{lens_mn} {lens_rd_s}, {}, #0x{lens_imm:x}", lens_sx(lens_rn, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1FE0_0000 == 0x0B20_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_sub = lens_raw & 0x4000_0000 != 0;
        let lens_set = lens_raw & 0x2000_0000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_option = (lens_raw >> 13) & 0x7;
        let lens_imm3 = (lens_raw >> 10) & 0x7;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_ext = [
            "uxtb", "uxth", "uxtw", "uxtx", "sxtb", "sxth", "sxtw", "sxtx",
        ][lens_option as usize];
        let lens_rm_x = lens_option == 0b011 || lens_option == 0b111;
        let lens_extstr = if lens_imm3 == 0 {
            format!(", {lens_ext}")
        } else {
            format!(", {lens_ext} #{lens_imm3}")
        };
        if lens_set && lens_rd == 31 {
            let lens_mn = if lens_sub { "cmp" } else { "cmn" };
            let lens_kind = if lens_sub { LensFlagKind::LensCmp } else { LensFlagKind::LensCmn };
            let lens_b = format!("{}{lens_extstr}", lens_sx(lens_rm, lens_rm_x));
            return lens_mk(
                format!("{lens_mn} {}, {lens_b}", lens_reg(lens_rn, lens_sf, true)),
                LensFlow::LensFallthrough,
            )
            .lens_with_flags(lens_reg(lens_rn, lens_sf, true), lens_b, lens_kind);
        }
        let lens_mn = match (lens_sub, lens_set) {
            (false, false) => "add",
            (false, true) => "adds",
            (true, false) => "sub",
            (true, true) => "subs",
        };
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, {}{lens_extstr}",
                lens_reg(lens_rd, lens_sf, !lens_set),
                lens_reg(lens_rn, lens_sf, true),
                lens_sx(lens_rm, lens_rm_x)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x3FE0_0000 == 0x3A40_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_op = lens_raw & 0x4000_0000 != 0;
        let lens_imm_form = (lens_raw >> 11) & 1 == 1;
        let lens_rm_or_imm = (lens_raw >> 16) & 0x1f;
        let lens_cond = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_nzcv = lens_raw & 0xf;
        let lens_mn = if lens_op { "ccmp" } else { "ccmn" };
        let lens_b = if lens_imm_form {
            format!("#0x{lens_rm_or_imm:x}")
        } else {
            lens_sx(lens_rm_or_imm, lens_sf)
        };
        return lens_mk(
            format!(
                "{lens_mn} {}, {lens_b}, #0x{lens_nzcv:x}, {}",
                lens_sx(lens_rn, lens_sf),
                LENS_COND[lens_cond as usize]
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x7FE0_0000 == 0x5AC0_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_opcode = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match lens_opcode {
            0b000000 => "rbit",
            0b000001 => "rev16",
            0b000010 => {
                if lens_sf {
                    "rev32"
                } else {
                    "rev"
                }
            }
            0b000011 => "rev",
            0b000100 => "clz",
            0b000101 => "cls",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!("{lens_mn} {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF00_0000 == 0xD400_0000 {
        let lens_opc = (lens_raw >> 21) & 0x7;
        let lens_ll = lens_raw & 0x3;
        let lens_imm = (lens_raw >> 5) & 0xffff;
        let lens_mn = match (lens_opc, lens_ll) {
            (0b001, 0b00) => "brk",
            (0b010, 0b00) => "hlt",
            (0b101, 0b01) => "dcps1",
            (0b101, 0b10) => "dcps2",
            (0b101, 0b11) => "dcps3",
            _ => return lens_unk(),
        };
        return lens_mk(format!("{lens_mn} #0x{lens_imm:x}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0xFFFF_0000 == 0x0000_0000 {
        return lens_mk(format!("udf #0x{:x}", lens_raw & 0xffff), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x3F00_0000 == 0x0800_0000 {
        let lens_size = (lens_raw >> 30) & 0x3;
        let lens_o2 = (lens_raw >> 23) & 1;
        let lens_l = (lens_raw >> 22) & 1;
        let lens_o1 = (lens_raw >> 21) & 1;
        let lens_rs = (lens_raw >> 16) & 0x1f;
        let lens_o0 = (lens_raw >> 15) & 1;
        let lens_rt2 = (lens_raw >> 10) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let lens_suf = match lens_size {
            0 => "b",
            1 => "h",
            _ => "",
        };
        let lens_sf = lens_size == 3;
        let lens_base = lens_reg(lens_rn, true, true);
        if lens_o1 == 1 {
            let lens_mn = match (lens_l, lens_o0) {
                (0, 0) => "stxp",
                (0, 1) => "stlxp",
                (1, 0) => "ldxp",
                _ => "ldaxp",
            };
            if lens_l == 0 {
                return lens_mk(
                    format!(
                        "{lens_mn} {}, {}, {}, [{lens_base}]",
                        lens_reg(lens_rs, false, false),
                        lens_sx(lens_rt, lens_sf),
                        lens_sx(lens_rt2, lens_sf)
                    ),
                    LensFlow::LensFallthrough,
                );
            }
            return lens_mk(
                format!("{lens_mn} {}, {}, [{lens_base}]", lens_sx(lens_rt, lens_sf), lens_sx(lens_rt2, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        if lens_o2 == 0 {
            let lens_mn = match (lens_l, lens_o0) {
                (0, 0) => format!("stxr{lens_suf}"),
                (0, 1) => format!("stlxr{lens_suf}"),
                (1, 0) => format!("ldxr{lens_suf}"),
                _ => format!("ldaxr{lens_suf}"),
            };
            if lens_l == 0 {
                return lens_mk(
                    format!("{lens_mn} {}, {}, [{lens_base}]", lens_reg(lens_rs, false, false), lens_sx(lens_rt, lens_sf),),
                    LensFlow::LensFallthrough,
                );
            }
            return lens_mk(format!("{lens_mn} {}, [{lens_base}]", lens_sx(lens_rt, lens_sf)), LensFlow::LensFallthrough);
        }
        let lens_mn = match (lens_l, lens_o0) {
            (0, 0) => format!("stllr{lens_suf}"),
            (0, 1) => format!("stlr{lens_suf}"),
            (1, 0) => format!("ldlar{lens_suf}"),
            _ => format!("ldar{lens_suf}"),
        };
        return lens_mk(format!("{lens_mn} {}, [{lens_base}]", lens_sx(lens_rt, lens_sf)), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x3F00_0000 == 0x3D00_0000 {
        let lens_size = (lens_raw >> 30) & 0x3;
        let lens_opc = (lens_raw >> 22) & 0x3;
        let lens_imm12 = (lens_raw >> 10) & 0xFFF;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let (lens_letter, lens_scale) = lens_simd_ls_size(lens_size, lens_opc);
        let lens_load = (lens_opc & 1) == 1;
        let lens_mn = if lens_load { "ldr" } else { "str" };
        let lens_off = lens_imm12 << lens_scale;
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_mem = if lens_off == 0 {
            format!("[{lens_base}]")
        } else {
            format!("[{lens_base}, #0x{lens_off:x}]")
        };
        return lens_mk(format!("{lens_mn} {lens_letter}{lens_rt}, {lens_mem}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x3F00_0000 == 0x3C00_0000 {
        let lens_size = (lens_raw >> 30) & 0x3;
        let lens_opc = (lens_raw >> 22) & 0x3;
        let lens_imm9 = lens_sext((lens_raw >> 12) & 0x1ff, 9);
        let lens_idx = (lens_raw >> 10) & 0x3;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let (lens_letter, lens__scale) = lens_simd_ls_size(lens_size, lens_opc);
        let lens_load = (lens_opc & 1) == 1;
        let lens_mn = if lens_idx == 0 {
            if lens_load {
                "ldur"
            } else {
                "stur"
            }
        } else if lens_load {
            "ldr"
        } else {
            "str"
        };
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_off = lens_simm(lens_imm9);
        let lens_mem = match lens_idx {
            0b01 => format!("[{lens_base}], #{lens_off}"),
            0b11 => format!("[{lens_base}, #{lens_off}]!"),
            _ if lens_imm9 == 0 => format!("[{lens_base}]"),
            _ => format!("[{lens_base}, #{lens_off}]"),
        };
        return lens_mk(format!("{lens_mn} {lens_letter}{lens_rt}, {lens_mem}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x3E00_0000 == 0x2C00_0000 {
        let lens_opc = (lens_raw >> 30) & 0x3;
        let lens_load = lens_raw & 0x0040_0000 != 0;
        let lens_imm7 = (lens_raw >> 15) & 0x7f;
        let lens_rt2 = (lens_raw >> 10) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let (lens_letter, lens_scale) = match lens_opc {
            0 => ('s', 2),
            1 => ('d', 3),
            _ => ('q', 4),
        };
        let lens_off = lens_sext(lens_imm7, 7) << lens_scale;
        let lens_mn = if lens_load { "ldp" } else { "stp" };
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_mem = if lens_off == 0 {
            format!("[{lens_base}]")
        } else {
            format!("[{lens_base}, #{}]", lens_simm(lens_off))
        };
        return lens_mk(
            format!("{lens_mn} {lens_letter}{lens_rt}, {lens_letter}{lens_rt2}, {lens_mem}"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF20_FC00 == 0x1E20_2000 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_opcode2 = lens_raw & 0x1f;
        let lens_e = lens_opcode2 & 0x10 != 0;
        let lens_zero = lens_opcode2 & 0x08 != 0;
        let lens_mn = if lens_e { "fcmpe" } else { "fcmp" };
        if lens_zero {
            return lens_mk(
                format!("{lens_mn} {}, #0.0", lens_fp_reg(lens_ftype, lens_rn)),
                LensFlow::LensFallthrough,
            );
        }
        return lens_mk(
            format!("{lens_mn} {}, {}", lens_fp_reg(lens_ftype, lens_rn), lens_fp_reg(lens_ftype, lens_rm)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF20_7C00 == 0x1E20_4000 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_opcode = (lens_raw >> 15) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match lens_opcode {
            0b000000 => "fmov",
            0b000001 => "fabs",
            0b000010 => "fneg",
            0b000011 => "fsqrt",
            0b000100 | 0b000101 | 0b000111 => "fcvt",
            0b001000 => "frintn",
            0b001001 => "frintp",
            0b001010 => "frintm",
            0b001011 => "frintz",
            0b001100 => "frinta",
            0b001110 => "frintx",
            0b001111 => "frinti",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!("{lens_mn} {}, {}", lens_fp_reg(lens_ftype, lens_rd), lens_fp_reg(lens_ftype, lens_rn)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF20_0C00 == 0x1E20_0800 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opcode = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match lens_opcode {
            0 => "fmul",
            1 => "fdiv",
            2 => "fadd",
            3 => "fsub",
            4 => "fmax",
            5 => "fmin",
            6 => "fmaxnm",
            7 => "fminnm",
            8 => "fnmul",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, {}",
                lens_fp_reg(lens_ftype, lens_rd),
                lens_fp_reg(lens_ftype, lens_rn),
                lens_fp_reg(lens_ftype, lens_rm)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF20_0C00 == 0x1E20_0C00 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_cond = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        return lens_mk(
            format!(
                "fcsel {}, {}, {}, {}",
                lens_fp_reg(lens_ftype, lens_rd),
                lens_fp_reg(lens_ftype, lens_rn),
                lens_fp_reg(lens_ftype, lens_rm),
                LENS_COND[lens_cond as usize]
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF20_0C00 == 0x1E20_0400 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_cond = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_e = lens_raw & 0x10 != 0;
        let lens_nzcv = lens_raw & 0xf;
        let lens_mn = if lens_e { "fccmpe" } else { "fccmp" };
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, #0x{lens_nzcv:x}, {}",
                lens_fp_reg(lens_ftype, lens_rn),
                lens_fp_reg(lens_ftype, lens_rm),
                LENS_COND[lens_cond as usize]
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFF20_1FE0 == 0x1E20_1000 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_imm8 = (lens_raw >> 13) & 0xff;
        let lens_rd = lens_raw & 0x1f;
        let lens_val = lens_vfp_expand_imm(lens_imm8);
        return lens_mk(
            format!("fmov {}, #{lens_val:.8}", lens_fp_reg(lens_ftype, lens_rd)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x7F20_FC00 == 0x1E20_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_rmode = (lens_raw >> 19) & 0x3;
        let lens_opcode = (lens_raw >> 16) & 0x7;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_rmode, lens_opcode) {
            (0, 0b010) => "scvtf",
            (0, 0b011) => "ucvtf",
            (0, 0b000) => "fcvtns",
            (0, 0b001) => "fcvtnu",
            (1, 0b000) => "fcvtps",
            (1, 0b001) => "fcvtpu",
            (2, 0b000) => "fcvtms",
            (2, 0b001) => "fcvtmu",
            (3, 0b000) => "fcvtzs",
            (3, 0b001) => "fcvtzu",
            (0, 0b100) => "fcvtas",
            (0, 0b101) => "fcvtau",
            (0, 0b110) => "fmov",
            (0, 0b111) => "fmov",
            _ => return lens_unk(),
        };
        let (lens_int_to_fp, lens_fp_to_int) = match lens_opcode {
            0b010 | 0b011 | 0b111 => (true, false),
            0b110 => (false, true),
            _ => (false, true),
        };
        if lens_int_to_fp {
            return lens_mk(
                format!("{lens_mn} {}, {}", lens_fp_reg(lens_ftype, lens_rd), lens_sx(lens_rn, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        if lens_fp_to_int {
            return lens_mk(
                format!("{lens_mn} {}, {}", lens_sx(lens_rd, lens_sf), lens_fp_reg(lens_ftype, lens_rn)),
                LensFlow::LensFallthrough,
            );
        }
        return lens_unk();
    }

    if lens_raw & 0xFF00_0000 == 0x1F00_0000 {
        let lens_ftype = (lens_raw >> 22) & 0x3;
        let lens_o1 = (lens_raw >> 21) & 1;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_o0 = (lens_raw >> 15) & 1;
        let lens_ra = (lens_raw >> 10) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_o1, lens_o0) {
            (0, 0) => "fmadd",
            (0, 1) => "fmsub",
            (1, 0) => "fnmadd",
            _ => "fnmsub",
        };
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, {}, {}",
                lens_fp_reg(lens_ftype, lens_rd),
                lens_fp_reg(lens_ftype, lens_rn),
                lens_fp_reg(lens_ftype, lens_rm),
                lens_fp_reg(lens_ftype, lens_ra)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x7FA0_0000 == 0x1380_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_imms = (lens_raw >> 10) & 0x3f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        if lens_rn == lens_rm {
            return lens_mk(
                format!("ror {}, {}, #0x{lens_imms:x}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        return lens_mk(
            format!(
                "extr {}, {}, {}, #0x{lens_imms:x}",
                lens_sx(lens_rd, lens_sf),
                lens_sx(lens_rn, lens_sf),
                lens_sx(lens_rm, lens_sf)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x1FE0_FC00 == 0x1A00_0000 {
        let lens_sf = lens_raw & 0x8000_0000 != 0;
        let lens_sub = lens_raw & 0x4000_0000 != 0;
        let lens_set = lens_raw & 0x2000_0000 != 0;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        if lens_sub && lens_rn == 31 {
            let lens_mn = if lens_set { "ngcs" } else { "ngc" };
            return lens_mk(
                format!("{lens_mn} {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rm, lens_sf)),
                LensFlow::LensFallthrough,
            );
        }
        let lens_mn = match (lens_sub, lens_set) {
            (false, false) => "adc",
            (false, true) => "adcs",
            (true, false) => "sbc",
            (true, true) => "sbcs",
        };
        return lens_mk(
            format!("{lens_mn} {}, {}, {}", lens_sx(lens_rd, lens_sf), lens_sx(lens_rn, lens_sf), lens_sx(lens_rm, lens_sf)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x9F20_0400 == 0x0E20_0400 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_u = (lens_raw >> 29) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_hi = (lens_raw >> 23) & 1;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opcode = (lens_raw >> 11) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        if lens_u == 0 && lens_opcode == 0x03 && lens_size == 2 && lens_rm == lens_rn {
            let lens_a = lens_varr(0, lens_q);
            return lens_mk(format!("mov v{lens_rd}.{lens_a}, v{lens_rn}.{lens_a}"), LensFlow::LensFallthrough);
        }
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x03) => ["and", "bic", "orr", "orn"][lens_size as usize],
            (1, 0x03) => ["eor", "bsl", "bit", "bif"][lens_size as usize],
            (0, 0x10) => "add",
            (1, 0x10) => "sub",
            (0, 0x11) => "cmtst",
            (1, 0x11) => "cmeq",
            (0, 0x06) => "cmgt",
            (1, 0x06) => "cmhi",
            (0, 0x07) => "cmge",
            (1, 0x07) => "cmhs",
            (0, 0x13) => "mul",
            (1, 0x13) => "pmul",
            (0, 0x0c) => "smax",
            (1, 0x0c) => "umax",
            (0, 0x0d) => "smin",
            (1, 0x0d) => "umin",
            (0, 0x01) => "sqadd",
            (1, 0x01) => "uqadd",
            (0, 0x0e) => "sabd",
            (1, 0x0e) => "uabd",
            (0, 0x08) => "sshl",
            (1, 0x08) => "ushl",
            (0, 0x12) => "mla",
            (1, 0x12) => "mls",
            (0, 0x18) => {
                if lens_hi == 0 {
                    "fmaxnm"
                } else {
                    "fminnm"
                }
            }
            (0, 0x19) => {
                if lens_hi == 0 {
                    "fmla"
                } else {
                    "fmls"
                }
            }
            (0, 0x1a) => {
                if lens_hi == 0 {
                    "fadd"
                } else {
                    "fsub"
                }
            }
            (0, 0x1b) => "fmulx",
            (0, 0x1c) => "fcmeq",
            (0, 0x1e) => {
                if lens_hi == 0 {
                    "fmax"
                } else {
                    "fmin"
                }
            }
            (0, 0x1f) => {
                if lens_hi == 0 {
                    "frecps"
                } else {
                    "frsqrts"
                }
            }
            (1, 0x18) => {
                if lens_hi == 0 {
                    "fmaxnmp"
                } else {
                    "fminnmp"
                }
            }
            (1, 0x1a) => {
                if lens_hi == 0 {
                    "faddp"
                } else {
                    "fabd"
                }
            }
            (1, 0x1b) => "fmul",
            (1, 0x1c) => {
                if lens_hi == 0 {
                    "fcmge"
                } else {
                    "fcmgt"
                }
            }
            (1, 0x1d) => {
                if lens_hi == 0 {
                    "facge"
                } else {
                    "facgt"
                }
            }
            (1, 0x1e) => {
                if lens_hi == 0 {
                    "fmaxp"
                } else {
                    "fminp"
                }
            }
            (1, 0x1f) => "fdiv",
            _ => return lens_unk(),
        };
        let lens_arr = if lens_opcode >= 0x18 {
            lens_farr(lens_hi, lens_q)
        } else {
            lens_varr(lens_size, lens_q)
        };
        return lens_mk(
            format!("{lens_mn} v{lens_rd}.{lens_arr}, v{lens_rn}.{lens_arr}, v{lens_rm}.{lens_arr}"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x9FE0_0400 == 0x0E00_0400 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_op = (lens_raw >> 29) & 1;
        let lens_imm5 = (lens_raw >> 16) & 0x1f;
        let lens_imm4 = (lens_raw >> 11) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_size = lens_imm5.trailing_zeros().min(3);
        let lens_arr = lens_varr(lens_size, lens_q);
        if lens_op == 1 {
            return lens_mk(
                format!("mov v{lens_rd}.{lens_arr}[?], v{lens_rn}.{lens_arr}[?]"),
                LensFlow::LensFallthrough,
            );
        }
        return match lens_imm4 {
            0b0000 => lens_mk(
                format!("dup v{lens_rd}.{lens_arr}, v{lens_rn}.{lens_arr}[?]"),
                LensFlow::LensFallthrough,
            ),
            0b0001 => {
                let lens_gp = lens_size == 3;
                lens_mk(
                    format!("dup v{lens_rd}.{lens_arr}, {}", lens_sx(lens_rn, lens_gp)),
                    LensFlow::LensFallthrough,
                )
            }
            0b0011 => lens_mk(
                format!("mov v{lens_rd}.{lens_arr}[?], {}", lens_sx(lens_rn, lens_size == 3)),
                LensFlow::LensFallthrough,
            ),
            0b0101 => lens_mk(
                format!("smov {}, v{lens_rn}.{lens_arr}[?]", lens_sx(lens_rd, lens_q == 1)),
                LensFlow::LensFallthrough,
            ),
            0b0111 => {
                let lens_mn = if lens_size >= 2 { "mov" } else { "umov" };
                lens_mk(
                    format!("{lens_mn} {}, v{lens_rn}.{lens_arr}[?]", lens_sx(lens_rd, lens_q == 1)),
                    LensFlow::LensFallthrough,
                )
            }
            _ => lens_unk(),
        };
    }

    if lens_raw & 0xBF20_8C00 == 0x0E00_0800 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opcode = (lens_raw >> 12) & 0x7;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match lens_opcode {
            0b001 => "uzp1",
            0b010 => "trn1",
            0b011 => "zip1",
            0b101 => "uzp2",
            0b110 => "trn2",
            0b111 => "zip2",
            _ => return lens_unk(),
        };
        let lens_a = lens_varr(lens_size, lens_q);
        return lens_mk(
            format!("{lens_mn} v{lens_rd}.{lens_a}, v{lens_rn}.{lens_a}, v{lens_rm}.{lens_a}"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xBF20_9C00 == 0x0E00_0000 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_op = (lens_raw >> 12) & 1;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = if lens_op == 1 { "tbx" } else { "tbl" };
        let lens_a = lens_varr(0, lens_q);
        return lens_mk(
            format!("{lens_mn} v{lens_rd}.{lens_a}, {{v{lens_rn}.16b}}, v{lens_rm}.{lens_a}"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x9F3E_0C00 == 0x0E30_0800 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_u = (lens_raw >> 29) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_opcode = (lens_raw >> 12) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x03) => "saddlv",
            (1, 0x03) => "uaddlv",
            (0, 0x0a) => "smaxv",
            (1, 0x0a) => "umaxv",
            (0, 0x1a) => "sminv",
            (1, 0x1a) => "uminv",
            (_, 0x1b) => "addv",
            (1, 0x0c) => "fmaxnmv",
            (1, 0x0f) => "fmaxv",
            (1, 0x2c) => "fminnmv",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!("{lens_mn} v{lens_rd}, v{lens_rn}.{}", lens_varr(lens_size, lens_q)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x9F3E_0C00 == 0x0E20_0800 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_u = (lens_raw >> 29) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_hi = (lens_raw >> 23) & 1;
        let lens_opcode = (lens_raw >> 12) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x00) => "rev64",
            (1, 0x00) => "rev32",
            (0, 0x01) => "rev16",
            (0, 0x04) => "cls",
            (1, 0x04) => "clz",
            (0, 0x05) => "cnt",
            (1, 0x05) => {
                if lens_size == 1 {
                    "rbit"
                } else {
                    "mvn"
                }
            }
            (0, 0x06) => "sadalp",
            (1, 0x06) => "uadalp",
            (0, 0x0b) => "abs",
            (1, 0x0b) => "neg",
            (0, 0x08) => "cmgt",
            (1, 0x08) => "cmge",
            (0, 0x09) => "cmeq",
            (1, 0x09) => "cmle",
            (0, 0x0a) => "cmlt",
            (0, 0x12) => "xtn",
            (1, 0x12) => "sqxtun",
            (0, 0x14) => "sqxtn",
            (1, 0x14) => "uqxtn",
            (0, 0x13) => "shll",
            (0, 0x0c) => "fcmgt",
            (1, 0x0c) => "fcmge",
            (0, 0x0d) => "fcmeq",
            (1, 0x0d) => "fcmle",
            (0, 0x0e) => "fcmlt",
            (0, 0x0f) => "fabs",
            (1, 0x0f) => "fneg",
            (0, 0x16) => "fcvtn",
            (0, 0x17) => "fcvtl",
            (0, 0x1a) => {
                if lens_hi == 1 {
                    "fcvtps"
                } else {
                    "fcvtns"
                }
            }
            (1, 0x1a) => {
                if lens_hi == 1 {
                    "fcvtpu"
                } else {
                    "fcvtnu"
                }
            }
            (0, 0x1b) => {
                if lens_hi == 1 {
                    "fcvtzs"
                } else {
                    "fcvtms"
                }
            }
            (1, 0x1b) => {
                if lens_hi == 1 {
                    "fcvtzu"
                } else {
                    "fcvtmu"
                }
            }
            (0, 0x1d) => {
                if lens_hi == 1 {
                    "frecpe"
                } else {
                    "scvtf"
                }
            }
            (1, 0x1d) => {
                if lens_hi == 1 {
                    "frsqrte"
                } else {
                    "ucvtf"
                }
            }
            (1, 0x1f) => "fsqrt",
            (0, 0x18) => {
                if lens_hi == 1 {
                    "frintp"
                } else {
                    "frintn"
                }
            }
            (0, 0x19) => {
                if lens_hi == 1 {
                    "frintz"
                } else {
                    "frintm"
                }
            }
            _ => return lens_unk(),
        };
        let lens_two = lens_q == 1 && matches!(lens_opcode, 0x12 | 0x13 | 0x14 | 0x16 | 0x17);
        let lens_mn = if lens_two {
            match lens_mn {
                "xtn" => "xtn2",
                "sqxtun" => "sqxtun2",
                "sqxtn" => "sqxtn2",
                "uqxtn" => "uqxtn2",
                "shll" => "shll2",
                "fcvtn" => "fcvtn2",
                "fcvtl" => "fcvtl2",
                lens_other => lens_other,
            }
        } else {
            lens_mn
        };
        let lens_arr = if (0x0c..=0x1f).contains(&lens_opcode) {
            lens_farr(lens_hi, lens_q)
        } else {
            lens_varr(lens_size, lens_q)
        };
        let lens_is_cmp0 = (0x08..=0x0a).contains(&lens_opcode) || (0x0c..=0x0e).contains(&lens_opcode);
        if lens_is_cmp0 {
            let lens_zero = if lens_opcode >= 0x0c { ", #0.0" } else { ", #0" };
            return lens_mk(
                format!("{lens_mn} v{lens_rd}.{lens_arr}, v{lens_rn}.{lens_arr}{lens_zero}"),
                LensFlow::LensFallthrough,
            );
        }
        return lens_mk(format!("{lens_mn} v{lens_rd}.{lens_arr}, v{lens_rn}.{lens_arr}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0xBF20_8400 == 0x2E00_0000 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_imm4 = (lens_raw >> 11) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_a = lens_varr(0, lens_q);
        return lens_mk(
            format!("ext v{lens_rd}.{lens_a}, v{lens_rn}.{lens_a}, v{lens_rm}.{lens_a}, #{lens_imm4}"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xBF00_0000 == 0x0C00_0000 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_post = (lens_raw >> 23) & 1;
        let lens_l = (lens_raw >> 22) & 1;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opcode = (lens_raw >> 12) & 0xf;
        let lens_size = (lens_raw >> 10) & 3;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let (lens_stem, lens_regs) = match lens_opcode {
            0b0000 => ("4", 4),
            0b0010 => ("1", 4),
            0b0100 => ("3", 3),
            0b0110 => ("1", 3),
            0b0111 => ("1", 1),
            0b1000 => ("2", 2),
            0b1010 => ("1", 2),
            _ => return lens_unk(),
        };
        let lens_mn = if lens_l == 1 {
            format!("ld{lens_stem}")
        } else {
            format!("st{lens_stem}")
        };
        let lens_a = lens_varr(lens_size, lens_q);
        let lens_list = (0..lens_regs)
            .map(|lens_i| format!("v{}.{lens_a}", (lens_rt + lens_i) & 0x1f))
            .collect::<Vec<_>>()
            .join(", ");
        let lens_base = lens_reg(lens_rn, true, true);
        let lens_mem = if lens_post == 0 {
            format!("[{lens_base}]")
        } else if lens_rm == 31 {
            let lens_bytes = lens_regs * if lens_q == 1 { 16 } else { 8 };
            format!("[{lens_base}], #{lens_bytes}")
        } else {
            format!("[{lens_base}], {}", lens_sx(lens_rm, true))
        };
        return lens_mk(format!("{lens_mn} {{{lens_list}}}, {lens_mem}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0xFFFF_F0FF == 0xD503_305F {
        let lens_crm = (lens_raw >> 8) & 0xf;
        if lens_crm == 0xf {
            return lens_mk("clrex".to_string(), LensFlow::LensFallthrough);
        }
        return lens_mk(format!("clrex #0x{lens_crm:x}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x9F20_0C00 == 0x0E20_0000 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_u = (lens_raw >> 29) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opcode = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x0) => "saddl",
            (1, 0x0) => "uaddl",
            (0, 0x1) => "saddw",
            (1, 0x1) => "uaddw",
            (0, 0x2) => "ssubl",
            (1, 0x2) => "usubl",
            (0, 0x3) => "ssubw",
            (1, 0x3) => "usubw",
            (0, 0x4) => "addhn",
            (1, 0x4) => "raddhn",
            (0, 0x5) => "sabal",
            (1, 0x5) => "uabal",
            (0, 0x6) => "subhn",
            (1, 0x6) => "rsubhn",
            (0, 0x7) => "sabdl",
            (1, 0x7) => "uabdl",
            (0, 0x8) => "smlal",
            (1, 0x8) => "umlal",
            (0, 0x9) => "sqdmlal",
            (0, 0xa) => "smlsl",
            (1, 0xa) => "umlsl",
            (0, 0xb) => "sqdmlsl",
            (0, 0xc) => "smull",
            (1, 0xc) => "umull",
            (0, 0xd) => "sqdmull",
            (0, 0xe) => "pmull",
            _ => return lens_unk(),
        };
        let lens_two = if lens_q == 1 { "2" } else { "" };
        let lens_da = lens_varr(lens_size + 1, 1);
        let lens_sa = lens_varr(lens_size, lens_q);
        return lens_mk(
            format!("{lens_mn}{lens_two} v{lens_rd}.{lens_da}, v{lens_rn}.{lens_sa}, v{lens_rm}.{lens_sa}"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0x9F00_0400 == 0x0F00_0000 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_u = (lens_raw >> 29) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_opcode = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (_, 0x0) => "mla",
            (_, 0x4) => "mls",
            (_, 0x8) => "mul",
            (0, 0x1) => "fmla",
            (0, 0x5) => "fmls",
            (0, 0x9) => "fmul",
            (1, 0x9) => "fmulx",
            (0, 0x2) => "smlal",
            (1, 0x2) => "umlal",
            (0, 0x6) => "smlsl",
            (1, 0x6) => "umlsl",
            (0, 0xa) => "smull",
            (1, 0xa) => "umull",
            (0, 0x3) => "sqdmlal",
            (0, 0x7) => "sqdmlsl",
            (0, 0xb) => "sqdmull",
            (0, 0xc) => "sqdmulh",
            (0, 0xd) => "sqrdmulh",
            _ => return lens_unk(),
        };
        let lens_is_fp = matches!(lens_opcode, 0x1 | 0x5 | 0x9);
        let lens_arr = if lens_is_fp {
            lens_farr((lens_size >> 1) & 1, lens_q)
        } else {
            lens_varr(lens_size, lens_q)
        };
        let lens_widening = matches!(lens_opcode, 0x2 | 0x6 | 0xa | 0x3 | 0x7 | 0xb);
        let lens_suf = if lens_widening && lens_q == 1 { "2" } else { "" };
        return lens_mk(
            format!("{lens_mn}{lens_suf} v{lens_rd}.{lens_arr}, v{lens_rn}.{lens_arr}, v?.?[?]"),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xDF20_0400 == 0x5E20_0400 {
        let lens_u = (lens_raw >> 29) & 1;
        let lens_hi = (lens_raw >> 23) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_ftype = (lens_raw >> 22) & 1;
        let lens_rm = (lens_raw >> 16) & 0x1f;
        let lens_opcode = (lens_raw >> 11) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x1a) => {
                if lens_hi == 0 {
                    "fadd"
                } else {
                    "fsub"
                }
            }
            (0, 0x1b) => "fmulx",
            (0, 0x1c) => "fcmeq",
            (0, 0x1f) => {
                if lens_hi == 0 {
                    "frecps"
                } else {
                    "frsqrts"
                }
            }
            (1, 0x1a) => "fabd",
            (1, 0x1b) => "fmul",
            (1, 0x1c) => {
                if lens_hi == 0 {
                    "fcmge"
                } else {
                    "fcmgt"
                }
            }
            (1, 0x1d) => {
                if lens_hi == 0 {
                    "facge"
                } else {
                    "facgt"
                }
            }
            (0, 0x10) => "add",
            (1, 0x10) => "sub",
            (1, 0x11) => "cmeq",
            _ => return lens_unk(),
        };
        let _ = lens_size;
        return lens_mk(
            format!(
                "{lens_mn} {}, {}, {}",
                lens_fp_reg(lens_ftype, lens_rd),
                lens_fp_reg(lens_ftype, lens_rn),
                lens_fp_reg(lens_ftype, lens_rm)
            ),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xDF3E_0C00 == 0x5E20_0800 {
        let lens_u = (lens_raw >> 29) & 1;
        let lens_hi = (lens_raw >> 23) & 1;
        let lens_opcode = (lens_raw >> 12) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_ftype = (lens_raw >> 22) & 1;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x1a) => {
                if lens_hi == 1 {
                    "fcvtps"
                } else {
                    "fcvtns"
                }
            }
            (0, 0x1b) => {
                if lens_hi == 1 {
                    "fcvtzs"
                } else {
                    "fcvtms"
                }
            }
            (1, 0x1a) => {
                if lens_hi == 1 {
                    "fcvtpu"
                } else {
                    "fcvtnu"
                }
            }
            (1, 0x1b) => {
                if lens_hi == 1 {
                    "fcvtzu"
                } else {
                    "fcvtmu"
                }
            }
            (0, 0x1d) => {
                if lens_hi == 1 {
                    "frecpe"
                } else {
                    "scvtf"
                }
            }
            (1, 0x1d) => {
                if lens_hi == 1 {
                    "frsqrte"
                } else {
                    "ucvtf"
                }
            }
            (0, 0x0c) => "fcmgt",
            (1, 0x0c) => "fcmge",
            (0, 0x0d) => "fcmeq",
            (1, 0x0d) => "fcmle",
            (0, 0x0e) => "fcmlt",
            (1, 0x1f) => "fsqrt",
            _ => return lens_unk(),
        };
        return lens_mk(
            format!("{lens_mn} {}, {}", lens_fp_reg(lens_ftype, lens_rd), lens_fp_reg(lens_ftype, lens_rn)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xDFE0_8400 == 0x5E00_0400 {
        let lens_imm5 = (lens_raw >> 16) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_size = lens_imm5.trailing_zeros().min(3);
        let lens_ft = match lens_size {
            0 => 0,
            1 => 3,
            2 => 0,
            _ => 1,
        };
        return lens_mk(
            format!("mov {}, v{lens_rn}.?[?]", lens_fp_reg(lens_ft, lens_rd)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xDF00_0400 == 0x5F00_0000 {
        let lens_u = (lens_raw >> 29) & 1;
        let lens_size = (lens_raw >> 22) & 3;
        let lens_opcode = (lens_raw >> 12) & 0xf;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x1) => "fmla",
            (0, 0x5) => "fmls",
            (0, 0x9) => "fmul",
            (1, 0x9) => "fmulx",
            (0, 0x3) => "sqdmlal",
            (0, 0x7) => "sqdmlsl",
            (0, 0xb) => "sqdmull",
            (0, 0xc) => "sqdmulh",
            (0, 0xd) => "sqrdmulh",
            _ => return lens_unk(),
        };
        let lens_ftype = (lens_size >> 1) & 1;
        return lens_mk(
            format!("{lens_mn} {}, {}, v?.?[?]", lens_fp_reg(lens_ftype, lens_rd), lens_fp_reg(lens_ftype, lens_rn)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xBF00_0000 == 0x0D00_0000 {
        let lens_l = (lens_raw >> 22) & 1;
        let lens_r = (lens_raw >> 21) & 1;
        let lens_opcode = (lens_raw >> 13) & 0x7;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rt = lens_raw & 0x1f;
        let lens_n = if lens_r == 1 { 2 } else { 1 } + if lens_opcode & 1 == 1 { 2 } else { 0 };
        let lens_mn = if lens_opcode == 0b110 && lens_l == 1 {
            format!("ld{lens_n}r")
        } else if lens_l == 1 {
            format!("ld{lens_n}")
        } else {
            format!("st{lens_n}")
        };
        return lens_mk(
            format!("{lens_mn} {{v{lens_rt}.?}}[?], [{}]", lens_reg(lens_rn, true, true)),
            LensFlow::LensFallthrough,
        );
    }

    if lens_raw & 0xFFFF_F01F == 0xD503_301F {
        let lens_crm = (lens_raw >> 8) & 0xf;
        let lens_opc = (lens_raw >> 5) & 0x7;
        let lens_mn = match lens_opc {
            0b100 => "dsb",
            0b101 => "dmb",
            0b110 => "isb",
            _ => return lens_unk(),
        };
        let lens_opt = match lens_crm {
            0b1111 => "sy".to_string(),
            0b1110 => "st".to_string(),
            0b1101 => "ld".to_string(),
            0b1011 => "ish".to_string(),
            0b1010 => "ishst".to_string(),
            0b1001 => "ishld".to_string(),
            0b0111 => "nsh".to_string(),
            0b0011 => "osh".to_string(),
            lens_other => format!("#0x{lens_other:x}"),
        };
        return lens_mk(format!("{lens_mn} {lens_opt}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x9FF8_0000 == 0x0F00_0000 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_op = (lens_raw >> 29) & 1;
        let lens_cmode = (lens_raw >> 12) & 0xf;
        let lens_abc = (lens_raw >> 16) & 0x7;
        let lens_defgh = (lens_raw >> 5) & 0x1f;
        let lens_imm8 = (lens_abc << 5) | lens_defgh;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = if lens_op == 0 {
            match lens_cmode {
                0xF => "fmov",
                0xC..=0xE => "movi",
                lens_c if lens_c & 1 == 0 => "movi",
                _ => "orr",
            }
        } else {
            match lens_cmode {
                0xF => "fmov",
                0xE => "movi",
                0xC | 0xD => "mvni",
                lens_c if lens_c & 1 == 0 => "mvni",
                _ => "bic",
            }
        };
        let lens_arr = if lens_q == 1 { "16b" } else { "8b" };
        return lens_mk(format!("{lens_mn} v{lens_rd}.{lens_arr}, #0x{lens_imm8:x}"), LensFlow::LensFallthrough);
    }

    if lens_raw & 0x9F80_0400 == 0x0F00_0400 {
        let lens_q = (lens_raw >> 30) & 1;
        let lens_u = (lens_raw >> 29) & 1;
        let lens_immh = (lens_raw >> 19) & 0xf;
        if lens_immh == 0 {
            return lens_unk();
        }
        let lens_opcode = (lens_raw >> 11) & 0x1f;
        let lens_rn = (lens_raw >> 5) & 0x1f;
        let lens_rd = lens_raw & 0x1f;
        let lens_mn = match (lens_u, lens_opcode) {
            (0, 0x00) => "sshr",
            (1, 0x00) => "ushr",
            (0, 0x02) => "ssra",
            (1, 0x02) => "usra",
            (0, 0x04) => "srshr",
            (1, 0x04) => "urshr",
            (0, 0x06) => "srsra",
            (1, 0x06) => "ursra",
            (1, 0x08) => "sri",
            (0, 0x0a) => "shl",
            (1, 0x0a) => "sli",
            (1, 0x0c) => "sqshlu",
            (0, 0x0e) => "sqshl",
            (1, 0x0e) => "uqshl",
            (0, 0x10) => "shrn",
            (1, 0x10) => "sqshrun",
            (0, 0x12) => "sqshrn",
            (1, 0x12) => "uqshrn",
            (0, 0x14) => "sshll",
            (1, 0x14) => "ushll",
            (0, 0x1c) => "scvtf",
            (1, 0x1c) => "ucvtf",
            (0, 0x1f) => "fcvtzs",
            (1, 0x1f) => "fcvtzu",
            _ => return lens_unk(),
        };
        let lens_size = if lens_immh & 0x8 != 0 {
            3
        } else if lens_immh & 0x4 != 0 {
            2
        } else if lens_immh & 0x2 != 0 {
            1
        } else {
            0
        };
        let lens_two = lens_q == 1 && matches!(lens_opcode, 0x14 | 0x10 | 0x12);
        let lens_suf = if lens_two { "2" } else { "" };
        let lens_a = lens_varr(lens_size, lens_q);
        return lens_mk(
            format!("{lens_mn}{lens_suf} v{lens_rd}.{lens_a}, v{lens_rn}.{lens_a}, #?"),
            LensFlow::LensFallthrough,
        );
    }

    lens_unk()
}

fn lens_simm(lens_v: i64) -> String {
    if lens_v < 0 {
        format!("-0x{:x}", lens_v.unsigned_abs())
    } else {
        format!("0x{lens_v:x}")
    }
}

fn lens_shift_str(lens_shtype: u32, lens_amount: u32) -> String {
    if lens_amount == 0 {
        return String::new();
    }
    let lens_s = match lens_shtype {
        0 => "lsl",
        1 => "lsr",
        2 => "asr",
        _ => "ror",
    };
    format!(", {lens_s} #{lens_amount}")
}

fn lens_varr(lens_size: u32, lens_q: u32) -> &'static str {
    match (lens_size, lens_q) {
        (0, 0) => "8b",
        (0, 1) => "16b",
        (1, 0) => "4h",
        (1, 1) => "8h",
        (2, 0) => "2s",
        (2, 1) => "4s",
        (3, 0) => "1d",
        _ => "2d",
    }
}

fn lens_farr(lens_hi: u32, lens_q: u32) -> &'static str {
    if lens_hi == 1 {
        "2d"
    } else if lens_q == 1 {
        "4s"
    } else {
        "2s"
    }
}

fn lens_fp_reg(lens_ftype: u32, lens_n: u32) -> String {
    let lens_p = match lens_ftype {
        0 => 's',
        1 => 'd',
        3 => 'h',
        _ => 'v',
    };
    format!("{lens_p}{lens_n}")
}

fn lens_simd_ls_size(lens_size: u32, lens_opc: u32) -> (char, u32) {
    if lens_opc & 0b10 != 0 {
        ('q', 4)
    } else {
        match lens_size {
            0 => ('b', 0),
            1 => ('h', 1),
            2 => ('s', 2),
            _ => ('d', 3),
        }
    }
}

fn lens_decode_bitmask(lens_n: u32, lens_imms: u32, lens_immr: u32, lens_datasize: u32) -> Option<u64> {
    let lens_x = ((lens_n & 1) << 6) | ((!lens_imms) & 0x3f);
    if lens_x == 0 {
        return None;
    }
    let lens_len = 31 - lens_x.leading_zeros();
    if lens_len == 0 {
        return None;
    }
    let lens_esize = 1u32 << lens_len;
    if lens_esize > lens_datasize {
        return None;
    }
    let lens_levels = lens_esize - 1;
    let lens_s = lens_imms & lens_levels;
    let lens_r = lens_immr & lens_levels;
    if lens_s == lens_levels {
        return None;
    }
    let lens_s1 = lens_s + 1;
    let lens_welem: u64 = if lens_s1 >= 64 { u64::MAX } else { (1u64 << lens_s1) - 1 };
    let lens_emask: u64 = if lens_esize >= 64 {
        u64::MAX
    } else {
        (1u64 << lens_esize) - 1
    };
    let lens_rot = lens_r % lens_esize;
    let lens_elem = if lens_rot == 0 {
        lens_welem & lens_emask
    } else {
        ((lens_welem >> lens_rot) | (lens_welem << (lens_esize - lens_rot))) & lens_emask
    };
    let mut lens_result: u64 = 0;
    let mut lens_pos = 0u32;
    while lens_pos < lens_datasize {
        lens_result |= lens_elem << lens_pos;
        lens_pos += lens_esize;
    }
    if lens_datasize < 64 {
        lens_result &= (1u64 << lens_datasize) - 1;
    }
    Some(lens_result)
}

fn lens_vfp_expand_imm(lens_imm8: u32) -> f64 {
    let lens_sign = (lens_imm8 >> 7) & 1;
    let lens_b = (lens_imm8 >> 6) & 1;
    let lens_cd = (lens_imm8 >> 4) & 0x3;
    let lens_frac = (lens_imm8 & 0xf) as u64;
    let lens_e_hi = (1 - lens_b) as u64;
    let lens_e_mid = if lens_b == 1 { 0xffu64 } else { 0 };
    let lens_exp = (lens_e_hi << 10) | (lens_e_mid << 2) | (lens_cd as u64);
    let lens_bits = ((lens_sign as u64) << 63) | (lens_exp << 52) | (lens_frac << 48);
    f64::from_bits(lens_bits)
}

#[cfg(test)]
mod lens_tests {
    use super::*;

    fn lens_t(lens_raw: u32) -> String {
        lens_decode(lens_raw, 0x1000).lens_text
    }

    #[test]
    fn lens_branches_and_returns() {
        assert_eq!(lens_decode(0xD65F03C0, 0x1000).lens_flow, LensFlow::LensReturn);
        assert_eq!(lens_t(0xD65F03C0), "ret");
        assert_eq!(lens_t(0xD503201F), "nop");
        assert_eq!(lens_decode(0x94000000, 0x1000).lens_flow, LensFlow::LensCall(0x1000));
        assert_eq!(lens_decode(0x14000004, 0x1000).lens_flow, LensFlow::LensBranch(0x1010));
        assert_eq!(lens_t(0x14000004), "b 0x1010");
    }

    #[test]
    fn lens_cond_branch_and_cbz() {
        let lens_d = lens_decode(0x54000040, 0x1000);
        assert_eq!(lens_d.lens_flow, LensFlow::LensCondBranch(0x1008));
        assert_eq!(lens_d.lens_text, "b.eq 0x1008");
        let lens_d = lens_decode(0xB4000040, 0x1000);
        assert_eq!(lens_d.lens_text, "cbz x0, 0x1008");
        assert_eq!(lens_d.lens_flow, LensFlow::LensCondBranch(0x1008));
    }

    #[test]
    fn lens_moves() {
        assert_eq!(lens_t(0xD2800020), "mov x0, #0x1");
        assert_eq!(lens_t(0x528000E1), "mov w1, #0x7");
    }

    #[test]
    fn lens_add_sub_cmp() {
        assert_eq!(lens_t(0x91004020), "add x0, x1, #0x10");
        assert_eq!(lens_t(0xF1000C1F), "cmp x0, #0x3");
    }

    #[test]
    fn lens_loads_stores() {
        assert_eq!(lens_t(0xF9400020), "ldr x0, [x1]");
        assert_eq!(lens_t(0xF9000FE0), "str x0, [sp, #0x18]");
    }

    #[test]
    fn lens_mov_reg_and_ret_reg() {
        assert_eq!(lens_t(0xAA0103E0), "mov x0, x1");
    }

    #[test]
    fn lens_unknown_is_word() {
        assert_eq!(lens_t(0x9e7e0000), ".word 0x9e7e0000");
    }

    #[test]
    fn lens_extra_isa_classes() {
        assert_eq!(lens_t(0x1a8883e9), "csel w9, wzr, w8, hi");
        assert_eq!(lens_t(0x9b027c20), "mul x0, x1, x2");
        assert_eq!(lens_t(0x531c6c20), "lsl w0, w1, #4");
        assert_eq!(lens_t(0xf85f8020), "ldur x0, [x1, #-0x8]");
        assert_eq!(lens_t(0x1ac20820), "udiv w0, w1, w2");
    }

    fn lens_m(lens_raw: u32) -> String {
        lens_t(lens_raw).split_whitespace().next().unwrap_or("").to_string()
    }

    #[test]
    fn lens_signed_and_simd_loads() {
        assert_eq!(lens_t(0xb98002a8), "ldrsw x8, [x21]");
        assert_eq!(lens_m(0x39c0010a), "ldrsb");
        assert_eq!(lens_m(0xb89503a0), "ldursw");
        assert_eq!(lens_m(0x3dc16100), "ldr");
        assert_eq!(lens_t(0x3dc16100), "ldr q0, [x8, #0x580]");
        assert_eq!(lens_t(0x3c900100), "stur q0, [x8, #-0x100]");
    }

    #[test]
    fn lens_logical_immediate() {
        assert_eq!(lens_t(0x927ff928), "and x8, x9, #0xfffffffffffffffe");
        assert_eq!(lens_t(0x320003e2), "mov w2, #0x1");
        assert_eq!(lens_m(0x52000348), "eor");
        assert_eq!(lens_m(0x7200011f), "tst");
    }

    #[test]
    fn lens_fp_scalar_family() {
        assert_eq!(lens_t(0x1e622020), "fcmp d1, d2");
        assert_eq!(lens_t(0x1e610808), "fmul d8, d0, d1");
        assert_eq!(lens_m(0x1e202800), "fadd");
        assert_eq!(lens_m(0x1e603901), "fsub");
        assert_eq!(lens_m(0x1e601908), "fdiv");
        assert_eq!(lens_t(0x1e629002), "fmov d2, #5.00000000");
        assert_eq!(lens_m(0x1e6202a0), "scvtf");
        assert_eq!(lens_m(0x1e780001), "fcvtzs");
        assert_eq!(lens_m(0x1e681d2a), "fcsel");
    }

    #[test]
    fn lens_atomics_and_exclusives() {
        assert_eq!(lens_t(0xc85ffe89), "ldaxr x9, [x20]");
        assert_eq!(lens_t(0x08dffd08), "ldarb w8, [x8]");
        assert_eq!(lens_m(0xc808ff80), "stlxr");
        assert_eq!(lens_m(0xc89ffd17), "stlr");
    }

    #[test]
    fn lens_misc_new_classes() {
        assert_eq!(lens_t(0xd4200020), "brk #0x1");
        assert_eq!(lens_t(0x5ac01129), "clz w9, w9");
        assert_eq!(lens_t(0xdac00128), "rbit x8, x9");
        assert_eq!(lens_m(0xfa401904), "ccmp");
        assert_eq!(lens_m(0x8b2ac108), "add");
        assert_eq!(lens_t(0x9b357d2a), "smull x10, w9, w21");
        assert_eq!(lens_m(0x4f00e400), "movi");
        assert_eq!(lens_m(0x00000010), "udf");
    }

    #[test]
    fn lens_neon_vector_families() {
        assert_eq!(lens_m(0x2e22dc84), "fmul");
        assert_eq!(lens_m(0x4e22ce54), "fmla");
        assert_eq!(lens_m(0x4e20d440), "fadd");
        assert_eq!(lens_m(0x0ea2d421), "fsub");
        assert_eq!(lens_m(0x6e20d400), "faddp");
        assert_eq!(lens_m(0x4e201c85), "and");
        assert_eq!(lens_m(0x4ea41c63), "orr");
        assert_eq!(lens_m(0x6e611c40), "bsl");
        assert_eq!(lens_m(0x4ee38421), "add");
        assert_eq!(lens_t(0x4ea81d00), "mov v0.16b, v8.16b");
        assert_eq!(lens_m(0x4e080e80), "dup");
        assert_eq!(lens_m(0x0e0e3c8b), "umov");
        assert_eq!(lens_m(0x6e004000), "ext");
        assert_eq!(lens_m(0x4e9038a5), "zip1");
        assert_eq!(lens_m(0x0e8b794f), "zip2");
        assert_eq!(lens_m(0x4e801802), "uzp1");
        assert_eq!(lens_m(0x6ea0fa24), "fneg");
        assert_eq!(lens_m(0x4ea1d800), "frecpe");
        assert_eq!(lens_m(0x4ea00a05), "rev64");
        assert_eq!(lens_m(0x4e020066), "tbl");
        assert_eq!(lens_m(0x4cdf7003), "ld1");
        assert_eq!(lens_t(0xd5033f5f), "clrex");
    }

    #[test]
    fn lens_indexed_scalar_and_misc_simd() {
        assert_eq!(lens_m(0x4f801023), "fmla");
        assert_eq!(lens_m(0x0f845061), "fmls");
        assert_eq!(lens_m(0x4f408261), "mul");
        assert_eq!(lens_m(0x5fa09042), "fmul");
        assert_eq!(lens_t(0x93c08008), "ror x8, x0, #0x20");
        assert_eq!(lens_t(0x1a090149), "adc w9, w10, w9");
        assert_eq!(lens_t(0xba1101b7), "adcs x23, x13, x17");
        assert_eq!(lens_m(0x4f095421), "shl");
        assert_eq!(lens_m(0x0f20a400), "sshll");
        assert_eq!(lens_m(0x2f10a484), "ushll");
        assert_eq!(lens_m(0x0e67c0d4), "smull");
        assert_eq!(lens_t(0x5e61d800), "scvtf d0, d0");
        assert_eq!(lens_m(0x7ee0d500), "fabd");
        assert_eq!(lens_m(0x5e0c0422), "mov");
        assert_eq!(lens_m(0x4d408101), "ld1");
    }

    #[test]
    fn lens_cmp_records_flags_and_bcond_carries_code() {
        let lens_cmp = lens_decode(0xF1000C1F, 0x1000);
        let lens_f = lens_cmp.lens_flags.expect("cmp should set flags");
        assert_eq!(lens_f.lens_kind, LensFlagKind::LensCmp);
        assert_eq!(lens_f.lens_a, "x0");
        assert_eq!(lens_f.lens_b, "0x3");
        let lens_bne = lens_decode(0x54000041, 0x1000);
        assert_eq!(lens_bne.lens_cond, Some(1));
        assert_eq!(lens_bne.lens_flow, LensFlow::LensCondBranch(0x1008));
    }
}

// Preserve upstream diagnostic labels independently of internal names.
impl std::fmt::Debug for LensFlow { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self {Self::LensFallthrough => { lens_formatter.write_str("Fallthrough") },Self::LensBranch(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("Branch");lens_debug.field(lens_slot0);lens_debug.finish() },Self::LensCall(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("Call");lens_debug.field(lens_slot0);lens_debug.finish() },Self::LensCondBranch(lens_slot0) => { let mut lens_debug = lens_formatter.debug_tuple("CondBranch");lens_debug.field(lens_slot0);lens_debug.finish() },Self::LensReturn => { lens_formatter.write_str("Return") },Self::LensIndirect => { lens_formatter.write_str("Indirect") },Self::LensIndirectCall => { lens_formatter.write_str("IndirectCall") }} } }
impl std::fmt::Debug for LensFlagKind { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self {Self::LensCmp => { lens_formatter.write_str("Cmp") },Self::LensCmn => { lens_formatter.write_str("Cmn") },Self::LensTst => { lens_formatter.write_str("Tst") }} } }
impl std::fmt::Debug for LensFlagOp { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("FlagOp");lens_debug.field("a", &self.lens_a);lens_debug.field("b", &self.lens_b);lens_debug.field("kind", &self.lens_kind);lens_debug.finish() } }
impl std::fmt::Debug for LensInsn { fn fmt(&self, lens_formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { let mut lens_debug = lens_formatter.debug_struct("Insn");lens_debug.field("addr", &self.lens_addr);lens_debug.field("raw", &self.lens_raw);lens_debug.field("text", &self.lens_text);lens_debug.field("flow", &self.lens_flow);lens_debug.field("cond", &self.lens_cond);lens_debug.field("flags", &self.lens_flags);lens_debug.finish() } }
