# Validation evidence

## 1.0.6 five-observation regression — 2026-10-05

On the public 1.0.5 source commit, four targeted Mach-O tests failed on the
recorded malformed inputs, and the GUI IPA test overwrote an owned marker
through a destination symlink. After the bounded corrections, the locked local
workspace suite passed 113 tests, including all five regressions. The source
now rejects malformed encryption commands, function-start members that end
inside an encoded number, commands outside the declared command area, and
overflowing ULEB128 numbers. GUI extraction creates a private directory,
uses exclusive file creation, and cleans the directory when its owner drops.
The GUI check uses an owned ZIP and marker, not
an external assistant or an interactive window. The interrupted deep source
audit, real GUI and device behavior, and other parser paths remain OPEN.

## 1.0.5 document-layout validation — 2026-10-05

Two historical upstream guide images moved byte-for-byte from root `resources/` to `项目文档/resources/`. The retained `UPSTREAM_GUIDE.md` image link now resolves relative to that document. `RENAME_MAP.json` and `SOURCE_MANIFEST.json` record the canonical tracked paths. Runtime Rust source, test source and original upstream/third-party notices remain unchanged. On this 1.0.5 candidate, the 424-file tracked source manifest and Build-only compatibility copy matched their SHA-256 records. With the official Rust 1.99.0 toolchain installed only in Build, the locked workspace suite passed 107/107 tests and the full workspace Release build passed. Exact-commit CI and source Release verification remain separate checks from this local result and from the historical 1.0.4 GUI result below.

## Historical rewrite comparison — PASS at its original handoff

The original and modified full workspaces built successfully. All 104 existing tests passed in the modified workspace; 104 original core/CLI tests passed, and the original GUI build passed. The release workspace built. Contract comparison passed 54 checks: 9 command families on raw Mach-O, an IPA containing that executable, malformed/missing inputs, help/flags, a project export, and independently compiled consumers over 50,000 ARM64 words, 20 type encodings and 5 method signatures. Two independently invoked Capstone scripts produced the same deterministic results (wall clock throughput excluded).

The local report contains **54 passed contract checks**. Executed local build/test logs and original-baseline evidence are retained outside this public repository. Source and build bundles are created only after final file contents are frozen. Independent consumers link/import the renamed API from a separate project.

## Compatibility/normalization

The required `eframe::App::update` trait name is preserved. No algorithm modernization was applied. Benchmark shell local variables and executable references were updated; compatibility environment keys remain accepted.

Only executable/project branding, compile dates and NSLog timestamps/process IDs are normalized in regular CLI comparisons. Rust export READMEs normalize the generator brand. The observed known formatter abort compares exception type/reason rather than ASLR stack addresses. Parsed fields and production algorithms are not normalized.

## Historical OPEN items at that handoff

- Interactive GUI launch, Windows execution, real-device IPA/app flows and complete program semantics are unverified.
- The hosted CI workflow has been authored but has not yet run on GitHub at this local handoff.
- Passing these finite checks is not a proof of every input or complete feature equivalence.

## Defensive assistant launch changes — 2026-10-02

Two current GUI tests pass: concurrent chats use separate private directories, prompt files are exclusive mode 0600 under mode 0700 on Unix, and dropping a directory cleans up only that chat; assistant arguments explicitly require Codex read-only sandbox and disable Claude tools without automatic grants. The GUI source compiles as part of these tests.

The previous unrestricted Codex flag and automatic Claude Bash grant have been removed. Codex is launched with user configuration/rules ignored in a private cwd, and Claude uses safe-mode with an empty built-in tool set. Current helper parameters require CLI versions supporting these flags; unsupported flags must fail rather than fall back to unrestricted execution. No request was sent to an external assistant to validate these settings.

Earlier CLI/crate comparison results remain finite historical evidence. The CI statement for the 2026-10-02 change applied to its then-current commit; it does not validate v1.0.4. Interactive GUI use, Windows behavior, actual external assistant behavior and all untrusted-input resource limits remain OPEN. Earlier v1.0.0 packages contain the predecessor's launch policy; current-source builds are required.

## GUI stderr pipe correction — 2026-10-05

The earlier GUI child-process path read stdout to completion before reading
stderr. A child that filled stderr before writing more stdout could stall.
Current source drains stderr on a separate thread while retaining at most
64 KiB of diagnostic bytes; stdout remains streamed to the UI. A local fake
child wrote 1 KiB and 1 MiB to stderr before `owned progress` on stdout; both
completed within a five-second watchdog, which kills a stalled fake child.
The targeted check passed, all three GUI tests passed, and the full local
workspace suite passed 107 tests with zero failures after the 1.0.4 version
update. The full workspace Release build also returned success after the
version update. The local `rust-objcopy` step
could not load `libLLVM.dylib`,
so no claim is made that debug information was stripped. This test uses no
real assistant process or network. The final source bundle and exact-commit CI
are separate checks; interactive GUI behavior,
Windows execution, and the other five recorded issues remain OPEN.
