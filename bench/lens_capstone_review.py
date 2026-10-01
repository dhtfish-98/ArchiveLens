#!/usr/bin/env python3
"""Score ArchiveLens's arm64 decoder against Capstone on a real __text section.

Input: a CSV produced by `lens-bench throughput <bin> --dump <csv>` with one
line per 4-byte word: `addr_hex,word_hex,reipa_mnemonic`. ArchiveLens emits the
mnemonic `.word` for anything it does not decode.

We reconstruct the raw instruction bytes from the words, disassemble them with
Capstone (the engine behind radare2/objection/many tools), and report:

  * decode coverage  - fraction of words each tool turns into a real insn
  * agreement        - where both decode, do the mnemonic families match
  * ArchiveLens gaps       - words ArchiveLens left as .word but Capstone decoded, bucketed
                       by Capstone mnemonic (shows what ISA ArchiveLens is missing)
  * Capstone throughput for the same bytes (wall-clock, native engine)

Usage: python lens_capstone_review.py <reipa_dump.csv> [--base 0xADDR]
"""
import sys
import time
from collections import Counter

try:
    from capstone import Cs, CS_ARCH_ARM64, CS_MODE_ARM
except ImportError:
    sys.exit("capstone not installed: pip install capstone")

# Alias groups: Capstone and ArchiveLens legitimately spell the same instruction
# differently. Treat members of a group as equal for agreement scoring.
lens_ALIASES = [
    {"mov", "movz", "movn", "orr", "movk"},   # mov synthesized from orr/movz/movn
    {"cmp", "subs"}, {"cmn", "adds"}, {"tst", "ands"},
    {"cset", "cinc", "csinc"}, {"csetm", "cinv", "csinv"}, {"cneg", "csneg"},
    {"ret", "br"}, {"bl", "blr"},
    {"lsl", "ubfm", "lsr", "asr", "sbfm", "ubfiz", "sbfiz", "ubfx", "sbfx", "bfi", "bfxil", "bfm"},
    {"neg", "sub"}, {"ngc", "sbc"}, {"mul", "madd"}, {"mneg", "msub"},
    {"mvn", "orn"}, {"nop", "hint"},
    # exact condition-code spellings (ARM defines these as equal):
    {"b.cs", "b.hs"}, {"b.cc", "b.lo"}, {"negs", "subs"}, {"ngcs", "sbcs"},
]

# Capstone's placeholder mnemonic for bytes it cannot decode (SKIPDATA mode).
lens_SKIP_MNEMONIC = ".skip"
def lens_canon(lens_m):
    lens_m = lens_m.lower()
    for lens_g in lens_ALIASES:
        if lens_m in lens_g:
            return "|".join(sorted(lens_g))
    return lens_m

