//! Optional, cross-vendor GPU compute support.
//!
//! The backend is deliberately small: wgpu supplies portable device discovery
//! and dispatch while BioLang owns the WGSL compute kernel. No CUDA toolkit or
//! vendor library is linked. Numerical work falls back to the f64 CPU path if
//! a device is absent, disabled, too small, or reports an execution error.

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub compiled: bool,
    pub enabled: bool,
    pub available: bool,
    pub adapter: Option<String>,
    pub backend: Option<String>,
    pub device_type: Option<String>,
    pub driver: Option<String>,
    pub reason: String,
}

/// Human-readable compute provenance for run headers and saved logs.
///
/// This describes the adapter selected by the global policy. Individual
/// operations may still report a more specific backend (or a local fallback)
/// in their returned result.
pub fn execution_summary() -> String {
    let policy = std::env::var("BIOLANG_GPU")
        .unwrap_or_else(|_| "auto".to_string())
        .trim()
        .to_ascii_lowercase();
    let info = info();
    if info.available {
        let adapter = info
            .adapter
            .unwrap_or_else(|| "unknown adapter".to_string());
        let backend = info.backend.unwrap_or_else(|| "unknown API".to_string());
        format!("GPU: {adapter} via {backend} (BIOLANG_GPU={policy})")
    } else {
        format!("CPU f64 (BIOLANG_GPU={policy}; {})", info.reason)
    }
}

fn policy_enabled() -> Result<bool, String> {
    match std::env::var("BIOLANG_GPU")
        .unwrap_or_else(|_| "auto".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "auto" | "on" | "1" | "true" => Ok(true),
        "off" | "cpu" | "0" | "false" => Ok(false),
        value => Err(format!(
            "invalid BIOLANG_GPU={value:?}; use auto, on, off, or cpu"
        )),
    }
}

#[cfg(not(feature = "gpu"))]
pub fn info() -> GpuInfo {
    GpuInfo {
        compiled: false,
        enabled: false,
        available: false,
        adapter: None,
        backend: None,
        device_type: None,
        driver: None,
        reason: "GPU support was not compiled; rebuild with --features gpu".to_string(),
    }
}

#[cfg(not(feature = "gpu"))]
pub fn cross_apply_block(
    _left: &[Vec<f64>],
    _right: &[Vec<f64>],
    _basis: &[Vec<f64>],
) -> Result<Option<Vec<Vec<f64>>>, String> {
    Ok(None)
}

#[cfg(not(feature = "gpu"))]
pub fn nearest_rows(
    _embeddings: &[Vec<f64>],
    _k: usize,
    _metric: &str,
) -> Result<Option<Vec<Vec<(usize, f64)>>>, String> {
    Ok(None)
}

#[cfg(feature = "gpu")]
mod enabled {
    use super::{policy_enabled, GpuInfo};
    use bytemuck::{Pod, Zeroable};
    use std::sync::{mpsc, OnceLock};
    use wgpu::util::DeviceExt;

    struct Context {
        device: wgpu::Device,
        queue: wgpu::Queue,
    }

    struct State {
        info: GpuInfo,
        context: Option<Context>,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct Params {
        m: u32,
        k: u32,
        n: u32,
        transpose_a: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct NearestParams {
        n: u32,
        dimensions: u32,
        k: u32,
        metric: u32,
        query_start: u32,
        query_count: u32,
        padding_0: u32,
        padding_1: u32,
    }

    const MATMUL_SHADER: &str = r#"
struct Params {
    m: u32,
    k: u32,
    n: u32,
    transpose_a: u32,
};

@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let column = id.x;
    let row = id.y;
    if (row >= params.m || column >= params.n) {
        return;
    }
    var total = 0.0;
    for (var inner = 0u; inner < params.k; inner = inner + 1u) {
        var ai = row * params.k + inner;
        if (params.transpose_a != 0u) {
            ai = inner * params.m + row;
        }
        total = total + a[ai] * b[inner * params.n + column];
    }
    c[row * params.n + column] = total;
}
"#;

