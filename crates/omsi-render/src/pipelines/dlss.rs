//! DLSS: the depth prepass that writes each pixel's motion beside its depth, and the sky's
//! motion laid first, so that Streamline has a history to work from (`DlssState` in the
//! renderer). Made only when DLSS is asked for.
//!
//! The motion is bind group 3 of the scene shader, beside the vehicle reflection's group 2.

use super::scene::SceneBase;
use crate::*;

pub(crate) fn build(device: &wgpu::Device, scene: &SceneBase, on: bool) -> Option<DlssPipelines> {
    if !on {
        return None;
    }
    let motion_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("dlss motion"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("dlss prepass"),
        // (group 2 is the vehicle reflection's in shader.wgsl: DLSS's motion is group 3)
        bind_group_layouts: &[
            Some(&scene.camera_layout),
            Some(&scene.material_layout),
            None,
            Some(&motion_layout),
        ],
        immediate_size: 0,
    });
    let motion_target = [Some(wgpu::ColorTargetState {
        format: DLSS_MOTION_FORMAT,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let make = |kind: u8, cull: bool| {
        let fragment = match kind {
            0 => "fs_motion",
            1 => "fs_motion_test",
            2 => "fs_motion_transmap",
            _ => unreachable!(),
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dlss depth prepass"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &scene.shader,
                entry_point: Some("vs_motion"),
                buffers: &[scene.vertex_layout.clone()],
                compilation_options: Default::default(),
            },
            primitive: one_sided_primitive(cull),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &scene.shader,
                entry_point: Some(fragment),
                targets: &motion_target,
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        })
    };
    let sky_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("dlss sky motion"),
        bind_group_layouts: &[None, None, None, Some(&motion_layout)],
        immediate_size: 0,
    });
    let sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("dlss sky motion"),
        layout: Some(&sky_pl),
        vertex: wgpu::VertexState {
            module: &scene.shader,
            entry_point: Some("vs_motion_sky"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &scene.shader,
            entry_point: Some("fs_motion_sky"),
            targets: &motion_target,
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    Some(DlssPipelines {
        prepass: [
            make(0, false),
            make(0, true),
            make(1, false),
            make(1, true),
            make(2, false),
            make(2, true),
        ],
        sky,
        motion_layout,
    })
}