def lens_main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    lens_csv_path = sys.argv[1]
    lens_base = None
    if "--base" in sys.argv:
        lens_base = int(sys.argv[sys.argv.index("--base") + 1], 16)

    lens_addrs, lens_words, lens_reipa = [], bytearray(), []
    with open(lens_csv_path, "r") as lens_f:
        for lens_line in lens_f:
            lens_parts = lens_line.rstrip("\n").split(",")
            if len(lens_parts) != 3:
                continue
            lens_a = int(lens_parts[0], 16)
            lens_w = int(lens_parts[1], 16)
            lens_addrs.append(lens_a)
            lens_words += lens_w.to_bytes(4, "little")
            lens_reipa.append(lens_parts[2])
    lens_n = len(lens_addrs)
    if lens_n == 0:
        sys.exit("empty dump")
    if lens_base is None:
        lens_base = lens_addrs[0]

    lens_md = Cs(CS_ARCH_ARM64, CS_MODE_ARM)
    lens_md.detail = False
    # Resync on 4-byte boundaries so Capstone attempts EVERY word, matching
    # ArchiveLens's linear-sweep decoder. Without this, disasm() halts at the first
    # undecodable word and only covers a contiguous prefix.
    lens_md.skipdata = True
    lens_md.skipdata_setup = (lens_SKIP_MNEMONIC, lambda lens_buffer, lens_size, lens_offset, lens_ud: 4, None)

    # Disassemble in 16 MB windows (4-byte aligned) so Capstone's bulk
    # allocation stays bounded on large binaries; instructions are independent
    # and word-aligned, so chunking at a 4-byte boundary is exact.
    lens_buf = bytes(lens_words)
    lens_CHUNK = 4_000_000 * 4  # 16 MB
    lens_t0 = time.perf_counter()
    lens_cap = {}  # addr -> mnemonic (real instructions only)
    for lens_cstart in range(0, len(lens_buf), lens_CHUNK):
        lens_chunk = lens_buf[lens_cstart:lens_cstart + lens_CHUNK]
        lens_caddr = lens_base + lens_cstart
        for lens_insn in lens_md.disasm(lens_chunk, lens_caddr):
            if lens_insn.mnemonic != lens_SKIP_MNEMONIC:
                lens_cap[lens_insn.address] = lens_insn.mnemonic
    lens_t1 = time.perf_counter()
    lens_cap_secs = lens_t1 - lens_t0
    lens_cap_minsn_s = (len(lens_cap) / lens_cap_secs) / 1e6 if lens_cap_secs else 0.0

    lens_reipa_decoded = sum(1 for lens_m in lens_reipa if lens_m != ".word")
    lens_cap_decoded = len(lens_cap)

    lens_both = lens_agree = 0
    lens_reipa_gap = Counter()      # ArchiveLens .word, Capstone decoded (missing ISA)
    lens_reipa_extra = 0            # ArchiveLens decoded, Capstone did not (data / over-eager)
    lens_disagree_samples = Counter()
    for lens_i, lens_a in enumerate(lens_addrs):
        lens_rm = lens_reipa[lens_i]
        lens_cm = lens_cap.get(lens_a)
        if lens_rm != ".word" and lens_cm is not None:
            lens_both += 1
            if lens_canon(lens_rm) == lens_canon(lens_cm):
                lens_agree += 1
            else:
                lens_disagree_samples[f"{lens_rm} vs {lens_cm}"] += 1
        elif lens_rm == ".word" and lens_cm is not None:
            lens_reipa_gap[lens_cm] += 1
        elif lens_rm != ".word" and lens_cm is None:
            lens_reipa_extra += 1

    print(f"words                : {lens_n}")
    print(f"ArchiveLens decoded        : {lens_reipa_decoded}  ({100*lens_reipa_decoded/lens_n:.1f}%)")
    print(f"Capstone decoded     : {lens_cap_decoded}  ({100*lens_cap_decoded/lens_n:.1f}%)")
    print(f"both decoded         : {lens_both}")
    print(f"  mnemonic agree     : {lens_agree}  ({100*lens_agree/lens_both:.2f}% of both)")
    print(f"  mnemonic disagree  : {lens_both-lens_agree}  ({100*(lens_both-lens_agree)/lens_both:.2f}% of both)")
    print(f"ArchiveLens .word, Cap real: {sum(lens_reipa_gap.values())}  (ArchiveLens ISA gaps)")
    print(f"ArchiveLens real, Cap none : {lens_reipa_extra}")
    print(f"Capstone throughput  : {lens_cap_minsn_s:.2f} Minsn/s  ({lens_cap_secs*1000:.1f} ms for {len(lens_cap)} insns)")
    print("\ntop ArchiveLens ISA gaps (Capstone mnemonic ArchiveLens left as .word):")
    for lens_m, lens_c in lens_reipa_gap.most_common(15):
        print(f"  {lens_c:>9}  {lens_m}")
    if lens_disagree_samples:
        print("\ntop mnemonic disagreements (after alias normalization):")
        for lens_s, lens_c in lens_disagree_samples.most_common(12):
            print(f"  {lens_c:>9}  {lens_s}")

if __name__ == "__main__":
    lens_main()
