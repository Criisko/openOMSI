# DLSS, kept here on purpose

The DLSS Super Resolution / DLAA change lives in the project's pull request
[#942](https://github.com/openOMSI-Project/openOMSI/pull/942), which is not merged upstream.
This fork carries it, so that is the one thing a merge here must never quietly lose. Two
copies of it are kept in *this* repository, so that closing or deleting the pull request on
GitHub cannot take it away:

- `FORK-DLSS.patch` - the whole change as one patch, ready to apply;
- the branch `dlss-upstream` and the tag `dlss-pr942-2026-10-03`, both on the same commit in
  this fork's own GitHub repository.

## What the patch is

The author's commit `6eeaff7` ("DLSS Super Resolution and DLAA through NVIDIA Streamline",
Nussaccino, taken on 2026-10-03), which sits on the project's `1ef9649`: fifteen files,
2233 insertions. `crates/omsi-render/src/dlss.rs` and `dlss_motion.wgsl` are the runtime,
the rest is its loading, the motion vectors of the depth prepass, the setting and the
launcher's rows. Nothing of NVIDIA's is in it - the player supplies the DLLs, as the user
guide says.

`git apply --check` of the patch against a worktree of `1ef9649` succeeds: it applies
exactly to the commit it was made on.

## Bringing DLSS onto a newer project version

```powershell
git fetch https://github.com/Criisko/openOMSI.git dlss-upstream
git merge dlss-upstream
```

That is a normal merge of a feature branch on top of the project's `main`, so a conflict
can only be where the project changed the same lines - the one place worth reading. From
this clone, `git am FORK-DLSS.patch` on a tree of the project's `main` does the same.

## Which revision this fork's `main` carries

An earlier revision of the same work, the one the game here has been running: its motion
shaders are still inside `shader.wgsl` instead of `dlss_motion.wgsl`, and its language
strings are the earlier set. That revision is in this fork's own history - it arrived with
the first commit of the fork's history, which is the author's branch as it stood then - and
is tagged `dlss-ours-2026-10-03`. Its `dlss.rs`, `upscale.wgsl`, `shader.wgsl` and
`crates/omsi-render/Cargo.toml` are byte-identical to that earlier commit.

The project split the renderer's one file into `passes/` and `pipelines/` modules with
0.2.20. The merge that brought 0.2.20 here carried this change over into that shape -
`crates/omsi-render/src/pipelines/dlss.rs` builds the motion pipelines, `passes/setup.rs`
sizes the frames and records the jitter, `passes/prepass.rs` draws the motion colours, and
`passes/post.rs` hands the picture to Streamline - so a later merge reads those files, not
the one `lib.rs` the patch was written against.

## The rest of the arrangement

`.github/workflows/sync-upstream.yml` merges the project's `main` into this fork's `main`
every night; a merge that conflicts fails that job and leaves the fork exactly where it was.
Before `.github/workflows/windows.yml` publishes anything, it checks that what this fork adds
is still whole: `crates/omsi-render/src/dlss.rs` is there, the word `dlss` still appears in the
three files that read or write the setting (`crates/omsi-app/src/settings.rs`,
`crates/omsi-app/src/launcher/pages.rs`, `crates/omsi-launcher-core/src/lib.rs`), and the
project's own three contracts pass (the settings round trip, every launcher row named in
`by_tab()`, and every `OMSI_*` name in the code declared in `omsi_cfg::flags`). A failure
there publishes nothing, so a merge that silently dropped part of a
change is a red run and no update, rather than a release without the feature.

## The pieces a merge has to keep

The files are listed in `FORK.md`; these are the pieces inside them, which is what a conflict
or a merge that reads suspiciously should be checked against.

- `crates/omsi-render/src/dlss.rs` - the whole runtime: Streamline's DLLs loaded from beside
  the game, the swap chain watched, the options set, and the present paced.
- `crates/omsi-render/src/lib.rs` - `DlssMode` and its round trip, `RenderOptions::dlss`,
  `DlssState` and `DlssPipelines`, the present (`dlss_pending`, `dlss_present`), and the
  `prev_models` swap in `arrays_as_textures`.
- `crates/omsi-render/src/pipelines/dlss.rs` - the motion pipelines: their bind group, the
  `DLSS_MOTION_FORMAT` target, the six prepass pipelines and the sky's, built on demand where
  the project builds its own.
- `crates/omsi-render/src/passes/setup.rs` - the frame's sizes and targets (`dlss_start`,
  `dlss_targets`, `scene_size`), the Halton jitter and the `MotionUniform` in
  `frame_uniforms`.
- `crates/omsi-render/src/passes/prepass.rs` - the motion colour attachment, and the sky's
  motion draw.
- `crates/omsi-render/src/passes/post.rs` - `encode_upscale` leaves the picture to Streamline
  (`dlss_end_of_scene`) instead of filtering it, and post keeps its FXAA off.
- `crates/omsi-render/src/passes/mod.rs` - `FrameCtx::dlss_frame` and `full_h`.
- `crates/omsi-render/src/shader.wgsl` - the entry points `vs_motion`, `fs_motion`,
  `fs_motion_test`, `fs_motion_transmap`, `vs_motion_sky`, `fs_motion_sky`, the `motion`
  uniform block, `@group(3) @binding(1) prev_models`, and `motion_pixels()`.
- `crates/omsi-render/src/upscale.wgsl` - the upscale's mode 2.
- `crates/omsi-render/Cargo.toml` - the `windows` features (`Win32_Graphics_Dxgi_Common`,
  `Direct3D12`, `System_LibraryLoader`), `windows-core`, and `wgpu-hal` with `dx12`.
- `crates/omsi-cfg/src/flags.rs`, `docs/DEBUG_FLAGS.md` - the five flags `OMSI_DLSS`,
  `OMSI_DLSS_JITTER_SIGN`, `OMSI_DLSS_PROJECT_ID`, `OMSI_DLSS_VERBOSE` and
  `OMSI_STREAMLINE_DIR`, declared in the project's own flag table, because its test wants
  every `OMSI_*` name the code mentions to be there and the document in step with it.
- `crates/omsi-app/src/settings.rs` - `pub dlss: String`, `dlss_mode()` with its `OMSI_DLSS`
  override, the `dlss: "off"` default, the `dlss={}` line the file is written with, and the
  arm that parses it back (which clamps anything unknown to `off`).
- `crates/omsi-app/src/launcher/pages.rs` - `dlss_on()`, `aa_setting()` (the row that offers
  MSAA or DLSS / DLAA), the `s-dlss` quality row, which takes the render scale row's place,
  and the `s-dlss` id in `by_tab()`.
- `crates/omsi-app/src/startup.rs` - DirectX 12 is wanted when the headset or DLSS asks for it.
- `crates/omsi-app/src/launcher/mod.rs` - the showroom's `RenderOptions` turns DLSS off.
- `crates/omsi-launcher-core/src/lib.rs` - `dlss_mode()`, the default in the options list, the
  arm that reads the file's value, the line that is written back, and
  `dlss_settings_round_trip`.
- `crates/omsi-app/locales/app.yml` - the strings, in every language the launcher has.
- `docs/USER_GUIDE.md` - what the player is told about the setting and the DLLs beside the
  game.
