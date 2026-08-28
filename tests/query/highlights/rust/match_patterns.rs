fn m(x: Shape) {
    match x {
        crate::Shape::Circle(r) => r,
//             ^ @type
//                    ^ @type.enum.variant
        geom::Style::Bold { weight } => weight,
//            ^ @type
//                   ^ @type.enum.variant
//                          ^ @variable.other.member
        Flag::MAX => 0,
//      ^ @type
//            ^ @constant
        _ => 1,
    };
}
