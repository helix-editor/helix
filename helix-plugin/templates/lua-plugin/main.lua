-- my-plugin/main.lua

-- The 'helix' object is injected into the global Lua environment
-- and provides access to the Helix API.

-- Function called when the plugin is loaded
function on_load()
    helix.show_message("My Lua plugin has been loaded!")
    helix.register_command("my-plugin:lua-greeting", "on_lua_greeting_command")
    helix.subscribe_to_event("buffer_save", "on_lua_buffer_save_event")
end

-- Callback function for the registered command
function on_lua_greeting_command(...)
    local args = {...}
    local args_str = table.concat(args, ", ")
    helix.show_message("Hello from the Lua command! Arguments: " .. args_str)
    helix.get_buffer_content(0, 456) -- Example of requesting buffer content
end

-- Callback function for editor events
function on_lua_buffer_save_event(event_data)
    helix.show_message("buffer_save event received: " .. event_data)
end

-- Callback function for request responses
function on_response(request_id, response_data)
    helix.show_message("Response for request " .. request_id .. ": " .. response_data)
end
