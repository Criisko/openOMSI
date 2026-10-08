# This is a personal fork

It exists for one reason: the Windows build here carries a small workaround that lets
**ReShade** load into the game. Otherwise it is the project's own code - in particular it is
`main` plus the DLSS Super Resolution / DLAA pull request
([#942](https://github.com/openOMSI-Project/openOMSI/pull/942)), which is not merged upstream yet.

That change is also kept in this repository on its own - as a patch, a branch and a tag - so
that closing or deleting the pull request cannot lose it, and so that it can be brought onto
a newer project version in one merge: see `FORK-DLSS.md`.

Nothing of NVIDIA's is included. The DLSS runtime (Streamline's `sl.interposer.dll`,
`sl.common.dll`, `sl.dlss.dll` and `nvngx_dlss.dll`) has to be supplied by the player, exactly as
the project's own documentation says.

## The workaround, and why it is needed

`wgpu` makes `IDXGIFactoryMedia` through the legacy `CreateDXGIFactory1`. ReShade hooks that
legacy export, and the process then dies with an access violation inside `dxgi.dll`
(`0xc0000005`) before any window shows - the game never starts. openOMSI never makes a
composition swapchain, so that factory is not needed at all, and wgpu itself treats the call as
optional (it is wrapped in `.ok()` upstream). The step leaves it out; see
`.github/workflows/windows.yml`, "The ReShade workaround".

Which copy of that crate is patched matters. The project patches `wgpu-hal` to the org's own
wgpu fork (`Cargo.toml`), and the whole set of wgpu crates then comes from that one source: a
patched copy standing beside it is a second `wgpu-hal` in the same build, and the fork's `wgpu`
does not recognise its types. So the line is taken out of the source cargo itself fetched,
before the build, and a step after the build checks that it is still out. A project that ever
stops patching that crate is the case the registry copy and the `[patch.crates-io]` entry are
still kept for.

It is a bug in ReShade rather than in wgpu or openOMSI: `CreateDXGIFactory1` with the
`IDXGIFactory1` IID works fine under ReShade, while the same call with the `IDXGIFactoryMedia`
IID does not. Most engines never call the legacy entry point, which is why ReShade works in most
games and not here.

## Releases

Windows x64 only. Every push to `main` builds one and publishes it, and the build's own update
check asks *this* repository for it (`crates/omsi-app/src/updater.rs`, `LATEST_API`), so the
launcher hands the player the newest project code together with the workaround above. Nothing to
set up on the player's machine: `OMSI_UPDATE_URL` only overrides the address for tests. What has
to stay current is `main`, and `.github/workflows/sync-upstream.yml` merges the project's `main`
into it every night - a merge that conflicts fails that job and leaves the fork exactly where it
was.

The archive keeps the name the launcher's updater looks for.

The fork keeps the project's own pipeline (`.github/workflows/release.yml`) renamed to
`release.yml.off`: its Android job wants `secrets.ANDROID_KEYSTORE_B64`, which a fork does not
have, and the project's release job waits for every platform - so one missing secret would mean
no release at all.

## What this fork adds, file by file

`git diff --stat origin/main...HEAD` is the whole answer - 32 files in October 2026 - and this
is what each of them is for. Anything not on this list is the project's own code, so a merge
that turns up a difference somewhere else has found something to read rather than to keep. The
tables are the list; the number is only there to tell you at a glance whether something new has
appeared.

**DLSS**, kept here on purpose (`FORK-DLSS.md` says what it is and where it touches the
project):

| file | what is there |
|---|---|
| `crates/omsi-render/src/dlss.rs` | the runtime - new file |
| `crates/omsi-render/src/pipelines/dlss.rs` | the motion pipelines, built where the project builds its own - new file |
| `crates/omsi-render/src/pipelines/mod.rs` | the module that file is declared in |
| `crates/omsi-render/src/passes/setup.rs` | the frame's sizes and targets, the jitter, and what is still to be presented |
| `crates/omsi-render/src/passes/prepass.rs` | the motion colours and the sky's motion draw |
| `crates/omsi-render/src/passes/post.rs` | the upscale hands the picture to Streamline instead of filtering it |
| `crates/omsi-render/src/passes/mod.rs` | the frame context's `dlss_frame` and `full_h` |
| `crates/omsi-render/src/lib.rs` | the mode, the pipeline state, the present |
| `crates/omsi-render/src/shader.wgsl` | the motion entry points the prepass draws with |
| `crates/omsi-render/src/upscale.wgsl` | the upscale pass's DLSS path |
| `crates/omsi-render/Cargo.toml` | the Windows features and the crates it needs |
| `crates/omsi-app/src/settings.rs` | the setting, its spelling and its round trip |
| `crates/omsi-app/src/launcher/pages.rs` | the row that offers it, in place of the render scale |
| `crates/omsi-app/src/startup.rs` | DirectX 12 when it is on |
| `crates/omsi-app/src/launcher/mod.rs` | the showroom draws without it |
| `crates/omsi-app/locales/app.yml` | the strings |
| `crates/omsi-launcher-core/src/lib.rs` | the mode where the launcher reads and writes it |
| `crates/omsi-cfg/src/flags.rs`, `docs/DEBUG_FLAGS.md` | the five flags that drive it, declared where the project keeps every flag |
| `docs/USER_GUIDE.md`, `CHANGELOG.md` | what the player is told |

**This fork's own pipeline**, so that a build from here updates itself and carries the
workaround:

| file | what is there |
|---|---|
| `.github/workflows/windows.yml` | the release: the ReShade workaround, the check that all of the above is still whole, the zip |
| `.github/workflows/sync-upstream.yml` | the nightly merge of the project's `main` |
| `.github/workflows/release.yml.off` | the project's own release, renamed away |
| `.gitattributes` | the changelog merges as a union, the settings may not, and why |
| `crates/omsi-app/src/updater.rs` | the update check asks this fork's releases |
| `crates/omsi-app/src/launcher/update.rs` | the repository it shows the player |
| `Cargo.lock` | the crates `omsi-render` gained for DLSS |

**These documents**: `FORK.md`, `FORK-DLSS.md`, `FORK-DLSS.patch`, `FORK-MERGE.md`.

Bringing the project's latest onto this fork is written down step by step in `FORK-MERGE.md`.

## Upstream

The real project, with the issues, the documentation and the discussions:
<https://github.com/openOMSI-Project/openOMSI>
