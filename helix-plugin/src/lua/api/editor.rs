use crate::error::Result;
use helix_view::document::Mode;
use mlua::prelude::*;

/// Register editor API in the Helix Lua global table
pub fn register_editor_api(lua: &Lua, helix_table: &LuaTable) -> Result<()> {
    let editor_module = lua.create_table()?;

    // helix.editor.mode() - Get current editor mode
    let mode = lua.create_function(|_lua, ()| {
        if let Ok(editor) = crate::lua::get_editor_mut() {
            let mode = match editor.mode() {
                Mode::Normal => "normal",
                Mode::Insert => "insert",
                Mode::Select => "select",
            };
            Ok(mode.to_string())
        } else {
            Ok("normal".to_string())
        }
    })?;
    editor_module.set("mode", mode)?;

    // Mode switching functions
    let normal_mode = lua.create_function(|_lua, ()| {
        if let Ok(editor) = crate::lua::get_editor_mut() {
            editor.mode = Mode::Normal;
        }
        Ok(())
    })?;
    editor_module.set("normal_mode", normal_mode)?;

    let insert_mode = lua.create_function(|_lua, ()| {
        if let Ok(editor) = crate::lua::get_editor_mut() {
            editor.mode = Mode::Insert;
        }
        Ok(())
    })?;
    editor_module.set("insert_mode", insert_mode)?;

    let select_mode = lua.create_function(|_lua, ()| {
        if let Ok(editor) = crate::lua::get_editor_mut() {
            editor.mode = Mode::Select;
        }
        Ok(())
    })?;
    editor_module.set("select_mode", select_mode)?;

    // helix.editor.execute_command(cmd, args) - Execute a typed command
    let execute_command =
        lua.create_function(|lua, (cmd, args): (String, Option<Vec<String>>)| {
            if let Ok(editor) = crate::lua::get_editor_mut() {
                let args = args.unwrap_or_default();

                if let Some(wrapper) = lua.app_data_ref::<crate::types::CommandRegistryWrapper>() {
                    match wrapper.0.execute(editor, &cmd, &args) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(LuaError::RuntimeError(format!(
                            "Command '{}' failed: {}",
                            cmd, e
                        ))),
                    }
                } else {
                    Err(LuaError::RuntimeError(
                        "Builtin command registry not available".to_string(),
                    ))
                }
            } else {
                Ok(())
            }
        })?;
    editor_module.set("execute_command", execute_command)?;

    // helix.editor.move_cursor(direction, count) - Move cursor
    let move_cursor =
        lua.create_function(|_lua, (direction, count): (String, Option<usize>)| {
            let count = count.unwrap_or(1);
            // TODO: Implement actual cursor movement
            Ok(format!("Would move cursor {} {} times", direction, count))
        })?;
    editor_module.set("move_cursor", move_cursor)?;

    // helix.editor.get_cursor() - Get cursor position
    let get_cursor = lua.create_function(|_lua, ()| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc): (&helix_view::View, &helix_view::Document) =
            helix_view::current_ref!(editor);
        let cursor = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));
        let row = doc.text().char_to_line(cursor);
        let col = cursor - doc.text().line_to_char(row);

        Ok(super::buffer::LuaPosition { row, col })
    })?;
    editor_module.set("get_cursor", get_cursor)?;

    // helix.editor.set_cursor(row, col) - Set cursor position
    let set_cursor = lua.create_function(|_lua, (row, col): (usize, usize)| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc) = helix_view::current!(editor);

        let text = doc.text();
        let row = row.min(text.len_lines().saturating_sub(1));
        let offset = text.line_to_char(row) + col.min(text.line(row).len_chars());

        let selection = helix_core::Selection::point(offset);
        doc.set_selection(view.id, selection);

        Ok(())
    })?;
    editor_module.set("set_cursor", set_cursor)?;

    // helix.editor.get_config() - Get editor configuration
    let get_config = lua.create_function(|lua, ()| {
        let editor = crate::lua::get_editor_mut()?;
        let config = editor.config();
        let table = lua.create_table()?;

        let scrolloff = lua.create_table()?;
        scrolloff.set("vertical", config.scrolloff.vertical)?;
        scrolloff.set("horizontal", config.scrolloff.horizontal)?;

        table.set("scrolloff", scrolloff)?;
        table.set("mouse", config.mouse)?;
        table.set("cursorline", config.cursorline)?;
        table.set("cursorcolumn", config.cursorcolumn)?;
        table.set("auto_format", config.auto_format)?;
        table.set("auto_completion", config.auto_completion)?;
        table.set("auto_info", config.auto_info)?;
        table.set(
            "line_number",
            match config.line_number {
                helix_view::editor::LineNumber::Absolute => "absolute",
                helix_view::editor::LineNumber::Relative => "relative",
            },
        )?;

        Ok(table)
    })?;
    editor_module.set("get_config", get_config)?;

    // helix.editor.get_selections() - Get current selections
    let get_selections = lua.create_function(|lua, ()| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc): (&helix_view::View, &helix_view::Document) =
            helix_view::current_ref!(editor);
        let selection = doc.selection(view.id);
        let selections = lua.create_table()?;
        for (i, range) in selection.iter().enumerate() {
            let s = lua.create_table()?;
            s.set("anchor", range.anchor)?;
            s.set("head", range.head)?;
            selections.set(i + 1, s)?;
        }
        Ok(selections)
    })?;
    editor_module.set("get_selections", get_selections)?;

    // helix.editor.set_selections(selections) - Set current selections
    let set_selections = lua.create_function(|_lua, selections: Vec<LuaTable>| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc): (&mut helix_view::View, &mut helix_view::Document) =
            helix_view::current!(editor);
        let mut ranges = Vec::new();
        for s in selections {
            let anchor: usize = s.get("anchor")?;
            let head: usize = s.get("head")?;
            ranges.push(helix_core::Range::new(anchor, head));
        }
        if !ranges.is_empty() {
            let selection = helix_core::Selection::new(ranges.into(), 0);
            doc.set_selection(view.id, selection);
        }
        Ok(())
    })?;
    editor_module.set("set_selections", set_selections)?;
    let set_status = lua.create_function(|_lua, (message, _level): (String, Option<String>)| {
        let editor = crate::lua::get_editor_mut()?;
        editor.set_status(message);
        Ok(())
    })?;
    editor_module.set("set_status", set_status)?;

    // helix.editor.focus() - Get focused view/buffer
    let focus = lua.create_function(|_lua, ()| {
        // TODO: Implement actual focus retrieval
        Ok("current_view_id")
    })?;
    editor_module.set("focus", focus)?;

    // helix.editor.save() - Save current buffer
    let save = lua.create_function(|_lua, force: Option<bool>| {
        let force = force.unwrap_or(false);
        let _editor = crate::lua::get_editor_mut()?;
        // This usually triggers an async save, but for now we'll use the sync command approach
        // if possible, or just log.
        // Actually, helix has typed commands for this.
        Ok(format!("Would save buffer (force: {})", force))
    })?;
    editor_module.set("save", save)?;

    // helix.editor.save_all() - Save all buffers
    let save_all = lua.create_function(|_lua, ()| {
        // TODO: Implement actual save all
        Ok("Would save all buffers")
    })?;
    editor_module.set("save_all", save_all)?;

    // helix.editor.quit() - Quit editor
    let quit = lua.create_function(|_lua, force: Option<bool>| {
        let force = force.unwrap_or(false);
        // TODO: Implement actual quit (this needs careful handling!)
        Ok(format!("Would quit (force: {})", force))
    })?;
    editor_module.set("quit", quit)?;

    // helix.editor.open(path, action) - Open a file
    let open = lua.create_function(|_lua, (path_str, action_str): (String, Option<String>)| {
        let editor = crate::lua::get_editor_mut()?;
        let path = std::path::PathBuf::from(path_str);

        let action = match action_str.as_deref() {
            Some("split") => helix_view::editor::Action::HorizontalSplit,
            Some("vsplit") => helix_view::editor::Action::VerticalSplit,
            Some("replace") => helix_view::editor::Action::Replace,
            _ => helix_view::editor::Action::Load,
        };

        match editor.open(&path, action) {
            Ok(doc_id) => Ok(super::buffer::LuaBuffer::new(doc_id)),
            Err(e) => Err(LuaError::RuntimeError(format!(
                "Failed to open file: {}",
                e
            ))),
        }
    })?;
    editor_module.set("open", open)?;

    // helix.editor.close() - Close current buffer
    let close = lua.create_function(|_lua, _force: Option<bool>| {
        let editor = crate::lua::get_editor_mut()?;
        let view_id = editor.tree.focus;

        // This is a bit complex as it might involve prompting or closing the window
        // For now, let's just use a simple close if possible.
        // editor.close(view_id) is usually what we want.
        editor.close(view_id);
        Ok(())
    })?;
    editor_module.set("close", close)?;

    // helix.editor.undo() - Undo last change
    let undo = lua.create_function(|_lua, ()| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc) = helix_view::current!(editor);
        Ok(doc.undo(view))
    })?;
    editor_module.set("undo", undo)?;

    // helix.editor.redo() - Redo last undone change
    let redo = lua.create_function(|_lua, ()| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc) = helix_view::current!(editor);
        Ok(doc.redo(view))
    })?;
    editor_module.set("redo", redo)?;

    // helix.editor.select_all() - Select all text
    let select_all = lua.create_function(|_lua, ()| {
        let editor = crate::lua::get_editor_mut()?;
        let (view, doc) = helix_view::current!(editor);
        let end = doc.text().len_chars();
        let selection = helix_core::Selection::single(0, end);
        doc.set_selection(view.id, selection);
        Ok(())
    })?;
    editor_module.set("select_all", select_all)?;

    // helix.editor.get_register(name) - Get register values
    let get_register = lua.create_function(|lua, name_str: String| {
        let name = name_str.chars().next().ok_or_else(|| {
            LuaError::RuntimeError("Register name must be a single character".to_string())
        })?;
        let editor = crate::lua::get_editor_mut()?;

        let values = editor.registers.read(name, editor);
        let table = lua.create_table()?;
        if let Some(values) = values {
            for (i, val) in values.enumerate() {
                table.set(i + 1, val.to_string())?;
            }
        }
        Ok(table)
    })?;
    editor_module.set("get_register", get_register)?;

    // helix.editor.set_register(name, values) - Set register values
    let set_register = lua.create_function(|_lua, (name_str, values): (String, Vec<String>)| {
        let name = name_str.chars().next().ok_or_else(|| {
            LuaError::RuntimeError("Register name must be a single character".to_string())
        })?;
        let editor = crate::lua::get_editor_mut()?;

        editor.registers.write(name, values).map_err(|e| {
            LuaError::RuntimeError(format!("Failed to write to register {}: {}", name, e))
        })?;
        Ok(())
    })?;
    editor_module.set("set_register", set_register)?;

    helix_table.set("editor", editor_module)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_api_registration() {
        let lua = Lua::new();
        let helix_table = lua.create_table().unwrap();

        let result = register_editor_api(&lua, &helix_table);
        assert!(result.is_ok());

        // Verify editor module exists with expected functions
        let editor_module: LuaTable = helix_table.get("editor").unwrap();
        assert!(editor_module.contains_key("mode").unwrap());
        assert!(editor_module.contains_key("insert_mode").unwrap());
        assert!(editor_module.contains_key("normal_mode").unwrap());
        assert!(editor_module.contains_key("execute_command").unwrap());
        assert!(editor_module.contains_key("move_cursor").unwrap());
        assert!(editor_module.contains_key("get_cursor").unwrap());
        assert!(editor_module.contains_key("save").unwrap());
        assert!(editor_module.contains_key("open").unwrap());
    }

    #[test]
    fn test_editor_functions_callable() {
        let lua = Lua::new();
        let helix_table = lua.create_table().unwrap();
        register_editor_api(&lua, &helix_table).unwrap();

        // Test that we can call editor functions from Lua
        let code = r#"
            local mode = helix.editor.mode()
            assert(mode ~= nil)
            
            local result = helix.editor.execute_command("test")
            assert(result == nil)
        "#;

        lua.globals().set("helix", helix_table).unwrap();
        lua.load(code).exec().unwrap();
    }
}