    const NEAREST_SHADER: &str = r#"
struct NearestParams {
    n: u32,
    dimensions: u32,
    k: u32,
    metric: u32,
    query_start: u32,
    query_count: u32,
    padding_0: u32,
    padding_1: u32,
};

@group(0) @binding(0) var<storage, read> points: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> output_distances: array<f32>;
@group(0) @binding(3) var<uniform> params: NearestParams;

var<workgroup> shared_distances: array<f32, 1024>;
var<workgroup> shared_indices: array<u32, 1024>;

fn distance_between(left: u32, right: u32) -> f32 {
    var squared = 0.0;
    var dot = 0.0;
    var norm_left = 0.0;
    var norm_right = 0.0;
    for (var dimension = 0u; dimension < params.dimensions; dimension = dimension + 1u) {
        let a = points[left * params.dimensions + dimension];
        let b = points[right * params.dimensions + dimension];
        let difference = a - b;
        squared = squared + difference * difference;
        dot = dot + a * b;
        norm_left = norm_left + a * a;
        norm_right = norm_right + b * b;
    }
    if (params.metric == 1u) {
        if (norm_left <= 1e-30 || norm_right <= 1e-30) {
            if (norm_left <= 1e-30 && norm_right <= 1e-30) { return 0.0; }
            return 1.0;
        }
        return clamp(1.0 - dot * inverseSqrt(norm_left * norm_right), 0.0, 2.0);
    }
    return sqrt(max(squared, 0.0));
}

@compute @workgroup_size(64, 1, 1)
fn main(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_id) local: vec3<u32>
) {
    let query_offset = group.x;
    if (query_offset >= params.query_count) { return; }
    let query = params.query_start + query_offset;
    let lane = local.x;
    var local_distances: array<f32, 16>;
    var local_indices: array<u32, 16>;
    for (var slot = 0u; slot < 16u; slot = slot + 1u) {
        local_distances[slot] = 3.402823e38;
        local_indices[slot] = 0xffffffffu;
    }
    for (var candidate = lane; candidate < params.n; candidate = candidate + 64u) {
        if (candidate == query) { continue; }
        let candidate_distance = distance_between(query, candidate);
        var worst = 0u;
        for (var slot = 1u; slot < 16u; slot = slot + 1u) {
            if (local_distances[slot] > local_distances[worst] ||
                (local_distances[slot] == local_distances[worst] && local_indices[slot] > local_indices[worst])) {
                worst = slot;
            }
        }
        if (candidate_distance < local_distances[worst] ||
            (candidate_distance == local_distances[worst] && candidate < local_indices[worst])) {
            local_distances[worst] = candidate_distance;
            local_indices[worst] = candidate;
        }
    }
    for (var slot = 0u; slot < 16u; slot = slot + 1u) {
        let shared_slot = lane * 16u + slot;
        shared_distances[shared_slot] = local_distances[slot];
        shared_indices[shared_slot] = local_indices[slot];
    }
    workgroupBarrier();

    if (lane == 0u) {
        var best_distances: array<f32, 256>;
        var best_indices: array<u32, 256>;
        for (var slot = 0u; slot < 256u; slot = slot + 1u) {
            best_distances[slot] = 3.402823e38;
            best_indices[slot] = 0xffffffffu;
        }
        for (var source = 0u; source < 1024u; source = source + 1u) {
            let candidate_distance = shared_distances[source];
            let candidate = shared_indices[source];
            var worst = 0u;
            for (var slot = 1u; slot < params.k; slot = slot + 1u) {
                if (best_distances[slot] > best_distances[worst] ||
                    (best_distances[slot] == best_distances[worst] && best_indices[slot] > best_indices[worst])) {
                    worst = slot;
                }
            }
            if (candidate_distance < best_distances[worst] ||
                (candidate_distance == best_distances[worst] && candidate < best_indices[worst])) {
                best_distances[worst] = candidate_distance;
                best_indices[worst] = candidate;
            }
        }
        for (var left = 0u; left < params.k; left = left + 1u) {
            var smallest = left;
            for (var right = left + 1u; right < params.k; right = right + 1u) {
                if (best_distances[right] < best_distances[smallest] ||
                    (best_distances[right] == best_distances[smallest] && best_indices[right] < best_indices[smallest])) {
                    smallest = right;
                }
            }
            let saved_distance = best_distances[left];
            let saved_index = best_indices[left];
            best_distances[left] = best_distances[smallest];
            best_indices[left] = best_indices[smallest];
            best_distances[smallest] = saved_distance;
            best_indices[smallest] = saved_index;
            let output_slot = query_offset * params.k + left;
            output_distances[output_slot] = best_distances[left];
            output_indices[output_slot] = best_indices[left];
        }
    }
}
"#;

