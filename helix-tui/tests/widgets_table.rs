// use helix_tui::{
//     backend::TestBackend,
//     buffer::Buffer,
//     layout::Constraint,
//     style::{Color, Modifier, Style},
//     text::{Span, Spans},
//     widgets::{Block, Borders, Cell, Row, Table, TableState},
//     Terminal,
// };

// #[test]
// fn widgets_table_column_spacing_can_be_changed() {
//     let test_case = |column_spacing, expected| {
//         let backend = TestBackend::new(30, 10);
//         let mut terminal = Terminal::new(backend).unwrap();

//         terminal
//             .draw(|f| {
//                 let size = f.size();
//                 let table = Table::new(vec![
//                     Row::new(vec!["Row11", "Row12", "Row13"]),
//                     Row::new(vec!["Row21", "Row22", "Row23"]),
//                     Row::new(vec!["Row31", "Row32", "Row33"]),
//                     Row::new(vec!["Row41", "Row42", "Row43"]),
//                 ])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//                 .block(Block::bordered())
//                 .widths(&[
//                     Constraint::Length(5),
//                     Constraint::Length(5),
//                     Constraint::Length(5),
//                 ])
//                 .column_spacing(column_spacing);
//                 f.render_widget(table, size);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&expected);
//     };

//     // no space between columns
//     test_case(
//         0,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1Head2Head3             │",
//             "│                            │",
//             "│Row11Row12Row13             │",
//             "│Row21Row22Row23             │",
//             "│Row31Row32Row33             │",
//             "│Row41Row42Row43             │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // one space between columns
//     test_case(
//         1,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1 Head2 Head3           │",
//             "│                            │",
//             "│Row11 Row12 Row13           │",
//             "│Row21 Row22 Row23           │",
//             "│Row31 Row32 Row33           │",
//             "│Row41 Row42 Row43           │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // enough space to just not hide the third column
//     test_case(
//         6,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1      Head2      Head3 │",
//             "│                            │",
//             "│Row11      Row12      Row13 │",
//             "│Row21      Row22      Row23 │",
//             "│Row31      Row32      Row33 │",
//             "│Row41      Row42      Row43 │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // enough space to hide part of the third column
//     test_case(
//         7,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1       Head2       Head│",
//             "│                            │",
//             "│Row11       Row12       Row1│",
//             "│Row21       Row22       Row2│",
//             "│Row31       Row32       Row3│",
//             "│Row41       Row42       Row4│",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );
// }

// #[test]
// fn widgets_table_columns_widths_can_use_fixed_length_constraints() {
//     let test_case = |widths, expected| {
//         let backend = TestBackend::new(30, 10);
//         let mut terminal = Terminal::new(backend).unwrap();

//         terminal
//             .draw(|f| {
//                 let size = f.size();
//                 let table = Table::new(vec![
//                     Row::new(vec!["Row11", "Row12", "Row13"]),
//                     Row::new(vec!["Row21", "Row22", "Row23"]),
//                     Row::new(vec!["Row31", "Row32", "Row33"]),
//                     Row::new(vec!["Row41", "Row42", "Row43"]),
//                 ])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//                 .block(Block::bordered())
//                 .widths(widths);
//                 f.render_widget(table, size);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&expected);
//     };

//     // columns of zero width show nothing
//     test_case(
//         &[
//             Constraint::Length(0),
//             Constraint::Length(0),
//             Constraint::Length(0),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of 1 width trim
//     test_case(
//         &[
//             Constraint::Length(1),
//             Constraint::Length(1),
//             Constraint::Length(1),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│H H H                       │",
//             "│                            │",
//             "│R R R                       │",
//             "│R R R                       │",
//             "│R R R                       │",
//             "│R R R                       │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of large width just before pushing a column off
//     test_case(
//         &[
//             Constraint::Length(8),
//             Constraint::Length(8),
//             Constraint::Length(8),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1    Head2    Head3     │",
//             "│                            │",
//             "│Row11    Row12    Row13     │",
//             "│Row21    Row22    Row23     │",
//             "│Row31    Row32    Row33     │",
//             "│Row41    Row42    Row43     │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );
// }

