# Current source version and limits

The maintained source release and the eight local Cargo workspace packages use
version **1.0.3** together. Earlier source Releases v1.0.0 through v1.0.2 used
the inherited Cargo value 0.1.0; those tags and assets remain historical records.
This alignment identifies the maintained source bundle. It does not assert that
the local libraries are published crates or that every public API/platform is
stable. Third-party package versions, copyright and license notices are unchanged.

This update changes version metadata and this explanation only. It does not fix
program behavior or repeat the interrupted deep source audit. These six previously
recorded issues remain **OPEN / not fixed or revalidated in this update**:

- ARCHIVELENS-EXISTING-CHAT-PIPE
- ARCHIVELENS-EXISTING-ENCRYPTION-COMMAND
- ARCHIVELENS-EXISTING-FUNCTION-STARTS-SPAN
- ARCHIVELENS-EXISTING-GUI-SYMLINK
- ARCHIVELENS-EXISTING-LOAD-COMMAND-AREA
- ARCHIVELENS-EXISTING-ULEB-OVERFLOW

The six issue identifiers refer to saved observations at source commit
`ea9e4af7e4f800d7cc0ead414a4fc3b9a7c20702`; ordinary current builds or CI success
do not establish that the recorded behavior is repaired. GUI, larger parsing
cores and platform coverage remain incomplete. Read the existing
[defensive scope](DEFENSIVE_SCOPE.md) and [origin record](ORIGIN.md) for their
separate limits and third-party attribution. CVP eligibility, applicant identity,
authorized real task, organizational approval and program acceptance remain OPEN.

Any v1.0.3 release asset is source only. No newly built binary is being published.
Current version maintenance is by dhtfish98; the attributed upstream derivative
relationship and original author rights remain unchanged.
