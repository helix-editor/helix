pub mod tui {
    pub mod buffer;
}
pub use tui::*;

criterion::criterion_main!(buffer::benches);