// #[test]
// fn widgets_table_columns_widths_can_use_percentage_constraints() {
//     let test_case = |widths, expected| {
//         let backend = TestBackend::new(30, 10);
//         let mut terminal = Terminal::new(backend).unwrap();

//         terminal
//             .draw(|f| {
//                 let size = f.size();
//                 let table = Table::new(vec![
//                     Row::new(vec!["Row11", "Row12", "Row13"]),
//                     Row::new(vec!["Row21", "Row22", "Row23"]),
//                     Row::new(vec!["Row31", "Row32", "Row33"]),
//                     Row::new(vec!["Row41", "Row42", "Row43"]),
//                 ])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//                 .block(Block::bordered())
//                 .widths(widths)
//                 .column_spacing(0);
//                 f.render_widget(table, size);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&expected);
//     };

//     // columns of zero width show nothing
//     test_case(
//         &[
//             Constraint::Percentage(0),
//             Constraint::Percentage(0),
//             Constraint::Percentage(0),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of not enough width trims the data
//     test_case(
//         &[
//             Constraint::Percentage(11),
//             Constraint::Percentage(11),
//             Constraint::Percentage(11),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│HeaHeaHea                   │",
//             "│                            │",
//             "│RowRowRow                   │",
//             "│RowRowRow                   │",
//             "│RowRowRow                   │",
//             "│RowRowRow                   │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of large width just before pushing a column off
//     test_case(
//         &[
//             Constraint::Percentage(33),
//             Constraint::Percentage(33),
//             Constraint::Percentage(33),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1    Head2    Head3     │",
//             "│                            │",
//             "│Row11    Row12    Row13     │",
//             "│Row21    Row22    Row23     │",
//             "│Row31    Row32    Row33     │",
//             "│Row41    Row42    Row43     │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // percentages summing to 100 should give equal widths
//     test_case(
//         &[Constraint::Percentage(50), Constraint::Percentage(50)],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1         Head2         │",
//             "│                            │",
//             "│Row11         Row12         │",
//             "│Row21         Row22         │",
//             "│Row31         Row32         │",
//             "│Row41         Row42         │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );
// }

// #[test]
// fn widgets_table_columns_widths_can_use_mixed_constraints() {
//     let test_case = |widths, expected| {
//         let backend = TestBackend::new(30, 10);
//         let mut terminal = Terminal::new(backend).unwrap();

//         terminal
//             .draw(|f| {
//                 let size = f.size();
//                 let table = Table::new(vec![
//                     Row::new(vec!["Row11", "Row12", "Row13"]),
//                     Row::new(vec!["Row21", "Row22", "Row23"]),
//                     Row::new(vec!["Row31", "Row32", "Row33"]),
//                     Row::new(vec!["Row41", "Row42", "Row43"]),
//                 ])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//                 .block(Block::bordered())
//                 .widths(widths);
//                 f.render_widget(table, size);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&expected);
//     };

//     // columns of zero width show nothing
//     test_case(
//         &[
//             Constraint::Percentage(0),
//             Constraint::Length(0),
//             Constraint::Percentage(0),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of not enough width trims the data
//     test_case(
//         &[
//             Constraint::Percentage(11),
//             Constraint::Length(20),
//             Constraint::Percentage(11),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Hea Head2                Hea│",
//             "│                            │",
//             "│Row Row12                Row│",
//             "│Row Row22                Row│",
//             "│Row Row32                Row│",
//             "│Row Row42                Row│",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of large width just before pushing a column off
//     test_case(
//         &[
//             Constraint::Percentage(33),
//             Constraint::Length(10),
//             Constraint::Percentage(33),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1     Head2      Head3  │",
//             "│                            │",
//             "│Row11     Row12      Row13  │",
//             "│Row21     Row22      Row23  │",
//             "│Row31     Row32      Row33  │",
//             "│Row41     Row42      Row43  │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of large size (>100% total) hide the last column
//     test_case(
//         &[
//             Constraint::Percentage(60),
//             Constraint::Length(10),
//             Constraint::Percentage(60),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1            Head2      │",
//             "│                            │",
//             "│Row11            Row12      │",
//             "│Row21            Row22      │",
//             "│Row31            Row32      │",
//             "│Row41            Row42      │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );
// }

