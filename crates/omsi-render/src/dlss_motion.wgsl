// DLSS's motion passes (see `DlssState` in lib.rs), appended to the scene module only where
// it reads its arrays from storage buffers (`scene_shader_source`): DLSS runs on DirectX 12,
// never on the chips that read them from textures, and its last-frame matrices are one more
// storage buffer in the vertex stage.

struct Motion {
    // this frame's view-projection without the sub-pixel jitter
    vp: mat4x4<f32>,
    // last frame's, for last frame's model matrices (relative to last frame's render origin)
    vp_prev: mat4x4<f32>,
    // last frame's, relative to this frame's render origin: the sky, and the entries without
    // a matrix of last frame
    vp_prev_here: mat4x4<f32>,
    // the inverse of `vp` (the sky's directions)
    vp_inv: mat4x4<f32>,
    // xy: the render size in pixels, z: 1 when `prev_models` holds last frame's matrices,
    // w: how many entries it holds
    params: vec4<f32>,
};
@group(3) @binding(0) var<uniform> motion: Motion;
// last frame's model matrices, as `models`
@group(3) @binding(1) var<storage, read> prev_models: array<vec4<f32>>;

struct MotionOut {
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) params: vec4<f32>,
    @location(4) params2: vec4<f32>,
    // (without the jitter) this frame's and last frame's clip position
    @location(5) cur: vec4<f32>,
    @location(6) prev: vec4<f32>,
};
// (the fragment side of MotionOut, without the invariant: see FsIn)
struct MotionFsIn {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) params: vec4<f32>,
    @location(4) params2: vec4<f32>,
    @location(5) cur: vec4<f32>,
    @location(6) prev: vec4<f32>,
};

@vertex
fn vs_motion(in: VsIn) -> MotionOut {
    let v = vertex_main(in.pos, in.normal, in.uv, in.inst);
    let e = draw_list[in.inst];
    let local = vec4<f32>(in.pos, 1.0);
    let wp = model_matrix(e) * local;
    var out: MotionOut;
    out.clip = v.clip;
    out.world = v.world;
    out.normal = v.normal;
    out.uv = v.uv;
    out.params = v.params;
    out.params2 = v.params2;
    out.cur = motion.vp * wp;
    if (motion.params.z > 0.5 && f32(e) < motion.params.w) {
        let k = e * 4u;
        let pm = mat4x4<f32>(prev_models[k], prev_models[k + 1u], prev_models[k + 2u], prev_models[k + 3u]);
        out.prev = motion.vp_prev * (pm * local);
    } else {
        out.prev = motion.vp_prev_here * wp;
    }
    return out;
}

// From this pixel to where the same point was last frame, in pixels (x right, y down).
fn motion_pixels(cur: vec4<f32>, prev: vec4<f32>) -> vec4<f32> {
    if (prev.w <= 1e-4 || cur.w <= 1e-4) {
        return vec4<f32>(0.0);
    }
    let d = (prev.xy / prev.w - cur.xy / cur.w) * vec2<f32>(0.5, -0.5) * motion.params.xy;
    return vec4<f32>(d, 0.0, 0.0);
}

@fragment
fn fs_motion(in: MotionFsIn) -> @location(0) vec4<f32> {
    return motion_pixels(in.cur, in.prev);
}

@fragment
fn fs_motion_test(in: MotionFsIn) -> @location(0) vec4<f32> {
    if (!cutout_covers(in.uv, in.params)) {
        discard;
    }
    return motion_pixels(in.cur, in.prev);
}

@fragment
fn fs_motion_transmap(in: MotionFsIn) -> @location(0) vec4<f32> {
    if (!transmap_covers(in.uv, in.params)) {
        discard;
    }
    return motion_pixels(in.cur, in.prev);
}

// A pane's film of water (`rain_glass`, as the colour pass draws it): where a drop covers
// the pixel, the pane's own motion - the drops go with the bus, not with the street seen
// through them, which DLSS dragged them along with as streaks. Between the drops the pixel
// keeps the motion of what lies behind the glass.
@fragment
fn fs_motion_rain(in: MotionFsIn) -> @location(0) vec4<f32> {
    let in_cab = inside_vehicle(camera.cam_pos.xyz) * near_player_vehicle(in.world) > 0.5;
    let g = rain_glass(in.world, in.uv - in.params.zw, in.normal, in.params.x, camera.post.y, in_cab);
    // (fading out with the distance, as the colour pass does)
    let near = 1.0 - smoothstep(5.0, 15.0, distance(in.world, camera.cam_pos.xyz));
    if (g.cover * near < 0.35) {
        discard;
    }
    return motion_pixels(in.cur, in.prev);
}

// How far DLSS takes the current picture over its history (its `BiasCurrentColor` mask):
// through a drop the street moves while the drop stays on the pane, one motion cannot be
// right for both, and the history showed the drops twice once DLSS scaled up. Where a drop
// covers the pixel, the current picture.
@fragment
fn fs_bias_rain(in: MotionFsIn) -> @location(1) f32 {
    let in_cab = inside_vehicle(camera.cam_pos.xyz) * near_player_vehicle(in.world) > 0.5;
    let g = rain_glass(in.world, in.uv - in.params.zw, in.normal, in.params.x, camera.post.y, in_cab);
    let near = 1.0 - smoothstep(5.0, 15.0, distance(in.world, camera.cam_pos.xyz));
    let bias = smoothstep(0.05, 0.5, g.cover * near);
    if (bias <= 0.0) {
        discard;
    }
    return bias;
}

// The sky (whatever the prepass leaves uncovered): a point on the far plane, drawn first.
struct SkyMotionOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_motion_sky(@builtin(vertex_index) i: u32) -> SkyMotionOut {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i >> 1u) * 4 - 1);
    var out: SkyMotionOut;
    // (reversed Z: depth 0 is the far plane)
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.ndc = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_motion_sky(in: SkyMotionOut) -> @location(0) vec4<f32> {
    let far = motion.vp_inv * vec4<f32>(in.ndc, 0.0, 1.0);
    let wp = vec4<f32>(far.xyz / far.w, 1.0);
    return motion_pixels(vec4<f32>(in.ndc, 0.0, 1.0), motion.vp_prev_here * wp);
}
