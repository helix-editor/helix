type shape = Circle of float
(*           ^ @constructor *)
let area s = match s with
(*  ^ @function *)
  | Rectangle r -> r
(*  ^ @constructor *)
let p = { name = 1 }
(*        ^ @variable.other.member *)
module M = struct end
(*     ^ @namespace *)
