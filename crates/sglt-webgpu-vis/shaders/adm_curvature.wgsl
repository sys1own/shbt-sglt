// shaders/adm_curvature.wgsl

struct ADMMetricParams {
    solar_mass: f32,
    j2_quadrupole: f32,
    grid_dim: u32,
    scale_factor: f32,
};

@group(0) @binding(0) var<uniform> params: ADMMetricParams;
@group(0) @binding(1) var<storage, read_write> grid_vertices: array<vec4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn main_compute(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x_idx = global_id.x;
    let y_idx = global_id.y;
    
    if (x_idx >= params.grid_dim || y_idx >= params.grid_dim) {
        return;
    }

    let index = y_idx * params.grid_dim + x_idx;
    let grid_half = f32(params.grid_dim) / 2.0;
    
    let x = (f32(x_idx) - grid_half) * params.scale_factor;
    let y = (f32(y_idx) - grid_half) * params.scale_factor;
    let r = max(sqrt(x * x + y * y), 0.1);

    // 2PN Spatial metric perturbation g_rr - 1
    let u_pot = params.solar_mass / r;
    let j2_term = params.j2_quadrupole * (1.0 / (r * r)) * (-1.0); // Mid-plane P2
    let h_spatial = 2.0 * u_pot + 1.5 * u_pot * u_pot - j2_term;

    // Embed metric spatial deformation as Z vertex displacement
    grid_vertices[index] = vec4<f32>(x, y, -h_spatial * 10.0, h_spatial);
}