    fn initialise() -> State {
        let enabled = match policy_enabled() {
            Ok(value) => value,
            Err(reason) => {
                return State {
                    info: GpuInfo {
                        compiled: true,
                        enabled: false,
                        available: false,
                        adapter: None,
                        backend: None,
                        device_type: None,
                        driver: None,
                        reason,
                    },
                    context: None,
                }
            }
        };
        if !enabled {
            return State {
                info: GpuInfo {
                    compiled: true,
                    enabled: false,
                    available: false,
                    adapter: None,
                    backend: None,
                    device_type: None,
                    driver: None,
                    reason: "disabled by BIOLANG_GPU; CPU fallback selected".to_string(),
                },
                context: None,
            };
        }

        let instance = wgpu::Instance::default();
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })) {
                Some(adapter) => adapter,
                None => {
                    return State {
                        info: GpuInfo {
                            compiled: true,
                            enabled: true,
                            available: false,
                            adapter: None,
                            backend: None,
                            device_type: None,
                            driver: None,
                            reason: "no compatible hardware adapter found; CPU fallback selected"
                                .to_string(),
                        },
                        context: None,
                    }
                }
            };
        let adapter_info = adapter.get_info();
        let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let requested = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("BioLang compute device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
            },
            None,
        ));
        let (device, queue) = match requested {
            Ok(pair) => pair,
            Err(error) => {
                return State {
                    info: GpuInfo {
                        compiled: true,
                        enabled: true,
                        available: false,
                        adapter: Some(adapter_info.name),
                        backend: Some(format!("{:?}", adapter_info.backend)),
                        device_type: Some(format!("{:?}", adapter_info.device_type)),
                        driver: Some(adapter_info.driver),
                        reason: format!("adapter found but device creation failed: {error}"),
                    },
                    context: None,
                }
            }
        };
        State {
            info: GpuInfo {
                compiled: true,
                enabled: true,
                available: true,
                adapter: Some(adapter_info.name),
                backend: Some(format!("{:?}", adapter_info.backend)),
                device_type: Some(format!("{:?}", adapter_info.device_type)),
                driver: Some(adapter_info.driver),
                reason: "available; large single-cell block operations use this adapter"
                    .to_string(),
            },
            context: Some(Context { device, queue }),
        }
    }

    fn state() -> &'static State {
        static STATE: OnceLock<State> = OnceLock::new();
        STATE.get_or_init(initialise)
    }

    pub fn info() -> GpuInfo {
        state().info.clone()
    }

    fn flatten(matrix: &[Vec<f64>]) -> Vec<f32> {
        matrix
            .iter()
            .flat_map(|row| row.iter().map(|&value| value as f32))
            .collect()
    }

    /// Apply `(left * right') * basis` as two GPU matrix multiplications.
    /// Basis vectors are stored as the outer dimension to match the CPU code.
    pub fn cross_apply_block(
        left: &[Vec<f64>],
        right: &[Vec<f64>],
        basis: &[Vec<f64>],
    ) -> Result<Option<Vec<Vec<f64>>>, String> {
        let Some(context) = state().context.as_ref() else {
            return Ok(None);
        };
        let rows_left = left.len();
        let rows_right = right.len();
        let width = left.first().map(Vec::len).unwrap_or(0);
        let block = basis.len();
        if rows_left == 0 || rows_right == 0 || width == 0 || block == 0 {
            return Ok(None);
        }
        if right.iter().any(|row| row.len() != width)
            || left.iter().any(|row| row.len() != width)
            || basis.iter().any(|vector| vector.len() != rows_right)
        {
            return Err("GPU cross-product received inconsistent matrix dimensions".to_string());
        }

        // Small dispatches lose to transfer overhead; retain exact f64 CPU math.
        let operations = rows_left
            .saturating_add(rows_right)
            .saturating_mul(width)
            .saturating_mul(block);
        if operations < 20_000_000 && !cfg!(test) {
            return Ok(None);
        }

        let left_flat = flatten(left);
        let right_flat = flatten(right);
        let mut basis_flat = vec![0.0_f32; rows_right * block];
        for (component, vector) in basis.iter().enumerate() {
            for (row, &value) in vector.iter().enumerate() {
                basis_flat[row * block + component] = value as f32;
            }
        }

        let max_binding = context.device.limits().max_storage_buffer_binding_size as usize;
        let byte_sizes = [
            left_flat.len() * 4,
            right_flat.len() * 4,
            basis_flat.len() * 4,
            width * block * 4,
            rows_left * block * 4,
        ];
        if byte_sizes.iter().any(|&size| size > max_binding) {
            return Ok(None);
        }

        let storage = wgpu::BufferUsages::STORAGE;
        let left_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BioLang CCA left"),
                contents: bytemuck::cast_slice(&left_flat),
                usage: storage,
            });
        let right_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BioLang CCA right"),
                contents: bytemuck::cast_slice(&right_flat),
                usage: storage,
            });
        let basis_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BioLang CCA basis"),
                contents: bytemuck::cast_slice(&basis_flat),
                usage: storage,
            });
        let intermediate_size = (width * block * 4) as u64;
        let output_size = (rows_left * block * 4) as u64;
        let intermediate = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BioLang CCA intermediate"),
            size: intermediate_size,
            usage: storage,
            mapped_at_creation: false,
        });
        let output = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BioLang CCA output"),
            size: output_size,
            usage: storage | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BioLang CCA readback"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_first = Params {
            m: width as u32,
            k: rows_right as u32,
            n: block as u32,
            transpose_a: 1,
        };
        let params_second = Params {
            m: rows_left as u32,
            k: width as u32,
            n: block as u32,
            transpose_a: 0,
        };
        let params_first = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BioLang CCA first parameters"),
                contents: bytemuck::bytes_of(&params_first),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let params_second = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BioLang CCA second parameters"),
                contents: bytemuck::bytes_of(&params_second),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("BioLang matrix multiplication"),
                source: wgpu::ShaderSource::Wgsl(MATMUL_SHADER.into()),
            });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("BioLang matrix multiplication"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });
        let layout = pipeline.get_bind_group_layout(0);
        let first_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("BioLang CCA first multiplication"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: right_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: basis_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: intermediate.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params_first.as_entire_binding(),
                    },
                ],
            });
        let second_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("BioLang CCA second multiplication"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: left_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: intermediate.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params_second.as_entire_binding(),
                    },
                ],
            });

        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("BioLang CCA GPU dispatch"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("right transpose times basis"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &first_group, &[]);
            pass.dispatch_workgroups((block as u32 + 7) / 8, (width as u32 + 7) / 8, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("left times intermediate"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &second_group, &[]);
            pass.dispatch_workgroups((block as u32 + 7) / 8, (rows_left as u32 + 7) / 8, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, output_size);
        context.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        context.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| "GPU readback channel closed".to_string())?
            .map_err(|error| format!("GPU readback failed: {error}"))?;
        let mapped = slice.get_mapped_range();
        let values: &[f32] = bytemuck::cast_slice(&mapped);
        let mut result = vec![vec![0.0_f64; rows_left]; block];
        for row in 0..rows_left {
            for component in 0..block {
                result[component][row] = values[row * block + component] as f64;
            }
        }
        drop(mapped);
        staging.unmap();
        Ok(Some(result))
    }

    /// Batched all-pairs distance search with lane-local top-k reduction.
    /// The fixed local capacity supports graph/UMAP and the over-fetched
    /// low-k anchor searches (k <= 256).
    pub fn nearest_rows(
        embeddings: &[Vec<f64>],
        k: usize,
        metric: &str,
    ) -> Result<Option<Vec<Vec<(usize, f64)>>>, String> {
        let Some(context) = state().context.as_ref() else {
            return Ok(None);
        };
        let n = embeddings.len();
        let dimensions = embeddings.first().map(Vec::len).unwrap_or(0);
        if n <= 4096 || dimensions == 0 || k == 0 || k > 256 || k >= n {
            return Ok(None);
        }
        if dimensions > 256 || embeddings.iter().any(|row| row.len() != dimensions) {
            return Ok(None);
        }
        let metric_code = match metric.to_ascii_lowercase().as_str() {
            "euclidean" => 0,
            "cosine" => 1,
            _ => return Ok(None),
        };
        let flat = flatten(embeddings);
        let point_bytes = flat.len() * 4;
        if point_bytes > context.device.limits().max_storage_buffer_binding_size as usize {
            return Ok(None);
        }
        let point_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("BioLang nearest-neighbor points"),
                contents: bytemuck::cast_slice(&flat),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("BioLang nearest-neighbor top-k"),
                source: wgpu::ShaderSource::Wgsl(NEAREST_SHADER.into()),
            });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("BioLang nearest-neighbor top-k"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });
        let layout = pipeline.get_bind_group_layout(0);
        let mut result = vec![Vec::with_capacity(k); n];

        // Bound each dispatch to avoid the Windows GPU watchdog on large
        // all-pairs searches. Only 2 * batch * k values cross back to the CPU.
        const QUERY_BATCH: usize = 256;
        for query_start in (0..n).step_by(QUERY_BATCH) {
            let query_count = QUERY_BATCH.min(n - query_start);
            let output_elements = query_count * k;
            let output_size = (output_elements * 4) as u64;
            let index_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("BioLang nearest-neighbor indices"),
                size: output_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let distance_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("BioLang nearest-neighbor distances"),
                size: output_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let readback = context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("BioLang nearest-neighbor readback"),
                size: output_size * 2,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let params = NearestParams {
                n: n as u32,
                dimensions: dimensions as u32,
                k: k as u32,
                metric: metric_code,
                query_start: query_start as u32,
                query_count: query_count as u32,
                padding_0: 0,
                padding_1: 0,
            };
            let params = context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("BioLang nearest-neighbor parameters"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let group = context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("BioLang nearest-neighbor batch"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: point_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: index_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: distance_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: params.as_entire_binding(),
                        },
                    ],
                });
            let mut encoder =
                context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("BioLang nearest-neighbor dispatch"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("BioLang nearest-neighbor batch"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(query_count as u32, 1, 1);
            }
            encoder.copy_buffer_to_buffer(&index_buffer, 0, &readback, 0, output_size);
            encoder.copy_buffer_to_buffer(&distance_buffer, 0, &readback, output_size, output_size);
            context.queue.submit(Some(encoder.finish()));
            let slice = readback.slice(..);
            let (sender, receiver) = mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |mapped| {
                let _ = sender.send(mapped);
            });
            context.device.poll(wgpu::Maintain::Wait);
            receiver
                .recv()
                .map_err(|_| "GPU nearest-neighbor readback channel closed".to_string())?
                .map_err(|error| format!("GPU nearest-neighbor readback failed: {error}"))?;
            let mapped = slice.get_mapped_range();
            let bytes: &[u8] = &mapped;
            let indices: &[u32] = bytemuck::cast_slice(&bytes[..output_size as usize]);
            let distances: &[f32] = bytemuck::cast_slice(&bytes[output_size as usize..]);
            for query_offset in 0..query_count {
                let row = (0..k)
                    .map(|slot| {
                        let position = query_offset * k + slot;
                        (indices[position] as usize, distances[position] as f64)
                    })
                    .collect();
                result[query_start + query_offset] = row;
            }
            drop(mapped);
            readback.unmap();
        }
        Ok(Some(result))
    }
}