// #[test]
// fn widgets_table_columns_widths_can_use_ratio_constraints() {
//     let test_case = |widths, expected| {
//         let backend = TestBackend::new(30, 10);
//         let mut terminal = Terminal::new(backend).unwrap();

//         terminal
//             .draw(|f| {
//                 let size = f.size();
//                 let table = Table::new(vec![
//                     Row::new(vec!["Row11", "Row12", "Row13"]),
//                     Row::new(vec!["Row21", "Row22", "Row23"]),
//                     Row::new(vec!["Row31", "Row32", "Row33"]),
//                     Row::new(vec!["Row41", "Row42", "Row43"]),
//                 ])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//                 .block(Block::bordered())
//                 .widths(widths)
//                 .column_spacing(0);
//                 f.render_widget(table, size);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&expected);
//     };

//     // columns of zero width show nothing
//     test_case(
//         &[
//             Constraint::Ratio(0, 1),
//             Constraint::Ratio(0, 1),
//             Constraint::Ratio(0, 1),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of not enough width trims the data
//     test_case(
//         &[
//             Constraint::Ratio(1, 9),
//             Constraint::Ratio(1, 9),
//             Constraint::Ratio(1, 9),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│HeaHeaHea                   │",
//             "│                            │",
//             "│RowRowRow                   │",
//             "│RowRowRow                   │",
//             "│RowRowRow                   │",
//             "│RowRowRow                   │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // columns of large width just before pushing a column off
//     test_case(
//         &[
//             Constraint::Ratio(1, 3),
//             Constraint::Ratio(1, 3),
//             Constraint::Ratio(1, 3),
//         ],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1    Head2    Head3     │",
//             "│                            │",
//             "│Row11    Row12    Row13     │",
//             "│Row21    Row22    Row23     │",
//             "│Row31    Row32    Row33     │",
//             "│Row41    Row42    Row43     │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // percentages summing to 100 should give equal widths
//     test_case(
//         &[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)],
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1         Head2         │",
//             "│                            │",
//             "│Row11         Row12         │",
//             "│Row21         Row22         │",
//             "│Row31         Row32         │",
//             "│Row41         Row42         │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );
// }

// #[test]
// fn widgets_table_can_have_rows_with_multi_lines() {
//     let test_case = |state: &mut TableState, expected: Buffer| {
//         let backend = TestBackend::new(30, 8);
//         let mut terminal = Terminal::new(backend).unwrap();
//         terminal
//             .draw(|f| {
//                 let size = f.size();
//                 let table = Table::new(vec![
//                     Row::new(vec!["Row11", "Row12", "Row13"]),
//                     Row::new(vec!["Row21", "Row22", "Row23"]).height(2),
//                     Row::new(vec!["Row31", "Row32", "Row33"]),
//                     Row::new(vec!["Row41", "Row42", "Row43"]).height(2),
//                 ])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//                 .block(Block::bordered())
//                 .highlight_symbol(">> ")
//                 .widths(&[
//                     Constraint::Length(5),
//                     Constraint::Length(5),
//                     Constraint::Length(5),
//                 ])
//                 .column_spacing(1);
//                 f.render_stateful_widget(table, size, state);
//             })
//             .unwrap();
//         terminal.backend().assert_buffer(&expected);
//     };

//     let mut state = TableState::default();
//     // no selection
//     test_case(
//         &mut state,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│Head1 Head2 Head3           │",
//             "│                            │",
//             "│Row11 Row12 Row13           │",
//             "│Row21 Row22 Row23           │",
//             "│                            │",
//             "│Row31 Row32 Row33           │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // select first
//     state.select(Some(0));
//     test_case(
//         &mut state,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│   Head1 Head2 Head3        │",
//             "│                            │",
//             "│>> Row11 Row12 Row13        │",
//             "│   Row21 Row22 Row23        │",
//             "│                            │",
//             "│   Row31 Row32 Row33        │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // select second (we don't show partially the 4th row)
//     state.select(Some(1));
//     test_case(
//         &mut state,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│   Head1 Head2 Head3        │",
//             "│                            │",
//             "│   Row11 Row12 Row13        │",
//             "│>> Row21 Row22 Row23        │",
//             "│                            │",
//             "│   Row31 Row32 Row33        │",
//             "└────────────────────────────┘",
//         ]),
//     );

