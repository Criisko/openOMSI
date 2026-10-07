# Bringing the project's latest onto this fork

`main` here is the project's `main` plus what `FORK.md` lists. Every night
`.github/workflows/sync-upstream.yml` merges the project's `main` into it, and a merge that
conflicts fails that job and leaves the fork exactly where it was - so nothing half-merged is
ever released. This is the same merge by hand, with the traps written down.

## Before the merge

Two files in the working tree belong to this machine and not to the repository: `Cargo.toml`
carries the `wgpu-hal` entry that makes the ReShade workaround possible, and `Cargo.lock`
follows it (see "The workaround" in `FORK.md`). Neither is ever committed. A merge refuses to
start while they are modified, so take them aside first:

```powershell
Copy-Item Cargo.toml, Cargo.lock C:\Users\Desktop\Desktop\_kopie-lokalne\
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
- **Two failures are the machine's, not the code's**, and are worth saying so rather than
  chasing: `a_picked_trip_starts_the_rest_of_the_tour` and `a_duty_passes_its_fleet_number` in
  `omsi-launcher-core` need an OMSI installation to be present.
- **`update-and-build.ps1` is a local file**, and two things in it have gone wrong once. It
  used to ask only whether a `[patch.crates-io]` section existed, and once the project had one
  of its own (for `gpu-allocator`) that check started passing while our entry was missing - a
  release built locally would have crashed under ReShade for everybody; it now looks for our
  entry and puts it inside the section that is there, the way the release workflow does. And its
  `cargo build` wants a local `$ErrorActionPreference = 'Continue'`: with `Stop` PowerShell turns
  cargo's progress on stderr into a terminating error, which ended the script before it copied
  anything into the game folder - the build itself had gone through, and the command line looked
  as if it had failed. The same holds for calling the script from a longer command: let it be
  the last thing in one, or redirect its output to a file rather than through `Select-Object`.

## How to know it went right

Before anything is published, the release workflow checks this ("What the fork adds is still
whole" in `.github/workflows/windows.yml`) and a failure publishes nothing: `dlss.rs` exists,
the three files that read the setting still mention it, and the project's own two contracts
pass. The same by hand:

```powershell
Test-Path crates/omsi-render/src/dlss.rs
cargo test --release -p omsi-launcher-core --lib the_games_options_survive_a_save
cargo test --release -p omsi-app --lib every_setting_is_on_exactly_one_tab
```

`git diff --stat origin/main...HEAD` is the whole answer to "what does this fork add now":
23 files in October 2026, and the list in `FORK.md` should read the same after a merge.
