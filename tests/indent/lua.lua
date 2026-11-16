local M = {}

local function add(a, b)
  return a + b
end

function M.process(items)
  local total = 0
  for i, v in ipairs(items) do
--                            ^ 4 eof
    if v > 0 then
--               ^ 6 eof
      total = total + v
    else
      total = total - v
    end
  end
  local config = {
    name = "test",
    values = {
      1,
      2,
    },
  }
  return total, config
end

return M
