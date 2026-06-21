const m = obj.method;
//            ^ @variable.other.member
const s = `${obj.compute()}`;
//               ^ @function.method
obj.first().second();
//  ^ @function.method
//           ^ @function.method