//     // select 4th (we don't show partially the 1st row)
//     state.select(Some(3));
//     test_case(
//         &mut state,
//         Buffer::with_lines(vec![
//             "┌────────────────────────────┐",
//             "│   Head1 Head2 Head3        │",
//             "│                            │",
//             "│   Row31 Row32 Row33        │",
//             "│>> Row41 Row42 Row43        │",
//             "│                            │",
//             "│                            │",
//             "└────────────────────────────┘",
//         ]),
//     );
// }

// #[test]
// fn widgets_table_can_have_elements_styled_individually() {
//     let backend = TestBackend::new(30, 4);
//     let mut terminal = Terminal::new(backend).unwrap();
//     let mut state = TableState::default();
//     state.select(Some(0));
//     terminal
//         .draw(|f| {
//             let size = f.size();
//             let table = Table::new(vec![
//                 Row::new(vec!["Row11", "Row12", "Row13"]).style(Style::default().fg(Color::Green)),
//                 Row::new(vec![
//                     Cell::from("Row21"),
//                     Cell::from("Row22").style(Style::default().fg(Color::Yellow)),
//                     Cell::from(Spans::from(vec![
//                         Span::raw("Row"),
//                         Span::styled("23", Style::default().fg(Color::Blue)),
//                     ]))
//                     .style(Style::default().fg(Color::Red)),
//                 ])
//                 .style(Style::default().fg(Color::LightGreen)),
//             ])
//             .header(Row::new(vec!["Head1", "Head2", "Head3"]).bottom_margin(1))
//             .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
//             .highlight_symbol(">> ")
//             .highlight_style(Style::default().add_modifier(Modifier::BOLD))
//             .widths(&[
//                 Constraint::Length(6),
//                 Constraint::Length(6),
//                 Constraint::Length(6),
//             ])
//             .column_spacing(1);
//             f.render_stateful_widget(table, size, &mut state);
//         })
//         .unwrap();

//     let mut expected = Buffer::with_lines(vec![
//         "│   Head1  Head2  Head3      │",
//         "│                            │",
//         "│>> Row11  Row12  Row13      │",
//         "│   Row21  Row22  Row23      │",
//     ]);
//     // First row = row color + highlight style
//     for col in 1..=28 {
//         expected.get_mut(col, 2).set_style(
//             Style::default()
//                 .fg(Color::Green)
//                 .add_modifier(Modifier::BOLD),
//         );
//     }
//     // Second row:
//     // 1. row color
//     for col in 1..=28 {
//         expected
//             .get_mut(col, 3)
//             .set_style(Style::default().fg(Color::LightGreen));
//     }
//     // 2. cell color
//     for col in 11..=16 {
//         expected
//             .get_mut(col, 3)
//             .set_style(Style::default().fg(Color::Yellow));
//     }
//     for col in 18..=23 {
//         expected
//             .get_mut(col, 3)
//             .set_style(Style::default().fg(Color::Red));
//     }
//     // 3. text color
//     for col in 21..=22 {
//         expected
//             .get_mut(col, 3)
//             .set_style(Style::default().fg(Color::Blue));
//     }
//     terminal.backend().assert_buffer(&expected);
// }

// #[test]
// fn widgets_table_should_render_even_if_empty() {
//     let backend = TestBackend::new(30, 4);
//     let mut terminal = Terminal::new(backend).unwrap();
//     terminal
//         .draw(|f| {
//             let size = f.size();
//             let table = Table::new(vec![])
//                 .header(Row::new(vec!["Head1", "Head2", "Head3"]))
//                 .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
//                 .widths(&[
//                     Constraint::Length(6),
//                     Constraint::Length(6),
//                     Constraint::Length(6),
//                 ])
//                 .column_spacing(1);
//             f.render_widget(table, size);
//         })
//         .unwrap();

//     let expected = Buffer::with_lines(vec![
//         "│Head1  Head2  Head3         │",
//         "│                            │",
//         "│                            │",
//         "│                            │",
//     ]);

//     terminal.backend().assert_buffer(&expected);
// }
