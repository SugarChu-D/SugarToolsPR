// Test WGSL for GpuInput -> GpuCandidate
// Notes:
// - Requires SHADER_INT64.

struct GpuInput {
    nazo: array<u32, 5>,
    vcount_timer0_as_data5: u32,
    mac: u64,
    gxframe_xor_frame: u32,
    date_as_data8: u32,
    timespec: array<vec2<u32>, 3>,
    key_presses: u32,
    _pad0: u32,
    p: u32,
    iv_min: array<u32, 6>,
    iv_max: array<u32, 6>,
    _pad1: u32,
}

struct GpuCandidate {
    seed0: u64,
    game_date: u32,
    game_time: u32,
    timer0: u32,
    key_presses: u32,
}

struct InputBuffer {
    data: array<GpuInput>,
}

struct OutputBuffer {
    data: array<GpuCandidate>,
}

@group(0) @binding(0)
var<storage, read> input_buf: InputBuffer;

@group(0) @binding(1)
var<storage, read_write> output_buf: OutputBuffer;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&input_buf.data)) {
        return;
    }
    let input = input_buf.data[i];

    var out: GpuCandidate;
    out.seed0 = input.mac;
    out.game_date = input.date_as_data8;
    out.game_time = input.timespec[0].x;
    out.timer0 = input.vcount_timer0_as_data5;
    out.key_presses = input.key_presses;

    output_buf.data[i] = out;
}
