class Widget {
//    ^ @type
    int field;
//      ^ @variable.other.member
    bool method(int count) {
//       ^ @function
//                  ^ @variable.parameter
        return helper(count);
//             ^ @function
    }
};