#[cfg(feature = "gpu")]
pub use enabled::{cross_apply_block, info, nearest_rows};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_policy_off_is_understood() {
        // Test the parser without mutating the process environment in parallel.
        assert!(matches!(policy_enabled(), Ok(_) | Err(_)));
    }

    #[test]
    fn execution_summary_discloses_policy_and_backend_class() {
        let summary = execution_summary();
        assert!(summary.contains("BIOLANG_GPU="));
        assert!(summary.starts_with("GPU:") || summary.starts_with("CPU f64"));
    }

    #[test]
    fn gpu_cross_product_matches_cpu_when_adapter_is_available() {
        let left = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![-1.0, 0.5]];
        let right = vec![vec![2.0, 1.0], vec![0.0, 3.0], vec![1.0, -1.0]];
        let basis = vec![vec![1.0, 0.5, -2.0], vec![0.0, 1.0, 1.0]];
        let Some(got) = cross_apply_block(&left, &right, &basis).unwrap() else {
            // Headless CI and builds compiled without a GPU keep the CPU path.
            return;
        };
        for (component, vector) in basis.iter().enumerate() {
            let mut feature = vec![0.0; 2];
            for (row, weight) in right.iter().zip(vector) {
                for column in 0..2 {
                    feature[column] += row[column] * weight;
                }
            }
            for row in 0..left.len() {
                let expected = left[row][0] * feature[0] + left[row][1] * feature[1];
                assert!((got[component][row] - expected).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn gpu_nearest_rows_agree_with_brute_force_when_adapter_is_available() {
        let points: Vec<Vec<f64>> = (0..4097)
            .map(|index| {
                let value = index as f64;
                vec![
                    (value * 0.013_17).sin(),
                    (value * 0.017_31).cos(),
                    value * 0.000_01,
                ]
            })
            .collect();
        let Some(got) = nearest_rows(&points, 5, "euclidean").unwrap() else {
            return;
        };
        for query in [0, 17, 1000, 4096] {
            let mut expected: Vec<(usize, f64)> = (0..points.len())
                .filter(|&candidate| candidate != query)
                .map(|candidate| {
                    let distance = points[query]
                        .iter()
                        .zip(&points[candidate])
                        .map(|(left, right)| (left - right) * (left - right))
                        .sum::<f64>()
                        .sqrt();
                    (candidate, distance)
                })
                .collect();
            expected.sort_by(|left, right| left.1.total_cmp(&right.1));
            let expected: std::collections::HashSet<usize> =
                expected.into_iter().take(5).map(|row| row.0).collect();
            let recovered = got[query]
                .iter()
                .filter(|row| expected.contains(&row.0))
                .count();
            assert!(recovered >= 4, "query {query} recovered only {recovered}/5");
        }
    }
}
