fn sum(xs: []u32) u32 {
//     ^ @variable.parameter
    var total: u32 = 0;
//      ^ @variable
    for (xs) |item| {
//       ^ @variable.parameter
//            ^ @variable.parameter
        total += item;
//               ^ @variable.parameter
    }
}
