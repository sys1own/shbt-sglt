// shaders/swarm_dynamics.wgsl

struct MNode {
    position: vec4<f32>,
    velocity: vec4<f32>,
};

struct SimParams {
    dt: f32,
    num_nodes: u32,
    solar_gm: f32,
    damping: f32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read_write> nodes: array<MNode>;

@compute @workgroup_size(256, 1, 1)
fn main_swarm(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    if (id >= params.num_nodes) {
        return;
    }

    var node = nodes[id];
    let pos = node.position.xyz;
    let vel = node.velocity.xyz;
    let r = length(pos);

    let acc_mag = -params.solar_gm / (r * r * r);
    let acc = pos * acc_mag - vel * params.damping;

    let new_pos = pos + vel * params.dt + 0.5 * acc * params.dt * params.dt;
    let new_vel = vel + acc * params.dt;

    nodes[id].position = vec4<f32>(new_pos, node.position.w);
    nodes[id].velocity = vec4<f32>(new_vel, node.velocity.w);
}
