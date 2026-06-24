struct ConfigBuffer {
    data: array<GpuIvConfig>,
}

struct OutputBuffer {
    data: array<u32>,
}

struct CounterBuffer {
    value: atomic<u32>,
}

struct DispatchParams {
    base_index: u64,
    total_len: u64,
}

@group(0) @binding(0)
var<storage, read> config_buf: ConfigBuffer;

@group(0) @binding(1)
var<storage, read_write> output_buf: OutputBuffer;

@group(0) @binding(2)
var<storage, read_write> counter_buf: CounterBuffer;

@group(0) @binding(3)
var<storage, read> params: DispatchParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global = params.base_index + u64(gid.x);
    if (global >= params.total_len) {
        return;
    }

    let cfg = config_buf.data[0];
    let seed_high = u32(global);
    if (mt_matches_seed_high(seed_high, cfg.iv_step, cfg.iv_min, cfg.iv_max)) {
        let idx = atomicAdd(&counter_buf.value, 1u);
        if (idx < MAX_RESULTS) {
            output_buf.data[idx] = seed_high;
        }
    }
}
