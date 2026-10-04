#!/usr/bin/env python3
"""Match resolved dependencies while retaining the upstream workspace version."""
import pathlib
import re
import sys

PACKAGES = [('archivelens', 'reipa'), ('lens-macho', 'reipa-macho'),
            ('lens-image', 'reipa-image'), ('lens-objc', 'reipa-objc'),
            ('lens-swift', 'reipa-swift'), ('lens-arm64', 'reipa-arm64'),
            ('lens-bench', 'reipa-bench'), ('lens-gui', 'reipa-gui')]


def remap_lock(text, upstream_version):
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?', upstream_version):
        raise ValueError('Unsupported upstream workspace version')
    for new, old in PACKAGES:
        text = text.replace('"' + new + '"', '"' + old + '"')
    for _, old in PACKAGES:
        pattern = r'(\[\[package\]\]\nname = "' + re.escape(old) + r'"\nversion = ")[^"\n]+("\n)'
        text, count = re.subn(pattern, lambda m: m[1] + upstream_version + m[2], text)
        if count != 1:
            raise ValueError('Expected exactly one mapped local package: ' + old)
    return text


def workspace_version(manifest):
    section = re.search(r'(?ms)^\[workspace\.package\][ \t]*\n(.*?)(?=^\[|\Z)', manifest)
    versions = re.findall(r'^version[ \t]*=[ \t]*"([^"\n]+)"[ \t]*$', section[1], re.M) if section else []
    if len(versions) != 1:
        raise ValueError('Expected one explicit upstream workspace version')
    return versions[0]


def main():
    root = pathlib.Path(__file__).resolve().parents[1]
    upstream = pathlib.Path(sys.argv[1]) / 'reipa'
    version = workspace_version((upstream / 'Cargo.toml').read_text())
    text = remap_lock((root / 'workspace/Cargo.lock').read_text(), version)
    (upstream / 'Cargo.lock').write_text(text)


if __name__ == '__main__':
    main()
