// shaders/sgl_caustic.wgsl

struct FragmentInput {
    @location(0) uv: vec2<f32>,
};

struct OpticsParams {
    k_wavenumber: f32,
    einstein_radius: f32,
    astigmatism_j2: f32,
    time: f32,
};

@group(0) @binding(0) var<uniform> optics: OpticsParams;

fn bessel_j0(x: f32) -> f32 {
    let ax = abs(x);
    if (ax < 3.75) {
        let y = (x / 3.75) * (x / 3.75);
        return 1.0 + y * (-2.2499997 + y * (1.2656208 + y * (-0.3163866 + y * 0.0444479)));
    } else {
        let y = 3.75 / ax;
        let f0 = 0.79788456 + y * (-0.00000077 + y * (-0.00552740 + y * 0.00009512));
        let theta0 = ax - 0.78539816 + y * (-0.00000016 + y * (-0.00004166 + y * -0.00000039));
        return (1.0 / sqrt(ax)) * f0 * cos(theta0);
    }
}

@fragment
fn main_fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    let pos = (input.uv - vec2<f32>(0.5)) * 2.0;
    let r = length(pos);
    let phi = atan2(pos.y, pos.x);

    let b_pert = r + optics.astigmatism_j2 * cos(2.0 * phi);
    let arg = optics.k_wavenumber * b_pert * optics.einstein_radius;

    let j0_val = bessel_j0(arg);
    let intensity = j0_val * j0_val;

    let color = vec3<f32>(intensity * 1.0, intensity * 0.8, intensity * 0.2);
    return vec4<f32>(color, 1.0);
}
