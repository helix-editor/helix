//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use helix_tui::{
    backend::{Backend, TestBackend},
    Terminal,
};

#[test]
fn terminal_buffer_size_should_not_be_limited() {
    let backend = TestBackend::new(400, 400);
    let terminal = Terminal::new(backend).unwrap();
    let size = terminal.backend().size().unwrap();
    assert_eq!(size.width, 400);
    assert_eq!(size.height, 400);
}

// #[test]
// fn terminal_draw_returns_the_completed_frame() -> Result<(), Box<dyn Error>> {
//     let backend = TestBackend::new(10, 10);
//     let mut terminal = Terminal::new(backend)?;
//     let frame = terminal.draw(|f| {
//         let text = Text::from("Test");
//         let paragraph = Paragraph::new(&text);
//         f.render_widget(paragraph, f.size());
//     })?;
//     assert_eq!(frame.buffer.get(0, 0).symbol, "T");
//     assert_eq!(frame.area, Rect::new(0, 0, 10, 10));
//     terminal.backend_mut().resize(8, 8);
//     let frame = terminal.draw(|f| {
//         let text = Text::from("test");
//         let paragraph = Paragraph::new(&text);
//         f.render_widget(paragraph, f.size());
//     })?;
//     assert_eq!(frame.buffer.get(0, 0).symbol, "t");
//     assert_eq!(frame.area, Rect::new(0, 0, 8, 8));
//     Ok(())
// }
