local M = {}

local selected_paths = ya.sync(function()
  local paths = {}
  local selected = cx.active.selected
  if selected ~= nil then
    for _, url in pairs(selected) do
      paths[#paths + 1] = tostring(url)
    end
  end
  if #paths == 0 then
    local hovered = cx.active.current.hovered
    if hovered ~= nil then
      paths[#paths + 1] = tostring(hovered.url)
    end
  end
  return paths
end)

function M:entry(_)
  local paths = selected_paths()
  if #paths == 0 then
    ya.notify({
      title = "Keynear",
      content = "No files or folders selected.",
      timeout = 3,
      level = "warn",
    })
    return
  end

  local cmd = Command("keynear"):arg("send"):arg("--")
  for _, path in ipairs(paths) do
    cmd = cmd:arg(path)
  end

  local child, err = cmd:spawn()
  if not child then
    ya.notify({
      title = "Keynear",
      content = "Failed to launch keynear: " .. tostring(err),
      timeout = 4,
      level = "error",
    })
  end
end

return M
