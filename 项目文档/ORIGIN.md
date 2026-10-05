# Source and modification record

ArchiveLens is a renamed and restructured derivative of [JRBusiness/REipa](https://github.com/JRBusiness/REipa) at commit `a21259b096344b9c69dac508f1fe935c785be9f2`. It is not an independently authored implementation of the upstream algorithms. Original copyright and license notices are retained.

License: **MIT**. The license applies to this derivative under its existing terms. The full license is included in `LICENSE`; `UPSTREAM_GUIDE.md` preserves the upstream description. ArchiveLens retains JRBusiness's MIT copyright notice.

Modified owned source files have new names, modules/types/functions/local bindings have new names, and build references are updated. The complete file/symbol identity mapping is in `RENAME_MAP.json`: 3392 mapped declarations and 28 mapped owned source files. Macro and supplemental Python/check mappings are recorded separately where applicable. Vendor source remains attributed and keeps its original file and symbol names.

Compiler entry points, external frameworks/trait overrides/selectors/KVC keys, serialization and command-line fields, required build metadata filenames, and fixed test fixture bytes remain compatibility boundaries. These exceptions are explicit in the mapping; replacing external names would change functionality. Original Apple Mach-O layout/field values and input class/method names are preserved.

No claim is made that renaming establishes authorship, eligibility for an application, or a formal proof of complete behavioral equivalence.

## Subsequent defensive maintenance — 2026-10-02

The earlier name/file mapping remains a historical transformation record. Later source changes and scope documents record subsequent maintenance, not independent authorship of the upstream algorithms. Current changes and their finite verification are listed in DEFENSIVE_SCOPE.md and VALIDATION.md. Earlier release packages and equivalence reports describe their corresponding earlier commits. Original attribution and license obligations remain.
