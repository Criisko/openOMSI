# This is a personal fork

It exists for one reason: the Windows build here carries a small workaround that lets
**ReShade** load into the game. Otherwise it is the project's own code - in particular it is
`main` plus the DLSS Super Resolution / DLAA pull request
([#942](https://github.com/openOMSI-Project/openOMSI/pull/942)), which is not merged upstream yet.

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

It is a bug in ReShade rather than in wgpu or openOMSI: `CreateDXGIFactory1` with the
`IDXGIFactory1` IID works fine under ReShade, while the same call with the `IDXGIFactoryMedia`
IID does not. Most engines never call the legacy entry point, which is why ReShade works in most
games and not here.

## Releases

Windows x64 only. Every push to `main` builds one and publishes it, so the launcher can update
itself from this repository (`OMSI_UPDATE_URL` points at these releases). The archive keeps the
name the launcher's updater looks for.

The fork keeps the project's own pipeline (`.github/workflows/release.yml`) renamed to
`release.yml.off`: its Android job wants `secrets.ANDROID_KEYSTORE_B64`, which a fork does not
have, and the project's release job waits for every platform - so one missing secret would mean
no release at all.

## Upstream

The real project, with the issues, the documentation and the discussions:
<https://github.com/openOMSI-Project/openOMSI>
