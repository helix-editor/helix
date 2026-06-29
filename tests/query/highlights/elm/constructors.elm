type Color = Red | Blue
--   ^ @type
--           ^ @constructor
--                 ^ @constructor
f x = case x of
  Just y -> y
--^ @constructor
  Nothing -> 0
--^ @constructor
a = Just 1
--  ^ @constructor
