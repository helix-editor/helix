use anyhow::Result;
use mlua::{Lua, Table, UserData, UserDataMethods};
use std::path::PathBuf;
use crate::api::HelixApi;

// Wrapper para HelixApi para que possa ser usada como UserData em mlua
struct LuaHelixApi(HelixApi);

impl UserData for LuaHelixApi {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("show_message", |_, this, message: String| {
            this.0.show_message(message)
                .map_err(|e| mlua::Error::external(format!("Host error: {}", e)))
        });

        methods.add_method("register_command", |_, this, (command_name, callback_function_name): (String, String)| {
            this.0.register_command(command_name, callback_function_name)
                .map_err(|e| mlua::Error::external(format!("Host error: {}", e)))
        });

        methods.add_method("get_buffer_content", |_, this, (doc_id, request_id): (u32, u32)| {
            this.0.get_buffer_content(doc_id, request_id)
                .map_err(|e| mlua::Error::external(format!("Host error: {}", e)))
        });
    }
}

pub struct LuaHost {
    lua: Lua,
}

impl LuaHost {
    pub fn new(lua_file: &PathBuf, helix_api: HelixApi) -> Result<Self> {
        let lua = Lua::new();

        // Registra a HelixApi como um global no ambiente Lua
        let helix_api_global = lua.create_userdata(LuaHelixApi(helix_api))?;
        lua.globals().set("helix", helix_api_global)?;

        // Carrega o arquivo Lua
        lua.load(std::fs::read_to_string(lua_file)?)
            .set_name(lua_file.to_str().unwrap_or("plugin"))?
            .exec()?;

        Ok(Self { lua })
    }

    // No futuro, podemos ter um método para chamar funções Lua específicas.
    pub fn call_function(&mut self, name: &str, args: &[String]) -> Result<()> {
        let func: mlua::Function = self.lua.globals().get(name)?;
        func.call::<Vec<String>, ()>(args.to_vec())?;
        Ok(())
    }

    /// Chama a função `on_response` no módulo Lua.
    pub fn on_response(&mut self, request_id: u32, response_data: String) -> Result<()> {
        let func: mlua::Function = self.lua.globals().get("on_response")?;
        func.call::<(u32, String), ()>((request_id, response_data))?;
        Ok(())
    }
}
}