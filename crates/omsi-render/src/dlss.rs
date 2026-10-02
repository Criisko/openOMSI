// The Streamline structures, constants and identifiers below are transcribed from NVIDIA's
// Streamline SDK headers (https://github.com/NVIDIA-RTX/Streamline), which come under this
// licence:
//
// Copyright (c) 2022-2024 NVIDIA CORPORATION. All rights reserved
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! NVIDIA DLSS Super Resolution and DLAA through NVIDIA Streamline (DirectX 12, GeForce RTX).
//!
//! Streamline's interposer (`sl.interposer.dll`) and its plugins (`sl.common.dll`,
//! `sl.dlss.dll`, NVIDIA's `nvngx_dlss.dll`) are loaded at run time from beside the game (or
//! `OMSI_STREAMLINE_DIR`): nothing of NVIDIA's is linked into openOMSI or shipped with it, and
//! a machine without them simply draws as before. The structures below are the C layout of
//! the Streamline 2.14 headers (`sl_core_types.h`, `sl_consts.h`, `sl_dlss.h`).
//!
//! wgpu keeps its own D3D12 command lists; DLSS is recorded into one of ours and executed on
//! wgpu's queue between the frame's scene submit and its HUD submit (as the OpenXR path
//! does it), so queue order puts it after the picture it reads and before the HUD drawn
//! over its result. Streamline is told the state each tagged resource is in (wgpu's legacy
//! barriers leave it in the one of its last use) and returns it in that state.

#[cfg(not(windows))]
use anyhow::Result;

/// DLSS quality mode: the share of the window's size the 3D picture is drawn at before DLSS
/// brings it up to the window (DLAA: the window's own size, DLSS as anti-aliasing only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DlssMode {
    #[default]
    Off,
    Dlaa,
    Quality,
    Balanced,
    Performance,
    UltraPerformance,
}

impl DlssMode {
    /// The settings' word for a mode (`dlss=` in settings.cfg).
    pub fn parse(s: &str) -> DlssMode {
        match s.trim().to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "dlaa" | "native" => DlssMode::Dlaa,
            "quality" | "on" | "1" | "true" => DlssMode::Quality,
            "balanced" => DlssMode::Balanced,
            "performance" => DlssMode::Performance,
            "ultra_performance" | "ultraperformance" => DlssMode::UltraPerformance,
            _ => DlssMode::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DlssMode::Off => "off",
            DlssMode::Dlaa => "dlaa",
            DlssMode::Quality => "quality",
            DlssMode::Balanced => "balanced",
            DlssMode::Performance => "performance",
            DlssMode::UltraPerformance => "ultra_performance",
        }
    }

    pub fn is_on(self) -> bool {
        self != DlssMode::Off
    }

    /// The share of the window's width and height the picture is drawn at when Streamline
    /// does not say (NVIDIA's published ratios).
    pub fn scale(self) -> f32 {
        match self {
            DlssMode::Off | DlssMode::Dlaa => 1.0,
            DlssMode::Quality => 0.667,
            DlssMode::Balanced => 0.58,
            DlssMode::Performance => 0.5,
            DlssMode::UltraPerformance => 0.333,
        }
    }
}

/// What DLSS reads and writes this frame: the D3D12 resources behind wgpu's textures.
pub struct Inputs<'a> {
    /// The 3D picture at the render size (the renderer's colour format, sRGB).
    pub color: &'a wgpu::Texture,
    /// The depth prepass's depth (reversed Z) at the render size.
    pub depth: &'a wgpu::Texture,
    /// Motion vectors (Rg32Float, pixels towards the last frame) at the render size.
    pub motion: &'a wgpu::Texture,
    /// The `BiasCurrentColor` mask (R8Unorm, 1: the current picture over the history) at
    /// the render size.
    pub bias: &'a wgpu::Texture,
    /// The result at the window's size (Rgba16Float, storage).
    pub output: &'a wgpu::Texture,
    pub render_size: (u32, u32),
    pub output_size: (u32, u32),
}

/// The camera of this frame for DLSS: every matrix as glam's columns (which are the rows of
/// Streamline's row-vector matrices), without the jitter.
pub struct FrameConstants {
    pub view_to_clip: [[f32; 4]; 4],
    pub clip_to_view: [[f32; 4]; 4],
    pub clip_to_prev_clip: [[f32; 4]; 4],
    pub prev_clip_to_clip: [[f32; 4]; 4],
    /// The sub-pixel offset the projection was moved by, in pixels.
    pub jitter: [f32; 2],
    pub camera_pos: [f32; 3],
    pub camera_up: [f32; 3],
    pub camera_right: [f32; 3],
    pub camera_fwd: [f32; 3],
    pub near: f32,
    pub far: f32,
    /// Vertical field of view (radians).
    pub fov: f32,
    pub aspect: f32,
    /// The history is not the same scene (a jump, a new size, the first frame).
    pub reset: bool,
}

#[cfg(windows)]
pub use imp::Runtime;

#[cfg(not(windows))]
pub struct Runtime;

#[cfg(not(windows))]
impl Runtime {
    pub fn load(_device: &wgpu::Device, _queue: &wgpu::Queue) -> Result<Runtime> {
        anyhow::bail!("DLSS needs Windows with DirectX 12")
    }
    pub fn render_size(&mut self, mode: DlssMode, out: (u32, u32)) -> (u32, u32) {
        fallback_render_size(mode, out)
    }
    pub fn evaluate(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _mode: DlssMode, _inputs: &Inputs, _c: &FrameConstants) -> Result<()> {
        anyhow::bail!("DLSS needs Windows with DirectX 12")
    }
}

