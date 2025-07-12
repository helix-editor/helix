use anyhow::Result;
use helix_view::editor::EditorEvent;
use tokio::sync::mpsc::UnboundedSender;

// Esta struct representa a API que o Helix expõe aos plugins.
// Ela será passada para os hosts WASM e Lua.
pub struct HelixApi {
    // Canal para enviar eventos de volta ao editor principal.
    editor_event_sender: UnboundedSender<EditorEvent>,
    plugin_idx: usize, // Identificador do plugin que possui esta API
    next_request_id: Cell<u32>,
}

impl HelixApi {
    pub fn new(editor_event_sender: UnboundedSender<EditorEvent>, plugin_idx: usize) -> Self {
        Self { editor_event_sender, plugin_idx, next_request_id: Cell::new(0) }
    }

    // Exemplo de função da API: exibir uma mensagem de status.
    pub fn show_message(&self, message: String) -> Result<()> {
        self.editor_event_sender.send(EditorEvent::PluginCommand(
            "show_message".to_string(),
            vec![message],
            None,
        ))?;
        Ok(())
    }

    // Registrar um comando de plugin.
    pub fn register_command(&self, command_name: String, callback_function_name: String) -> Result<()> {
        self.editor_event_sender.send(EditorEvent::RegisterPluginCommand(
            command_name,
            callback_function_name,
            self.plugin_idx,
        ))?;
        Ok(())
    }

    pub fn subscribe_to_event(&self, event_name: String, callback_function_name: String) -> Result<()> {
        self.editor_event_sender.send(EditorEvent::PluginCommand(
            "subscribe_to_event".to_string(),
            vec![event_name, callback_function_name, self.plugin_idx.to_string()],
            None,
        ))?;
        Ok(())
    }

    pub fn get_buffer_content(&self, doc_id: u32, request_id: u32) -> Result<()> {
        self.editor_event_sender.send(EditorEvent::PluginCommand(
            "get_buffer_content".to_string(),
            vec![doc_id.to_string()],
            Some(request_id),
        ))?;
        Ok(())
    }

    pub fn insert_text(&self, doc_id: u32, position: u32, text: String) -> Result<()> {
        self.editor_event_sender.send(EditorEvent::PluginCommand(
            "insert_text".to_string(),
            vec![doc_id.to_string(), position.to_string(), text],
            None,
        ))?;
        Ok(())
    }

    pub fn delete_text(&self, doc_id: u32, start: u32, end: u32) -> Result<()> {
        self.editor_event_sender.send(EditorEvent::PluginCommand(
            "delete_text".to_string(),
            vec![doc_id.to_string(), start.to_string(), end.to_string()],
            None,
        ))?;
        Ok(())
    }

}
