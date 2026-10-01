#!/usr/bin/env python3
"""Disassemble individual 32-bit little-endian hex words with Capstone.
Usage: python lens_word_oracle.py <hexword> [<hexword> ...]  (e.g. 1e202008)
"""
import sys
from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM
lens_md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
for lens_h in sys.argv[1:]:
    lens_w = int(lens_h, 16)
    lens_b = lens_w.to_bytes(4, "little")
    lens_got = list(lens_md.disasm(lens_b, 0x1000))
    if lens_got:
        lens_i = lens_got[0]
        print(f"{lens_h}: {lens_i.mnemonic} {lens_i.op_str}")
    else:
        print(f"{lens_h}: (undecoded)")
