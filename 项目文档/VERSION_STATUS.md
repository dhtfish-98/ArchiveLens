# Current source version and limits

The maintained source and the eight local Cargo workspace packages use version
**1.0.7** together. Earlier source Releases remain historical versions;
earlier v1.0.0 through v1.0.2 source Releases used the inherited Cargo value
0.1.0. Local package versions identify this source bundle; they do not assert
crate-registry publication, full API stability or platform coverage. Third-party
package versions, original copyright and license notices are unchanged.

Version 1.0.7 corrects one cross-project template sentence in ORIGIN.md.
It changes no runtime source; the five bounded runtime fixes remain the v1.0.6
evidence and still have the limitations below.

Version 1.0.4 addressed **ARCHIVELENS-EXISTING-CHAT-PIPE** in the tested
GUI child-process path. The GUI now drains stderr concurrently with streaming
stdout and retains at most 64 KiB of stderr bytes for an error report. A local
fake child writing 1 KiB and 1 MiB to stderr before stdout finished within a
five-second watchdog; the three GUI tests and a Release build passed. This is
not an interactive test of the actual Claude or Codex CLIs or another platform.
Version 1.0.6 re-tested the five other recorded observations against public
v1.0.5 source. All five reproduced: a present but truncated encryption command
was treated as absent, function-start decoding crossed its declared member,
GUI extraction followed an existing destination symlink, a command outside a
zero-length declared command area was accepted, and an overflowing ULEB128 was
truncated. The maintained source now rejects the four malformed parser inputs
and extracts an IPA executable into a newly created private directory using
exclusive file creation; the GUI retains and removes that directory at exit.
Each regression fails on v1.0.5 and passes after the
correction. The issue identifiers are:

- ARCHIVELENS-EXISTING-ENCRYPTION-COMMAND
- ARCHIVELENS-EXISTING-FUNCTION-STARTS-SPAN
- ARCHIVELENS-EXISTING-GUI-SYMLINK
- ARCHIVELENS-EXISTING-LOAD-COMMAND-AREA
- ARCHIVELENS-EXISTING-ULEB-OVERFLOW

The six issue identifiers refer to saved observations at source commit
`ea9e4af7e4f800d7cc0ead414a4fc3b9a7c20702`; the five v1.0.6 checks are
separate, current-source reproductions. The interrupted deep source audit has
not resumed. Interactive GUI use, larger parsing cores, platform coverage,
and other input handling remain incomplete. Read the existing
[defensive scope](DEFENSIVE_SCOPE.md) and [origin record](ORIGIN.md) for their
separate limits and third-party attribution. CVP eligibility, applicant identity,
authorized real task, organizational approval and program acceptance remain OPEN.

The prior v1.0.4 and v1.0.5 Release assets are source only. Version 1.0.5 moved the two
historical upstream guide images byte-for-byte to `项目文档/resources/`, where
the retained upstream guide can resolve its image link. No runtime source
changed; any new source Release must be checked against its own tag.
Current version maintenance is by dhtfish98; the attributed upstream derivative
relationship and original author rights remain unchanged.
