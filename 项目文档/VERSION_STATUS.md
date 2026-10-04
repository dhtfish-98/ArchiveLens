# Current source version and limits

The maintained source and the eight local Cargo workspace packages use version
**1.0.4** together. The v1.0.3 source Release remains a historical version;
earlier v1.0.0 through v1.0.2 source Releases used the inherited Cargo value
0.1.0. Local package versions identify this source bundle; they do not assert
crate-registry publication, full API stability or platform coverage. Third-party
package versions, original copyright and license notices are unchanged.

This source update addresses **ARCHIVELENS-EXISTING-CHAT-PIPE** in the tested
GUI child-process path. The GUI now drains stderr concurrently with streaming
stdout and retains at most 64 KiB of stderr bytes for an error report. A local
fake child writing 1 KiB and 1 MiB to stderr before stdout finished within a
five-second watchdog; the three GUI tests and a Release build passed. This is
not an interactive test of the actual Claude or Codex CLIs or another platform.
The interrupted deep source audit has not resumed. These five other previously
recorded issues remain **OPEN / not fixed or revalidated in this update**:

- ARCHIVELENS-EXISTING-ENCRYPTION-COMMAND
- ARCHIVELENS-EXISTING-FUNCTION-STARTS-SPAN
- ARCHIVELENS-EXISTING-GUI-SYMLINK
- ARCHIVELENS-EXISTING-LOAD-COMMAND-AREA
- ARCHIVELENS-EXISTING-ULEB-OVERFLOW

The six issue identifiers refer to saved observations at source commit
`ea9e4af7e4f800d7cc0ead414a4fc3b9a7c20702`; ordinary current builds or CI success
do not establish that the other recorded behaviors are repaired. GUI, larger
parsing cores and platform coverage remain incomplete. Read the existing
[defensive scope](DEFENSIVE_SCOPE.md) and [origin record](ORIGIN.md) for their
separate limits and third-party attribution. CVP eligibility, applicant identity,
authorized real task, organizational approval and program acceptance remain OPEN.

The prior v1.0.3 Release asset is source only. This revision does not publish a
newly built binary; any new source Release must be checked against its own tag.
Current version maintenance is by dhtfish98; the attributed upstream derivative
relationship and original author rights remain unchanged.
