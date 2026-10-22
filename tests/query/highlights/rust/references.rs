fn takes(x: &mut i32) -> &mut i32 {
//          ^ @keyword.storage.modifier.ref
//           ^ @keyword.storage.modifier.mut
//                       ^ @keyword.storage.modifier.ref
//                        ^ @keyword.storage.modifier.mut
    let mut y = x;
//      ^ @keyword.storage.modifier.mut
    let r = &mut y;
//          ^ @operator
//           ^ @operator
    r
}
