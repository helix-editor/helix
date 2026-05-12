// use helix_tui::{
//     backend::TestBackend,
//     buffer::Buffer,
//     layout::Rect,
//     style::{Color, Style},
//     symbols,
//     widgets::{Block, Borders, List, ListItem, ListState},
//     Terminal,
// };

// #[test]
// fn widgets_list_should_highlight_the_selected_item() {
//     let backend = TestBackend::new(10, 3);
//     let mut terminal = Terminal::new(backend).unwrap();
//     let mut state = ListState::default();
//     state.select(Some(1));
//     terminal
//         .draw(|f| {
//             let size = f.size();
//             let items = vec![
//                 ListItem::new("Item 1"),
//                 ListItem::new("Item 2"),
//                 ListItem::new("Item 3"),
//             ];
//             let list = List::new(items)
//                 .highlight_style(Style::default().bg(Color::Yellow))
//                 .highlight_symbol(">> ");
//             f.render_stateful_widget(list, size, &mut state);
//         })
//         .unwrap();
//     let mut expected = Buffer::with_lines(vec!["   Item 1 ", ">> Item 2 ", "   Item 3 "]);
//     for x in 0..10 {
//         expected.get_mut(x, 1).set_bg(Color::Yellow);
//     }
//     terminal.backend().assert_buffer(&expected);
// }

// #[test]
// fn widgets_list_should_truncate_items() {
//     let backend = TestBackend::new(10, 2);
//     let mut terminal = Terminal::new(backend).unwrap();

//     struct TruncateTestCase<'a> {
//         selected: Option<usize>,
//         items: Vec<ListItem<'a>>,
//         expected: Buffer,
//     }

//     let cases = vec![
//         // An item is selected
//         TruncateTestCase {
//             selected: Some(0),
//             items: vec![
//                 ListItem::new("A very long line"),
//                 ListItem::new("A very long line"),
//             ],
//             expected: Buffer::with_lines(vec![
//                 format!(">> A ve{}  ", symbols::line::VERTICAL),
//                 format!("   A ve{}  ", symbols::line::VERTICAL),
//             ]),
//         },
//         // No item is selected
//         TruncateTestCase {
//             selected: None,
//             items: vec![
//                 ListItem::new("A very long line"),
//                 ListItem::new("A very long line"),
//             ],
//             expected: Buffer::with_lines(vec![
//                 format!("A very {}  ", symbols::line::VERTICAL),
//                 format!("A very {}  ", symbols::line::VERTICAL),
//             ]),
//         },
//     ];
//     for case in cases {
//         let mut state = ListState::default();
//         state.select(case.selected);
//         terminal
//             .draw(|f| {
//                 let list = List::new(case.items.clone())
//                     .block(Block::default().borders(Borders::RIGHT))
//                     .highlight_symbol(">> ");
//                 f.render_stateful_widget(list, Rect::new(0, 0, 8, 2), &mut state);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&case.expected);
//     }
// }
