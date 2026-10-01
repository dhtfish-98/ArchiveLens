use crate::{LensFlow, LensInsn};
use std::collections::BTreeSet;

pub struct LensBlock {
    pub lens_start: u64,
    pub lens_insns: Vec<LensInsn>,
    pub lens_succ: Vec<u64>,
}

pub fn lens_build_blocks(lens_insns: &[LensInsn]) -> Vec<LensBlock> {
    if lens_insns.is_empty() {
        return Vec::new();
    }
    let lens_lo = lens_insns.first().map(|lens_i| lens_i.lens_addr).unwrap_or(0);
    let lens_hi = lens_insns.last().map(|lens_i| lens_i.lens_addr + 4).unwrap_or(0);
    let lens_in_range = |lens_a: u64| lens_a >= lens_lo && lens_a < lens_hi;

    let mut lens_leaders: BTreeSet<u64> = BTreeSet::new();
    lens_leaders.insert(lens_insns[0].lens_addr);
    for (lens_idx, lens_ins) in lens_insns.iter().enumerate() {
        let lens_next = lens_ins.lens_addr + 4;
        match &lens_ins.lens_flow {
            LensFlow::LensBranch(lens_t) | LensFlow::LensCondBranch(lens_t) => {
                if lens_in_range(*lens_t) {
                    lens_leaders.insert(*lens_t);
                }
                if lens_idx + 1 < lens_insns.len() {
                    lens_leaders.insert(lens_next);
                }
            }
            LensFlow::LensCall(_) | LensFlow::LensIndirectCall | LensFlow::LensReturn | LensFlow::LensIndirect => {
                if lens_idx + 1 < lens_insns.len() {
                    lens_leaders.insert(lens_next);
                }
            }
            LensFlow::LensFallthrough => {}
        }
    }

    let mut lens_blocks: Vec<LensBlock> = Vec::new();
    let mut lens_cur: Vec<LensInsn> = Vec::new();
    let mut lens_start = lens_insns[0].lens_addr;
    for (lens_idx, lens_ins) in lens_insns.iter().enumerate() {
        if lens_leaders.contains(&lens_ins.lens_addr) && !lens_cur.is_empty() {
            let lens_s = lens_start;
            lens_blocks.push(LensBlock {
                lens_start: lens_s,
                lens_insns: std::mem::take(&mut lens_cur),
                lens_succ: vec![lens_ins.lens_addr],
            });
            lens_start = lens_ins.lens_addr;
        }
        let lens_flow = lens_ins.lens_flow.clone();
        lens_cur.push(lens_ins.clone());
        let lens_is_last = lens_idx + 1 == lens_insns.len();
        let lens_terminates = !matches!(lens_flow, LensFlow::LensFallthrough | LensFlow::LensCall(_) | LensFlow::LensIndirectCall);
        if lens_terminates || lens_is_last {
            let lens_next = lens_ins.lens_addr + 4;
            let lens_succ = match lens_flow {
                LensFlow::LensBranch(lens_t) => lens_filt(&[lens_t], lens_in_range),
                LensFlow::LensCondBranch(lens_t) => lens_filt(&[lens_t, lens_next], lens_in_range),
                LensFlow::LensReturn | LensFlow::LensIndirect => Vec::new(),
                LensFlow::LensCall(_) | LensFlow::LensIndirectCall | LensFlow::LensFallthrough => lens_filt(&[lens_next], lens_in_range),
            };
            lens_blocks.push(LensBlock {
                lens_start: lens_start,
                lens_insns: std::mem::take(&mut lens_cur),
                lens_succ: lens_succ,
            });
            lens_start = lens_next;
        }
    }
    if !lens_cur.is_empty() {
        lens_blocks.push(LensBlock {
            lens_start: lens_start,
            lens_insns: lens_cur,
            lens_succ: Vec::new(),
        });
    }
    lens_blocks
}

fn lens_filt(lens_addrs: &[u64], lens_in_range: impl Fn(u64) -> bool) -> Vec<u64> {
    lens_addrs.iter().copied().filter(|lens_a| lens_in_range(*lens_a)).collect()
}

#[cfg(test)]
mod lens_tests {
    use super::*;
    use crate::lens_decode;

    #[test]
    fn lens_splits_on_conditional_branch() {
        let lens_insns = vec![
            lens_decode(0x34000040, 0x1000),
            lens_decode(0xD2800021, 0x1004),
            lens_decode(0xD65F03C0, 0x1008),
        ];
        let lens_blocks = lens_build_blocks(&lens_insns);
        assert_eq!(lens_blocks[0].lens_start, 0x1000);
        assert!(lens_blocks[0].lens_succ.contains(&0x1008));
        assert!(lens_blocks[0].lens_succ.contains(&0x1004));
        let lens_ret_block = lens_blocks.iter().find(|lens_b| lens_b.lens_start == 0x1008).unwrap();
        assert!(lens_ret_block.lens_succ.is_empty());
    }

    #[test]
    fn lens_straight_line_is_one_block() {
        let lens_insns = vec![
            lens_decode(0xD2800020, 0x1000),
            lens_decode(0x91000400, 0x1004),
            lens_decode(0xD65F03C0, 0x1008),
        ];
        let lens_blocks = lens_build_blocks(&lens_insns);
        assert_eq!(lens_blocks.len(), 1);
        assert_eq!(lens_blocks[0].lens_insns.len(), 3);
        assert!(lens_blocks[0].lens_succ.is_empty());
    }
}
