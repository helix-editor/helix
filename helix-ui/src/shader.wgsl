
// struct Vertex {
//     [[location(0)]] position: vec2<f32>;
// };

struct View {
    size: vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: View;

[[stage(vertex)]]
fn vs_main([[location(0)]] input: vec2<f32>) -> [[builtin(position)]] vec4<f32> {
    // TODO: scale by hidpi factor?
    return vec4<f32>(
        input.xy / view.size.xy * 2.0 * 1.5,
        0.0, 1.0
    );
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
