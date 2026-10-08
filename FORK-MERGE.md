# Bringing the project's latest onto this fork

`main` here is the project's `main` plus what `FORK.md` lists. Every night
`.github/workflows/sync-upstream.yml` merges the project's `main` into it, and a merge that
conflicts fails that job and leaves the fork exactly where it was - so nothing half-merged is
ever released. This is the same merge by hand, with the traps written down.

## Before the merge

Nothing of this machine's own is written into the repository's files any more, so a merge
normally starts on a clean tree: the ReShade workaround is applied by `update-and-build.ps1`
(and by the release workflow) to the `wgpu-hal` source cargo itself fetched, and no
`[patch.crates-io]` entry for it is written into `Cargo.toml` any more (see "The workaround"
in `FORK.md`).

A tree that still carries one, from a build made before this changed, wants a
`git diff Cargo.toml Cargo.lock` first: a merge refuses to start while either file is
modified, and a build can leave `Cargo.lock` looking modified when its contents are the same,
because cargo writes its line endings its own way.

```powershell
git checkout -- Cargo.toml Cargo.lock
```

## The merge

```powershell
git fetch origin
git log --oneline origin/main..fork-main      # what is ours
git diff --stat origin/main...fork-main       # and in which files
git merge origin/main --no-commit
git diff --name-only --diff-filter=U          # the conflicts
```

Every conflict is resolved the same way: **the project's version, with what is ours put back
into it** - what is ours is the list in `FORK-DLSS.md` and nothing else.

`crates/omsi-app/src/settings.rs` is the fiddly one. Both sides rewrite the same `Self { ... }`
defaults line, which is a single line of two thousand characters, so by hand it is not
practical: take the project's line and insert the pieces DLSS needs (`dlss: "off".into(), `
before `render_scale: 0.0, `, and the others down the file).

Two tools in `C:\Users\Desktop\Desktop\_kopie-lokalne` do this, and both are written in Python
rather than PowerShell - PowerShell cannot even parse `'<<<<<<< HEAD'`, and a two-thousand
character line wants a script rather than an editor:

- `pokaz-konflikty.py` prints, for every conflict block, only the part where the two sides
  differ (`pelne` as an argument prints the blocks whole). This is what makes a conflict in
  `settings.rs` readable at all.
- `rozwiaz-konflikty.py` resolves every block by a rule that names a fragment of the *project's*
  side and says what to do with it, and it stops if a block matches no rule, so nothing is
  settled in silence. **The rules are different every time** - they say what moved around our
  code since the last merge - so it is a template rather than a tool: read it, then rewrite the
  rules for the merge at hand. The one it replaced is kept beside it with the date in its name.

Then:

```powershell
git add -A
git commit -m "Merge remote-tracking branch 'origin/main' into the fork"
.\update-and-build.ps1 -SkipPull -SkipCopy     # puts the workaround back and builds
cargo test --release --workspace --no-fail-fast
```

## What a merge can surprise you with

- **A dependency the project moves into the workspace can take one of ours with it.** The
  October 2026 merge brought `windows` into the workspace and dropped `windows-core` from
  `crates/omsi-render/Cargo.toml`; the `#[windows::core::implement]` macro in `dlss.rs`
  expands to code naming that crate, so the renderer stopped compiling. It has to be a direct
  dependency there even though nothing imports it by name.
- **A test can hold a property our change did not know it had.** The project rewrites the
  scene shader's arrays into textures for the devices whose vertex stage cannot read a storage
  buffer, and the test that guards it rejects the plain `models[` indexing. `prev_models`,
  which the DLSS motion pass added, was not part of that rewrite, so the shader no longer
  translated and `the_scene_shader_reads_its_arrays_from_textures_without_vertex_storage`
  failed. A new array belongs in `arrays_as_textures` beside the others - and **without** the
  `@group(3) @binding(1)` prefix, or the texture it makes comes out with no binding.
