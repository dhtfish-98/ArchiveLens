> 目录已整理：文档在「项目文档」，构建、缓存与暂存输入在「Build」。从仓库根目录运行 `python3 构建.py --build`；如需使用本文原有源码命令，先运行 `python3 构建.py --stage --ci`，再进入 `Build/源码`。暂存会恢复原输入路径。现有版本和历史验证记录按各自提交理解。

# ArchiveLens

防御用途、实际能力及本轮验证范围见 [DEFENSIVE_SCOPE.md](<DEFENSIVE_SCOPE.md>)。

ArchiveLens is a Rust derivative of [JRBusiness/REipa](https://github.com/JRBusiness/REipa) with renamed owned source files and symbols. It preserves the upstream feature set within the tested contracts. See [source/license record](<ORIGIN.md>), [verification record](<VALIDATION.md>) and [complete mapping](<../RENAME_MAP.json>).

`workspace/crates` contains the Mach-O loader, image view, Objective-C/Swift metadata and ARM64 decoding/analysis layers. `workspace/bin` contains the CLI, benchmark and GUI adapters. `bench` contains the Capstone comparison tools.

The command names, flags, positional metavariables, CSV fields and parsed data layouts are fixed explicitly. Fifteen manual Debug implementations retain upstream diagnostic labels. The GUI resolves the renamed sibling CLI. Executables are `archivelens`, `lens-bench` and `lens-gui`.

## Build

On macOS with the command-line tools (Rust stable is also needed for ArchiveLens):

```sh
cargo build --release --workspace --locked --manifest-path workspace/Cargo.toml
```

## Test and independently consume

```sh
cargo test --workspace --locked --manifest-path workspace/Cargo.toml
```

To compare against the pinned original, clone the upstream repository into `../upstream`, check out the commit in ORIGIN.md, then run:

```sh
python3 verification/lens_upstream_lock.py ../upstream
python3 verification/lens_contract.py --upstream ../upstream
```

The contract script builds and runs the original and derivative, generates neutral fixtures, checks outputs/files and compiles a separate consumer against the public renamed API. `.github/workflows/source-contracts.yml` repeats this with an upstream checkout pinned to the recorded commit.

The original and modified full workspaces built successfully. All 104 existing tests passed in the modified workspace; 104 original core/CLI tests passed, and the original GUI build passed. The release workspace built. Contract comparison passed 54 checks: 9 command families on raw Mach-O, an IPA containing that executable, malformed/missing inputs, help/flags, a project export, and independently compiled consumers over 50,000 ARM64 words, 20 type encodings and 5 method signatures. Two independently invoked Capstone scripts produced the same deterministic results (wall clock throughput excluded).

Runtime/integration boundaries are listed in VALIDATION.md. Built packages are distributed with the complete corresponding source package and original notices.


## Current source version

The maintained source bundle and all eight local Cargo packages are version
**1.0.3**. See [current version and open limits](VERSION_STATUS.md). This metadata
alignment does not close the six recorded issues or establish CVP eligibility.
