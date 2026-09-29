package Vehicles {
//      ^ @namespace
    private import ScalarValues::*;
//  ^ @keyword.storage.modifier
//          ^ @keyword.control.import
//                 ^ @namespace
    part def Vehicle :> Base {
//  ^ @keyword.storage.type
//       ^ @keyword.storage.type
//           ^ @type
//                      ^ @type
        attribute mass : MassValue = 1200.5 [kg];
//                ^ @variable
//                       ^ @type
//                                   ^ @constant.numeric.float
        part wheels : Wheel[4] ordered;
//           ^ @variable
//                    ^ @type
//                          ^ @constant.numeric.integer
        perform action drive : Drive;
//      ^ @keyword.function
//                     ^ @function
        exhibit state modes : Modes;
//                    ^ @label
        attribute ready : Boolean = wheels.count(mass) > 0 and true;
//                                  ^ @variable
//                                         ^ @function
//                                                      ^ @operator
//                                                          ^ @keyword.operator
//                                                              ^ @constant.builtin.boolean
    }
    doc /* Documentation. */
//  ^ @keyword
//      ^ @comment.block.documentation
    // A note.
//  ^ @comment
}
