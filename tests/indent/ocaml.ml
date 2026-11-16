let process items =
(*                 ^ 2 eof *)
  let total = ref 0 in
  List.iter (fun v ->
    total := !total + v) items;
  match !total with
  | 0 -> "zero"
  | _ -> "many"

let config = {
  name = "test";
  age = 1;
}

let nums = [
  1;
  2;
]

let classify x =
  if x > 0 then
(*             ^ 4 eof *)
    "pos"
  else
    "neg"

module M = struct
  let x = 1
  let y = 2
end
