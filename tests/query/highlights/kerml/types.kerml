package Kernel {
//      ^ @namespace
    private import Base::*;
//          ^ @keyword.control.import
//                 ^ @namespace
    classifier Vehicle specializes Thing {
//  ^ @keyword.storage.type
//             ^ @type
//                     ^ @keyword
//                                 ^ @type
        feature mass : Real = 1200.5;
//      ^ @keyword.storage.type
//              ^ @variable
//                     ^ @type
//                            ^ @constant.numeric.float
        step drive : Drive;
//      ^ @keyword.function
//           ^ @function
        inv massIsPositive { mass > 0 and true }
//      ^ @keyword.storage.type
//          ^ @label
//                                ^ @operator
//                                    ^ @keyword.operator
//                                        ^ @constant.builtin.boolean
    }
    doc /* Documentation. */
//      ^ @comment.block.documentation
}
