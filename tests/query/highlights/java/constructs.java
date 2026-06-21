class Box {
//    ^ @type
    boolean method(int count) {
//  ^ @type.builtin
//          ^ @function.method
        String s = "a\tb";
//      ^ @type
//                   ^ @constant.character.escape
        return helper(count);
//             ^ @function.method
    }
}