- **The project can split a file we have a piece in.** The October 2026 merge took the
  renderer's one `lib.rs` apart into `passes/` and `pipelines/` modules, so what the DLSS
  change had added there had to be carried into those: `crates/omsi-render/src/pipelines/dlss.rs`
  is new, and `passes/setup.rs`, `passes/prepass.rs` and `passes/post.rs` each took a piece.
  Inside a line the project wrote, the vertex entry's parameters are `in_pos`, `in_normal` and
  `in_uv` - an `in.pos` left in a function that takes those by value is not a warning but nine
  failing tests, all of them saying `no definition in scope for identifier: 'in'`.
- **A new `OMSI_*` name has to be declared where the project keeps its flags.**
  `flags_match_source` in `omsi-cfg` reads the code and wants every name it mentions in
  `crates/omsi-cfg/src/flags.rs` (that table is sorted, and searched by binary search), while
  `flags_doc_up_to_date` wants `docs/DEBUG_FLAGS.md` in step with it: `$env:OMSI_FLAGS_BLESS =
  '1'; cargo test --release -p omsi-cfg --lib flags`. The five DLSS ones (`OMSI_DLSS`,
  `OMSI_DLSS_JITTER_SIGN`, `OMSI_DLSS_PROJECT_ID`, `OMSI_DLSS_VERBOSE`, `OMSI_STREAMLINE_DIR`)
  were carried over in this merge.
- **Two failures are the machine's, not the code's**, and are worth saying so rather than
  chasing: `a_picked_trip_starts_the_rest_of_the_tour` and `a_duty_passes_its_fleet_number` in
  `omsi-launcher-core` need an OMSI installation to be present.
- **`update-and-build.ps1` is a local file**, and its `cargo build` wants a local
  `$ErrorActionPreference = 'Continue'`: with `Stop` PowerShell turns cargo's progress on
  stderr into a terminating error, which ended the script before it copied anything into the
  game folder - the build itself had gone through, and the command line looked as if it had
  failed. The same holds for calling the script from a longer command: let it be the last
  thing in one, or redirect its output to a file rather than through `Select-Object`.
- **Where the workaround is applied depends on where `wgpu-hal` comes from, and the wrong
  place is worse than no workaround.** The October 2026 merge met a project that patches that
  crate to the org's own wgpu fork, and the whole set of wgpu crates then comes from that one
  source: a patched copy of the crate standing beside it is a *second* `wgpu-hal` in the same
  build, and the fork's `wgpu` does not recognise its types (`the trait bound
  wgpu_hal::dx12::Api: wgpu::wgpu_hal::Api is not satisfied`, and one missing method after
  another); a copy of the fork's whole repository put inside this workspace is worse still,
  because it inherits from *this* workspace and stops at `error inheriting authors`. The line
  is taken out of the checkout cargo fetched now.
- **Cargo does not notice a file edited inside a git dependency's checkout.** That dependency
  is fingerprinted by the revision it fetched, not by the files in it, so the older `wgpu-hal`
  artifact is reused and the workaround silently is not in the build - the game then dies in
  `dxgi.dll` (`0xc0000005`) under ReShade before any window shows, which looks exactly like the
  workaround never having been written. `cargo clean -p wgpu-hal` after patching, before the
  build, is what makes it real, and what checks it is the *binary*: the id of the call that was
  removed (`IDXGIFactoryMedia`, `{41E7D1F2-A591-4F7B-A2E5-FA9C843E1C12}`) must not be in
  `openomsi.exe` at all. A build that has it is a red run, not a release.

## How to know it went right

Before anything is published, the release workflow checks this ("What the fork adds is still
whole" in `.github/workflows/windows.yml`) and a failure publishes nothing: `dlss.rs` and the
motion pipelines beside it exist, the three files that read the setting still mention it, and
the project's own three contracts pass. The same by hand:

```powershell
Test-Path crates/omsi-render/src/dlss.rs
Test-Path crates/omsi-render/src/pipelines/dlss.rs
cargo test --release -p omsi-launcher-core --lib the_games_options_survive_a_save
cargo test --release -p omsi-app --lib every_setting_is_on_exactly_one_tab
cargo test --release -p omsi-cfg --lib flags
```

`git diff --stat origin/main...HEAD` is the whole answer to "what does this fork add now":
32 files in October 2026, and the list in `FORK.md` should read the same after a merge.