/// The render size of a mode by NVIDIA's ratios (whole pixels, at least one).
pub fn fallback_render_size(mode: DlssMode, (w, h): (u32, u32)) -> (u32, u32) {
    let s = mode.scale();
    (((w as f32 * s).round() as u32).max(1), ((h as f32 * s).round() as u32).max(1))
}

#[cfg(windows)]
mod imp {
    use super::{fallback_render_size, DlssMode, FrameConstants, Inputs};
    use anyhow::{anyhow, bail, Context, Result};
    use std::ffi::{c_char, c_void, CStr};
    use windows::core::{Interface, PCSTR, PCWSTR};
    use windows::Win32::Foundation::{FreeLibrary, HANDLE, HMODULE};
    use windows::Win32::Graphics::Direct3D12::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::Win32::Graphics::Dxgi::{IDXGIDeviceSubObject_Impl, IDXGIObject_Impl, IDXGIOutput, IDXGISwapChain, IDXGISwapChain_Impl, DXGI_FRAME_STATISTICS, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_CHAIN_FLAG};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR};

    // --- Streamline 2.14's C layout

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct StructType {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Base {
        next: *mut c_void,
        struct_type: StructType,
        struct_version: usize,
    }

    impl Base {
        const fn new(struct_type: StructType, struct_version: usize) -> Base {
            Base { next: std::ptr::null_mut(), struct_type, struct_version }
        }
    }

    const PREFERENCES: StructType = StructType { data1: 0x1ca10965, data2: 0xbf8e, data3: 0x432b, data4: [0x8d, 0xa1, 0x67, 0x16, 0xd8, 0x79, 0xfb, 0x14] };
    const RESOURCE: StructType = StructType { data1: 0x3a9d70cf, data2: 0x2418, data3: 0x4b72, data4: [0x83, 0x91, 0x13, 0xf8, 0x72, 0x1c, 0x72, 0x61] };
    const RESOURCE_TAG: StructType = StructType { data1: 0x4c6a5aad, data2: 0xb445, data3: 0x496c, data4: [0x87, 0xff, 0x1a, 0xf3, 0x84, 0x5b, 0xe6, 0x53] };
    const VIEWPORT_HANDLE: StructType = StructType { data1: 0x171b6435, data2: 0x9b3c, data3: 0x4fc8, data4: [0x99, 0x94, 0xfb, 0xe5, 0x25, 0x69, 0xaa, 0xa4] };
    const ADAPTER_INFO: StructType = StructType { data1: 0x0677315f, data2: 0xa746, data3: 0x4492, data4: [0x9f, 0x42, 0xcb, 0x61, 0x42, 0xc9, 0xc3, 0xd4] };
    const CONSTANTS: StructType = StructType { data1: 0xdcd35ad7, data2: 0x4e4a, data3: 0x4bad, data4: [0xa9, 0x0c, 0xe0, 0xc4, 0x9e, 0xb2, 0x3a, 0xfe] };
    const DLSS_OPTIONS: StructType = StructType { data1: 0x6ac826e4, data2: 0x4c61, data3: 0x4101, data4: [0xa9, 0x2d, 0x63, 0x8d, 0x42, 0x10, 0x57, 0xb8] };
    const DLSS_OPTIMAL_SETTINGS: StructType = StructType { data1: 0xef1d0957, data2: 0xfd58, data3: 0x4df7, data4: [0xb5, 0x04, 0x8b, 0x69, 0xd8, 0xaa, 0x6b, 0x76] };

    /// `sl::kSDKVersion` of the headers this layout is taken from (2.14.1).
    const SDK_VERSION: u64 = (2 << 48) | (14 << 32) | (1 << 16) | 0xfedc;
    const FEATURE_DLSS: u32 = 0;
    const BUFFER_DEPTH: u32 = 0;
    const BUFFER_MOTION_VECTORS: u32 = 1;
    const BUFFER_SCALING_INPUT_COLOR: u32 = 3;
    const BUFFER_SCALING_OUTPUT_COLOR: u32 = 4;
    const BUFFER_BIAS_CURRENT_COLOR_HINT: u32 = 29;
    /// `ResourceLifecycle::eValidUntilEvaluate`
    const VALID_UNTIL_EVALUATE: u32 = 2;
    const FLAG_DISABLE_CL_STATE_TRACKING: u64 = 1 << 0;
    const FLAG_DISABLE_DEBUG_TEXT: u64 = 1 << 1;
    const FLAG_USE_MANUAL_HOOKING: u64 = 1 << 2;
    const FLAG_FRAME_BASED_TAGGING: u64 = 1 << 7;
    const FALSE: u8 = 0;
    const TRUE: u8 = 1;
    /// openOMSI's own project id for NVIDIA's NGX (engine type "custom" asks for one).
    const PROJECT_ID: &CStr = c"6f3c2a8e-1d4b-4e7a-9c52-0b8d7e1f4a36";

    #[repr(C)]
    struct Preferences {
        base: Base,
        show_console: bool,
        log_level: u32,
        paths_to_plugins: *const *const u16,
        num_paths_to_plugins: u32,
        path_to_logs_and_data: *const u16,
        allocate_callback: *const c_void,
        release_callback: *const c_void,
        log_message_callback: Option<unsafe extern "C" fn(u32, *const c_char)>,
        flags: u64,
        features_to_load: *const u32,
        num_features_to_load: u32,
        application_id: u32,
        engine: u32,
        engine_version: *const c_char,
        project_id: *const c_char,
        render_api: u32,
    }

    #[repr(C)]
    struct Resource {
        base: Base,
        kind: u8,
        native: *mut c_void,
        memory: *mut c_void,
        view: *mut c_void,
        state: u32,
        width: u32,
        height: u32,
        native_format: u32,
        mip_levels: u32,
        array_layers: u32,
        gpu_virtual_address: u64,
        flags: u32,
        usage: u32,
        internal_flags: u16,
        reserved: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Extent {
        top: u32,
        left: u32,
        width: u32,
        height: u32,
    }

    #[repr(C)]
    struct ResourceTag {
        base: Base,
        resource: *mut Resource,
        kind: u32,
        lifecycle: u32,
        extent: Extent,
    }

    #[repr(C)]
    struct ViewportHandle {
        base: Base,
        value: u32,
    }

    #[repr(C)]
    struct AdapterInfo {
        base: Base,
        device_luid: *mut u8,
        device_luid_size_in_bytes: u32,
        vk_physical_device: *mut c_void,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Float2 {
        x: f32,
        y: f32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Float3 {
        x: f32,
        y: f32,
        z: f32,
    }

    #[repr(C)]
    struct Constants {
        base: Base,
        camera_view_to_clip: [[f32; 4]; 4],
        clip_to_camera_view: [[f32; 4]; 4],
        clip_to_lens_clip: [[f32; 4]; 4],
        clip_to_prev_clip: [[f32; 4]; 4],
        prev_clip_to_clip: [[f32; 4]; 4],
        jitter_offset: Float2,
        mvec_scale: Float2,
        camera_pinhole_offset: Float2,
        camera_pos: Float3,
        camera_up: Float3,
        camera_right: Float3,
        camera_fwd: Float3,
        camera_near: f32,
        camera_far: f32,
        camera_fov: f32,
        camera_aspect_ratio: f32,
        motion_vectors_invalid_value: f32,
        depth_inverted: u8,
        camera_motion_included: u8,
        motion_vectors_3d: u8,
        reset: u8,
        orthographic_projection: u8,
        motion_vectors_dilated: u8,
        motion_vectors_jittered: u8,
        min_relative_linear_depth_object_separation: f32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct DlssOptions {
        base: Base,
        mode: u32,
        output_width: u32,
        output_height: u32,
        sharpness: f32,
        pre_exposure: f32,
        exposure_scale: f32,
        color_buffers_hdr: u8,
        indicator_invert_axis_x: u8,
        indicator_invert_axis_y: u8,
        dlaa_preset: u32,
        quality_preset: u32,
        balanced_preset: u32,
        performance_preset: u32,
        ultra_performance_preset: u32,
        ultra_quality_preset: u32,
        use_auto_exposure: u8,
        alpha_upscaling_enabled: u8,
    }

    #[repr(C)]
    #[derive(Default)]
    struct DlssOptimalSettings {
        optimal_render_width: u32,
        optimal_render_height: u32,
        optimal_sharpness: f32,
        render_width_min: u32,
        render_height_min: u32,
        render_width_max: u32,
        render_height_max: u32,
    }

    #[repr(C)]
    struct DlssOptimalSettingsS {
        base: Base,
        s: DlssOptimalSettings,
    }

    // the ABI checks the Streamline headers make in `sl::test::AbiValidation`
    const _: () = {
        assert!(std::mem::size_of::<Base>() == 32);
        assert!(std::mem::size_of::<Preferences>() == 144);
        assert!(std::mem::size_of::<Resource>() == 112);
        assert!(std::mem::size_of::<ResourceTag>() == 64);
        assert!(std::mem::size_of::<ViewportHandle>() == 40);
        assert!(std::mem::size_of::<Constants>() == 456);
        assert!(std::mem::size_of::<DlssOptions>() == 88);
    };

    type Fn0 = unsafe extern "C" fn() -> i32;
    type FnInit = unsafe extern "C" fn(*const Preferences, u64) -> i32;
    type FnIsFeatureSupported = unsafe extern "C" fn(u32, *const AdapterInfo) -> i32;
    type FnIsFeatureLoaded = unsafe extern "C" fn(u32, *mut bool) -> i32;
    type FnSetD3DDevice = unsafe extern "C" fn(*mut c_void) -> i32;
    type FnGetFeatureFunction = unsafe extern "C" fn(u32, *const c_char, *mut *mut c_void) -> i32;
    type FnGetNewFrameToken = unsafe extern "C" fn(*mut *const c_void, *const u32) -> i32;
    type FnSetTagForFrame = unsafe extern "C" fn(*const c_void, *const ViewportHandle, *const ResourceTag, u32, *mut c_void) -> i32;
    type FnSetConstants = unsafe extern "C" fn(*const Constants, *const c_void, *const ViewportHandle) -> i32;
    type FnEvaluateFeature = unsafe extern "C" fn(u32, *const c_void, *const *const Base, u32, *mut c_void) -> i32;
    type FnFreeResources = unsafe extern "C" fn(u32, *const ViewportHandle) -> i32;
    type FnUpgradeInterface = unsafe extern "C" fn(*mut *mut c_void) -> i32;
    type FnDlssGetOptimalSettings = unsafe extern "C" fn(*const DlssOptions, *mut DlssOptimalSettingsS) -> i32;
    type FnDlssSetOptions = unsafe extern "C" fn(*const ViewportHandle, *const DlssOptions) -> i32;

    /// `sl::Result` as text.
    fn result_name(r: i32) -> &'static str {
        const NAMES: [&str; 41] = [
            "eOk", "eErrorIO", "eErrorDriverOutOfDate", "eErrorOSOutOfDate", "eErrorOSDisabledHWS", "eErrorDeviceNotCreated",
            "eErrorNoSupportedAdapterFound", "eErrorAdapterNotSupported", "eErrorNoPlugins", "eErrorVulkanAPI", "eErrorDXGIAPI",
            "eErrorD3DAPI", "eErrorNRDAPI", "eErrorNVAPI", "eErrorReflexAPI", "eErrorNGXFailed", "eErrorJSONParsing",
            "eErrorMissingProxy", "eErrorMissingResourceState", "eErrorInvalidIntegration", "eErrorMissingInputParameter",
            "eErrorNotInitialized", "eErrorComputeFailed", "eErrorInitNotCalled", "eErrorExceptionHandler",
            "eErrorInvalidParameter", "eErrorMissingConstants", "eErrorDuplicatedConstants", "eErrorMissingOrInvalidAPI",
            "eErrorCommonConstantsMissing", "eErrorUnsupportedInterface", "eErrorFeatureMissing", "eErrorFeatureNotSupported",
            "eErrorFeatureMissingHooks", "eErrorFeatureFailedToLoad", "eErrorFeatureWrongPriority",
            "eErrorFeatureMissingDependency", "eErrorFeatureManagerInvalidState", "eErrorInvalidState", "eWarnOutOfVRAM", "?",
        ];
        NAMES.get(r as usize).copied().unwrap_or("?")
    }

    fn check(what: &str, r: i32) -> Result<()> {
        if r == 0 {
            Ok(())
        } else {
            Err(anyhow!("Streamline {what}: {} ({r})", result_name(r)))
        }
    }

    unsafe extern "C" fn sl_log(kind: u32, msg: *const c_char) {
        if msg.is_null() {
            return;
        }
        let text = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
        let text = text.trim_end();
        match kind {
            2 => log::error!("Streamline: {text}"),
            1 => log::warn!("Streamline: {text}"),
            _ => log::debug!("Streamline: {text}"),
        }
    }

    fn wide(s: &std::path::Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        s.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    }

    fn mode_code(mode: DlssMode) -> u32 {
        // sl::DLSSMode: eOff, eMaxPerformance, eBalanced, eMaxQuality, eUltraPerformance,
        // eUltraQuality, eDLAA
        match mode {
            DlssMode::Off => 0,
            DlssMode::Performance => 1,
            DlssMode::Balanced => 2,
            DlssMode::Quality => 3,
            DlssMode::UltraPerformance => 4,
            DlssMode::Dlaa => 6,
        }
    }

    fn dxgi_format(f: wgpu::TextureFormat) -> u32 {
        use wgpu::TextureFormat as Tf;
        (match f {
            Tf::Bgra8UnormSrgb => DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
            Tf::Bgra8Unorm => DXGI_FORMAT_B8G8R8A8_UNORM,
            Tf::Rgba8UnormSrgb => DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
            Tf::Rgba8Unorm => DXGI_FORMAT_R8G8B8A8_UNORM,
            Tf::Rgb10a2Unorm => DXGI_FORMAT_R10G10B10A2_UNORM,
            Tf::Rgba16Float => DXGI_FORMAT_R16G16B16A16_FLOAT,
            Tf::Rg16Float => DXGI_FORMAT_R16G16_FLOAT,
            Tf::Rg32Float => DXGI_FORMAT_R32G32_FLOAT,
            Tf::R8Unorm => DXGI_FORMAT_R8_UNORM,
            // (the depth is sampled too: wgpu makes it R32_TYPELESS, its view R32_FLOAT)
            Tf::Depth32Float => DXGI_FORMAT_R32_FLOAT,
            _ => DXGI_FORMAT_UNKNOWN,
        })
        .0 as u32
    }

    /// A swap chain that presents nothing. With manual hooking Streamline's end-of-frame work
    /// (`presentCommon`: its frame counter, its garbage collection, the VRAM budget) runs in
    /// the hooks of its own swap-chain proxy, and wgpu presents on its own swap chain;
    /// Streamline's proxy around this one is presented once a frame instead.
    #[windows::core::implement(IDXGISwapChain)]
    struct PresentPacer {
        device: ID3D12Device,
    }

    fn not_implemented<T>() -> windows::core::Result<T> {
        Err(windows::Win32::Foundation::E_NOTIMPL.into())
    }

    impl IDXGIObject_Impl for PresentPacer_Impl {
        fn SetPrivateData(&self, _: *const windows::core::GUID, _: u32, _: *const c_void) -> windows::core::Result<()> {
            not_implemented()
        }
        fn SetPrivateDataInterface(&self, _: *const windows::core::GUID, _: windows::core::Ref<windows::core::IUnknown>) -> windows::core::Result<()> {
            not_implemented()
        }
        fn GetPrivateData(&self, _: *const windows::core::GUID, _: *mut u32, _: *mut c_void) -> windows::core::Result<()> {
            not_implemented()
        }
        fn GetParent(&self, _: *const windows::core::GUID, _: *mut *mut c_void) -> windows::core::Result<()> {
            not_implemented()
        }
    }

    impl IDXGIDeviceSubObject_Impl for PresentPacer_Impl {
        fn GetDevice(&self, riid: *const windows::core::GUID, device: *mut *mut c_void) -> windows::core::Result<()> {
            unsafe { self.device.query(riid, device) }.ok()
        }
    }

    impl IDXGISwapChain_Impl for PresentPacer_Impl {
        fn Present(&self, _: u32, _: DXGI_PRESENT) -> windows::core::HRESULT {
            windows::Win32::Foundation::S_OK
        }
        fn GetBuffer(&self, _: u32, _: *const windows::core::GUID, _: *mut *mut c_void) -> windows::core::Result<()> {
            not_implemented()
        }
        fn SetFullscreenState(&self, _: windows::core::BOOL, _: windows::core::Ref<IDXGIOutput>) -> windows::core::Result<()> {
            not_implemented()
        }
        fn GetFullscreenState(&self, _: *mut windows::core::BOOL, _: windows::core::OutRef<IDXGIOutput>) -> windows::core::Result<()> {
            not_implemented()
        }
        fn GetDesc(&self) -> windows::core::Result<DXGI_SWAP_CHAIN_DESC> {
            not_implemented()
        }
        fn ResizeBuffers(&self, _: u32, _: u32, _: u32, _: DXGI_FORMAT, _: &DXGI_SWAP_CHAIN_FLAG) -> windows::core::Result<()> {
            Ok(())
        }
        fn ResizeTarget(&self, _: *const DXGI_MODE_DESC) -> windows::core::Result<()> {
            not_implemented()
        }
        fn GetContainingOutput(&self) -> windows::core::Result<IDXGIOutput> {
            not_implemented()
        }
        fn GetFrameStatistics(&self, _: *mut DXGI_FRAME_STATISTICS) -> windows::core::Result<()> {
            not_implemented()
        }
        fn GetLastPresentCount(&self) -> windows::core::Result<u32> {
            not_implemented()
        }
    }

    /// A D3D12 command list of ours, and when the GPU is done with it.
    struct Slot {
        allocator: ID3D12CommandAllocator,
        list: ID3D12GraphicsCommandList,
        done_at: u64,
    }

    pub struct Runtime {
        module: HMODULE,
        shutdown: Fn0,
        get_new_frame_token: FnGetNewFrameToken,
        set_tag_for_frame: FnSetTagForFrame,
        set_constants: FnSetConstants,
        evaluate_feature: FnEvaluateFeature,
        free_resources: FnFreeResources,
        get_optimal_settings: FnDlssGetOptimalSettings,
        set_options: FnDlssSetOptions,
        /// (the wide strings Streamline was handed at `slInit`)
        _paths: (Vec<u16>, Vec<*const u16>),
        viewport: ViewportHandle,
        /// The options last handed to `slDLSSSetOptions`: (mode, output size).
        options_set: Option<(DlssMode, (u32, u32))>,
        /// Render sizes as Streamline said them: (mode, output) -> render size.
        optimal: Vec<((DlssMode, (u32, u32)), (u32, u32))>,
        frame: u32,
        slots: Vec<Slot>,
        next_slot: usize,
        fence: ID3D12Fence,
        fence_value: u64,
        /// Whether DLSS allocated its resources for the viewport (they are freed on drop).
        allocated: bool,
        /// Streamline's proxy around a `PresentPacer` (None when Streamline would not make one).
        pacer: Option<IDXGISwapChain>,
    }

    // (Streamline is used from the render thread only; the handles are plain pointers)
    unsafe impl Send for Runtime {}

    fn load_fn<T: Copy>(module: HMODULE, name: &CStr) -> Result<T> {
        debug_assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<usize>());
        let f = unsafe { GetProcAddress(module, PCSTR(name.as_ptr().cast())) }
            .ok_or_else(|| anyhow!("sl.interposer.dll has no {}", name.to_string_lossy()))?;
        Ok(unsafe { std::mem::transmute_copy::<_, T>(&f) })
    }

    /// The folder Streamline's DLLs are looked for in: `OMSI_STREAMLINE_DIR`, else the
    /// game's own folder.
    fn runtime_dir() -> std::path::PathBuf {
        if let Some(d) = omsi_cfg::env::var_os("OMSI_STREAMLINE_DIR") {
            return d.into();
        }
        std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| ".".into())
    }

    impl Runtime {
        /// Streamline with DLSS on wgpu's D3D12 device. An error says what is missing (the
        /// DLLs, an RTX card, a newer driver, DirectX 12).
        pub fn load(device: &wgpu::Device, _queue: &wgpu::Queue) -> Result<Runtime> {
            let hal = unsafe { device.as_hal::<wgpu_hal::api::Dx12>() }.ok_or_else(|| anyhow!("DLSS needs the DirectX 12 graphics API (Settings → Graphics API)"))?;
            let raw_device = hal.raw_device().clone();
            drop(hal);
            let dir = runtime_dir();
            let missing: Vec<&str> = ["sl.interposer.dll", "sl.common.dll", "sl.dlss.dll", "nvngx_dlss.dll"].into_iter().filter(|f| !dir.join(f).is_file()).collect();
            if !missing.is_empty() {
                bail!("NVIDIA's Streamline runtime is not beside the game in {} (missing {}): copy the DLLs from the Streamline SDK's bin/x64 there", dir.display(), missing.join(", "));
            }
            let interposer = wide(&dir.join("sl.interposer.dll"));
            let module = unsafe { LoadLibraryExW(PCWSTR(interposer.as_ptr()), None, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) }.context("load sl.interposer.dll")?;
            match Self::start(module, &dir, raw_device) {
                Ok(r) => Ok(r),
                Err(e) => {
                    unsafe {
                        let _ = FreeLibrary(module);
                    }
                    Err(e)
                }
            }
        }

        fn start(module: HMODULE, dir: &std::path::Path, device: ID3D12Device) -> Result<Runtime> {
            let init: FnInit = load_fn(module, c"slInit")?;
            let shutdown: Fn0 = load_fn(module, c"slShutdown")?;
            let is_supported: FnIsFeatureSupported = load_fn(module, c"slIsFeatureSupported")?;
            let is_loaded: FnIsFeatureLoaded = load_fn(module, c"slIsFeatureLoaded")?;
            let set_device: FnSetD3DDevice = load_fn(module, c"slSetD3DDevice")?;
            let get_feature_function: FnGetFeatureFunction = load_fn(module, c"slGetFeatureFunction")?;
            let get_new_frame_token: FnGetNewFrameToken = load_fn(module, c"slGetNewFrameToken")?;
            let set_tag_for_frame: FnSetTagForFrame = load_fn(module, c"slSetTagForFrame")?;
            let set_constants: FnSetConstants = load_fn(module, c"slSetConstants")?;
            let evaluate_feature: FnEvaluateFeature = load_fn(module, c"slEvaluateFeature")?;
            let free_resources: FnFreeResources = load_fn(module, c"slFreeResources")?;
            let upgrade_interface: FnUpgradeInterface = load_fn(module, c"slUpgradeInterface")?;

            let plugin_dir = wide(dir);
            let plugin_paths = vec![plugin_dir.as_ptr()];
            static FEATURES: [u32; 1] = [FEATURE_DLSS];
            let engine_version = concat!("openOMSI ", env!("CARGO_PKG_VERSION"), "\0");
            let project_id = omsi_cfg::env::var("OMSI_DLSS_PROJECT_ID").ok().and_then(|s| std::ffi::CString::new(s).ok());
            let verbose = omsi_cfg::env::var_os("OMSI_DLSS_VERBOSE").is_some();
            let prefs = Preferences {
                base: Base::new(PREFERENCES, 1),
                show_console: false,
                // eDefault, or eVerbose for OMSI_DLSS_VERBOSE
                log_level: if verbose { 2 } else { 1 },
                paths_to_plugins: plugin_paths.as_ptr(),
                num_paths_to_plugins: 1,
                path_to_logs_and_data: std::ptr::null(),
                allocate_callback: std::ptr::null(),
                release_callback: std::ptr::null(),
                log_message_callback: Some(sl_log),
                // (wgpu's command lists are not Streamline's business: no state tracking, no
                // hooks on the device, tags per frame)
                flags: FLAG_DISABLE_CL_STATE_TRACKING | FLAG_DISABLE_DEBUG_TEXT | FLAG_USE_MANUAL_HOOKING | FLAG_FRAME_BASED_TAGGING,
                features_to_load: FEATURES.as_ptr(),
                num_features_to_load: 1,
                application_id: 0,
                // EngineType::eCustom, RenderAPI::eD3D12
                engine: 0,
                engine_version: engine_version.as_ptr().cast(),
                project_id: project_id.as_deref().unwrap_or(PROJECT_ID).as_ptr(),
                render_api: 1,
            };
            check("slInit", unsafe { init(&prefs, SDK_VERSION) })?;
            // from here on Streamline is up: shut it down again on every way out
            let up = |r: Result<Runtime>| -> Result<Runtime> {
                if r.is_err() {
                    unsafe { shutdown() };
                }
                r
            };
            up((|| {
                check("slSetD3DDevice", unsafe { set_device(device.as_raw()) })?;
                let mut luid = unsafe { device.GetAdapterLuid() };
                let adapter = AdapterInfo {
                    base: Base::new(ADAPTER_INFO, 1),
                    device_luid: (&mut luid as *mut windows::Win32::Foundation::LUID).cast(),
                    device_luid_size_in_bytes: std::mem::size_of::<windows::Win32::Foundation::LUID>() as u32,
                    vk_physical_device: std::ptr::null_mut(),
                };
                check("DLSS support (a GeForce RTX card and a current driver are needed)", unsafe { is_supported(FEATURE_DLSS, &adapter) })?;
                let mut loaded = false;
                check("slIsFeatureLoaded", unsafe { is_loaded(FEATURE_DLSS, &mut loaded) })?;
                if !loaded {
                    bail!("Streamline did not load its DLSS plugin (sl.dlss.dll)");
                }
                let feature_fn = |name: &CStr| -> Result<*mut c_void> {
                    let mut f: *mut c_void = std::ptr::null_mut();
                    check(&format!("slGetFeatureFunction({})", name.to_string_lossy()), unsafe { get_feature_function(FEATURE_DLSS, name.as_ptr(), &mut f) })?;
                    if f.is_null() {
                        bail!("Streamline has no {}", name.to_string_lossy());
                    }
                    Ok(f)
                };
                let get_optimal_settings: FnDlssGetOptimalSettings = unsafe { std::mem::transmute(feature_fn(c"slDLSSGetOptimalSettings")?) };
                let set_options: FnDlssSetOptions = unsafe { std::mem::transmute(feature_fn(c"slDLSSSetOptions")?) };
                let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }.context("D3D12 fence for DLSS")?;
                let slots = (0..3)
                    .map(|_| -> Result<Slot> {
                        let allocator: ID3D12CommandAllocator = unsafe { device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }?;
                        let list: ID3D12GraphicsCommandList = unsafe { device.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None::<&ID3D12PipelineState>) }?;
                        unsafe { list.Close() }?;
                        Ok(Slot { allocator, list, done_at: 0 })
                    })
                    .collect::<Result<Vec<_>>>()
                    .context("D3D12 command lists for DLSS")?;
                let pacer: IDXGISwapChain = PresentPacer { device: device.clone() }.into();
                let mut raw = pacer.as_raw();
                let r = unsafe { upgrade_interface(&mut raw) };
                let pacer = if r == 0 && !raw.is_null() && raw != pacer.as_raw() {
                    // (the proxy, with one reference that is ours)
                    Some(unsafe { IDXGISwapChain::from_raw(raw) })
                } else {
                    log::warn!("DLSS: Streamline made no swap-chain proxy ({}): its end-of-frame bookkeeping will not run", result_name(r));
                    None
                };
                log::info!("DLSS: Streamline {}.{}.{} up with its DLSS plugin, from {}", SDK_VERSION >> 48, (SDK_VERSION >> 32) & 0xffff, (SDK_VERSION >> 16) & 0xffff, dir.display());
                Ok(Runtime {
                    module,
                    shutdown,
                    get_new_frame_token,
                    set_tag_for_frame,
                    set_constants,
                    evaluate_feature,
                    free_resources,
                    get_optimal_settings,
                    set_options,
                    _paths: (plugin_dir.clone(), plugin_paths.clone()),
                    viewport: ViewportHandle { base: Base::new(VIEWPORT_HANDLE, 1), value: 0 },
                    options_set: None,
                    optimal: Vec::new(),
                    frame: 0,
                    slots,
                    next_slot: 0,
                    fence,
                    fence_value: 0,
                    allocated: false,
                    pacer,
                })
            })())
        }

        fn options(mode: DlssMode, (w, h): (u32, u32)) -> DlssOptions {
            DlssOptions {
                base: Base::new(DLSS_OPTIONS, 3),
                mode: mode_code(mode),
                output_width: w,
                output_height: h,
                sharpness: 0.0,
                pre_exposure: 1.0,
                exposure_scale: 1.0,
                // the picture DLSS gets is the tone-mapped one (the HUD comes after it)
                color_buffers_hdr: FALSE,
                indicator_invert_axis_x: FALSE,
                indicator_invert_axis_y: FALSE,
                dlaa_preset: 0,
                quality_preset: 0,
                balanced_preset: 0,
                performance_preset: 0,
                ultra_performance_preset: 0,
                ultra_quality_preset: 0,
                use_auto_exposure: TRUE,
                alpha_upscaling_enabled: FALSE,
            }
        }

        /// The size the 3D picture is drawn at for this mode and window size: what
        /// Streamline says is optimal, else NVIDIA's ratios.
        pub fn render_size(&mut self, mode: DlssMode, out: (u32, u32)) -> (u32, u32) {
            if mode == DlssMode::Dlaa {
                return out;
            }
            if let Some((_, s)) = self.optimal.iter().find(|(k, _)| *k == (mode, out)) {
                return *s;
            }
            let options = Self::options(mode, out);
            let mut settings = DlssOptimalSettingsS { base: Base::new(DLSS_OPTIMAL_SETTINGS, 1), s: DlssOptimalSettings::default() };
            let r = unsafe { (self.get_optimal_settings)(&options, &mut settings) };
            let size = if r == 0 && settings.s.optimal_render_width > 0 && settings.s.optimal_render_height > 0 {
                (settings.s.optimal_render_width.min(out.0), settings.s.optimal_render_height.min(out.1))
            } else {
                log::warn!("DLSS: no optimal render size for {mode:?} at {}x{} ({}); using NVIDIA's ratio", out.0, out.1, result_name(r));
                fallback_render_size(mode, out)
            };
            log::info!("DLSS {mode:?}: {}x{} drawn for a {}x{} window", size.0, size.1, out.0, out.1);
            self.optimal.push(((mode, out), size));
            size
        }

        /// Record DLSS for this frame into a command list of ours and execute it on wgpu's
        /// queue: call between the submit that drew `inputs` and the one that reads the
        /// output.
        pub fn evaluate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mode: DlssMode, inputs: &Inputs, c: &FrameConstants) -> Result<()> {
            let queue_hal = unsafe { queue.as_hal::<wgpu_hal::api::Dx12>() }.ok_or_else(|| anyhow!("wgpu did not expose its DX12 queue"))?;
            let raw_queue = queue_hal.as_raw().clone();
            drop(queue_hal);
            let _ = device;
            if self.options_set != Some((mode, inputs.output_size)) {
                // (a new size: what Streamline keeps for the old one goes, as when a swap chain
                // is resized)
                if let (Some(p), Some(_)) = (&self.pacer, self.options_set) {
                    let _ = unsafe { p.ResizeBuffers(0, inputs.output_size.0, inputs.output_size.1, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0)) };
                }
                let options = Self::options(mode, inputs.output_size);
                check("slDLSSSetOptions", unsafe { (self.set_options)(&self.viewport, &options) })?;
                self.options_set = Some((mode, inputs.output_size));
            }
            // the command list whose last use the GPU has finished
            let slot = &mut self.slots[self.next_slot];
            self.next_slot = (self.next_slot + 1) % 3;
            if unsafe { self.fence.GetCompletedValue() } < slot.done_at {
                // (no event: the call waits until the fence gets there)
                unsafe { self.fence.SetEventOnCompletion(slot.done_at, HANDLE::default()) }.context("wait for DLSS command list")?;
            }
            unsafe {
                slot.allocator.Reset()?;
                slot.list.Reset(&slot.allocator, None::<&ID3D12PipelineState>)?;
            }
            let raw = |t: &wgpu::Texture| -> Result<ID3D12Resource> {
                let h = unsafe { t.as_hal::<wgpu_hal::api::Dx12>() }.ok_or_else(|| anyhow!("a DLSS texture is not a DX12 one"))?;
                Ok(unsafe { h.raw_resource() }.clone())
            };
            let (color, depth, motion, bias, output) = (raw(inputs.color)?, raw(inputs.depth)?, raw(inputs.motion)?, raw(inputs.bias)?, raw(inputs.output)?);
            let (rw, rh) = inputs.render_size;
            let (ow, oh) = inputs.output_size;
            // (the states of their last use in the scene's submit, see `render_inner`)
            let resource = |r: &ID3D12Resource, state: D3D12_RESOURCE_STATES, format: wgpu::TextureFormat, (w, h): (u32, u32)| Resource {
                base: Base::new(RESOURCE, 1),
                // ResourceType::eTex2d
                kind: 0,
                native: r.as_raw(),
                memory: std::ptr::null_mut(),
                view: std::ptr::null_mut(),
                state: state.0 as u32,
                width: w,
                height: h,
                native_format: dxgi_format(format),
                mip_levels: 1,
                array_layers: 1,
                gpu_virtual_address: 0,
                flags: 0,
                usage: 0,
                internal_flags: 0,
                reserved: 0,
            };
            let mut res = [
                resource(&color, D3D12_RESOURCE_STATE_RENDER_TARGET, inputs.color.format(), (rw, rh)),
                resource(&depth, D3D12_RESOURCE_STATE_DEPTH_WRITE, inputs.depth.format(), (rw, rh)),
                resource(&motion, D3D12_RESOURCE_STATE_RENDER_TARGET, inputs.motion.format(), (rw, rh)),
                resource(&output, D3D12_RESOURCE_STATE_RENDER_TARGET, inputs.output.format(), (ow, oh)),
                resource(&bias, D3D12_RESOURCE_STATE_RENDER_TARGET, inputs.bias.format(), (rw, rh)),
            ];
            let render_extent = Extent { top: 0, left: 0, width: rw, height: rh };
            let output_extent = Extent { top: 0, left: 0, width: ow, height: oh };
            let [c0, c1, c2, c3, c4] = &mut res;
            let tag = |r: &mut Resource, kind: u32, extent: Extent| ResourceTag { base: Base::new(RESOURCE_TAG, 1), resource: r, kind, lifecycle: VALID_UNTIL_EVALUATE, extent };
            let tags = [
                tag(c0, BUFFER_SCALING_INPUT_COLOR, render_extent),
                tag(c1, BUFFER_DEPTH, render_extent),
                tag(c2, BUFFER_MOTION_VECTORS, render_extent),
                tag(c3, BUFFER_SCALING_OUTPUT_COLOR, output_extent),
                tag(c4, BUFFER_BIAS_CURRENT_COLOR_HINT, render_extent),
            ];
            let mut token: *const c_void = std::ptr::null();
            self.frame = self.frame.wrapping_add(1);
            check("slGetNewFrameToken", unsafe { (self.get_new_frame_token)(&mut token, &self.frame) })?;
            if token.is_null() {
                bail!("Streamline gave no frame token");
            }
            let f2 = |v: [f32; 2]| Float2 { x: v[0], y: v[1] };
            let f3 = |v: [f32; 3]| Float3 { x: v[0], y: v[1], z: v[2] };
            let identity = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
            let constants = Constants {
                base: Base::new(CONSTANTS, 2),
                camera_view_to_clip: c.view_to_clip,
                clip_to_camera_view: c.clip_to_view,
                clip_to_lens_clip: identity,
                clip_to_prev_clip: c.clip_to_prev_clip,
                prev_clip_to_clip: c.prev_clip_to_clip,
                jitter_offset: f2(c.jitter),
                // (the motion vectors are in pixels of the render size)
                mvec_scale: f2([1.0 / rw as f32, 1.0 / rh as f32]),
                camera_pinhole_offset: f2([0.0, 0.0]),
                camera_pos: f3(c.camera_pos),
                camera_up: f3(c.camera_up),
                camera_right: f3(c.camera_right),
                camera_fwd: f3(c.camera_fwd),
                camera_near: c.near,
                camera_far: c.far,
                camera_fov: c.fov,
                camera_aspect_ratio: c.aspect,
                motion_vectors_invalid_value: 0.0,
                depth_inverted: TRUE,
                camera_motion_included: TRUE,
                motion_vectors_3d: FALSE,
                reset: if c.reset { TRUE } else { FALSE },
                orthographic_projection: FALSE,
                motion_vectors_dilated: FALSE,
                motion_vectors_jittered: FALSE,
                min_relative_linear_depth_object_separation: 40.0,
            };
            let list_ptr = slot.list.as_raw();
            let recorded = (|| -> Result<()> {
                check("slSetConstants", unsafe { (self.set_constants)(&constants, token, &self.viewport) })?;
                check("slSetTagForFrame", unsafe { (self.set_tag_for_frame)(token, &self.viewport, tags.as_ptr(), tags.len() as u32, list_ptr) })?;
                let inputs: [*const Base; 1] = [&self.viewport.base];
                check("slEvaluateFeature(DLSS)", unsafe { (self.evaluate_feature)(FEATURE_DLSS, token, inputs.as_ptr(), 1, list_ptr) })?;
                Ok(())
            })();
            // (the list is closed and executed either way: an empty one is harmless, and
            // the slot's fence value stays true)
            unsafe { slot.list.Close() }?;
            let list: ID3D12CommandList = slot.list.cast()?;
            unsafe { raw_queue.ExecuteCommandLists(&[Some(list)]) };
            self.fence_value += 1;
            unsafe { raw_queue.Signal(&self.fence, self.fence_value) }?;
            slot.done_at = self.fence_value;
            recorded?;
            self.allocated = true;
            // the frame's end for Streamline
            if let Some(p) = &self.pacer {
                let _ = unsafe { p.Present(0, DXGI_PRESENT(0)) };
            }
            Ok(())
        }
    }

    impl Drop for Runtime {
        fn drop(&mut self) {
            unsafe {
                // DLSS's resources may still be in use on the GPU
                if self.fence.GetCompletedValue() < self.fence_value {
                    let _ = self.fence.SetEventOnCompletion(self.fence_value, HANDLE::default());
                }
                if self.allocated {
                    let _ = (self.free_resources)(FEATURE_DLSS, &self.viewport);
                }
                self.slots.clear();
                self.pacer = None;
                let r = (self.shutdown)();
                if r != 0 {
                    log::warn!("DLSS: slShutdown: {}", result_name(r));
                }
                let _ = FreeLibrary(self.module);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_read_back_and_scale() {
        for m in [DlssMode::Off, DlssMode::Dlaa, DlssMode::Quality, DlssMode::Balanced, DlssMode::Performance, DlssMode::UltraPerformance] {
            assert_eq!(DlssMode::parse(m.as_str()), m);
        }
        assert_eq!(DlssMode::parse("Ultra Performance"), DlssMode::UltraPerformance);
        assert_eq!(DlssMode::parse(""), DlssMode::Off);
        assert_eq!(fallback_render_size(DlssMode::Dlaa, (2560, 1440)), (2560, 1440));
        assert_eq!(fallback_render_size(DlssMode::Performance, (3840, 2160)), (1920, 1080));
    }
}
