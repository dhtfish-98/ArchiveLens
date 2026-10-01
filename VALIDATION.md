# Validation evidence

## PASS

The original and modified full workspaces built successfully. All 104 existing tests passed in the modified workspace; 104 original core/CLI tests passed, and the original GUI build passed. The release workspace built. Contract comparison passed 54 checks: 9 command families on raw Mach-O, an IPA containing that executable, malformed/missing inputs, help/flags, a project export, and independently compiled consumers over 50,000 ARM64 words, 20 type encodings and 5 method signatures. Two independently invoked Capstone scripts produced the same deterministic results (wall clock throughput excluded).

The local report contains **54 passed contract checks**. Executed local build/test logs and original-baseline evidence are retained outside this public repository. Source and build bundles are created only after final file contents are frozen. Independent consumers link/import the renamed API from a separate project.

## Compatibility/normalization

The required `eframe::App::update` trait name is preserved. No algorithm modernization was applied. Benchmark shell local variables and executable references were updated; compatibility environment keys remain accepted.

Only executable/project branding, compile dates and NSLog timestamps/process IDs are normalized in regular CLI comparisons. Rust export READMEs normalize the generator brand. The observed known formatter abort compares exception type/reason rather than ASLR stack addresses. Parsed fields and production algorithms are not normalized.

## OPEN

- Interactive GUI launch, Windows execution, real-device IPA/app flows and complete program semantics are unverified.
- The hosted CI workflow has been authored but has not yet run on GitHub at this local handoff.
- Passing these finite checks is not a proof of every input or complete feature equivalence.
