### Relatório Completo: Implementação do Módulo de Plugins/Scripts para o Helix Editor

**Data:** 11 de julho de 2025

**Objetivo:** Desenvolver um sistema de plugins extensível para o Helix editor, permitindo que a comunidade adicione novas funcionalidades e personalize o editor através de plugins baseados em WebAssembly (WASM) e Lua.

---

#### **1. Fase 1: Fundação e Estrutura Básica**

**Propósito:** Estabelecer a base do sistema de plugins, incluindo a criação do novo crate, a adição de dependências essenciais e a definição das estruturas de dados para descoberta e carregamento de plugins.

**Alterações e Progresso:**

*   **Criação do Crate `helix-plugin`:**
    *   Um novo crate de biblioteca Rust, `helix-plugin`, foi criado no workspace do projeto Helix.
    *   **Comando:** `cargo new --lib helix-plugin`
    *   **Impacto:** Adicionou `helix-plugin/Cargo.toml` e `helix-plugin/src/lib.rs`.
*   **Adição de Dependências Iniciais:**
    *   As dependências `wasmtime`, `toml`, `walkdir`, `serde`, `anyhow` e `mlua` (com a feature `lua54`) foram adicionadas ao `helix-plugin/Cargo.toml`.
    *   **Comando:** `cargo add -p helix-plugin wasmtime toml walkdir serde anyhow mlua --features "lua54"`
    *   **Impacto:** Configuração do ambiente de desenvolvimento para os hosts WASM e Lua, além de utilitários para parsing e manipulação de arquivos.
*   **Estrutura de Módulos:**
    *   Diretórios `helix-plugin/src/host` e `helix-plugin/tests` foram criados.
    *   Arquivos `helix-plugin/src/manager.rs`, `helix-plugin/src/config.rs`, `helix-plugin/src/host/wasm.rs`, `helix-plugin/src/host/lua.rs` e `helix-plugin/src/api.rs` foram criados.
    *   Os módulos foram declarados em `helix-plugin/src/lib.rs` e `helix-plugin/src/host/mod.rs` para organização do código.
*   **Definição do Manifesto do Plugin (`plugin.toml`):**
    *   Em `helix-plugin/src/config.rs`, as structs `PluginManifest` e `Activation` foram definidas usando `serde` para desserialização de arquivos TOML. Isso permite que os plugins declarem seus metadados e pontos de entrada.
*   **Lógica de Descoberta de Plugins:**
    *   Em `helix-plugin/src/manager.rs`, a função `discover_plugins_in` foi implementada. Ela utiliza `walkdir` para escanear um diretório (`~/.config/helix/plugins/`) em busca de arquivos `plugin.toml`, lê-os e parseia seus conteúdos em `PluginManifest`s.
*   **Hosts Iniciais (WASM e Lua):**
    *   Em `helix-plugin/src/host/wasm.rs`, a struct `WasmHost` foi criada com a capacidade de carregar um arquivo `.wasm` usando `wasmtime`.
    *   Em `helix-plugin/src/host/lua.rs`, a struct `LuaHost` foi criada com a capacidade de carregar um arquivo `.lua` usando `mlua`.
*   **Testes de Descoberta:**
    *   Um diretório de teste `tests/test-plugins/my-first-plugin` foi criado com um `plugin.toml` de exemplo.
    *   Um teste de integração (`helix-plugin/tests/discovery.rs`) foi adicionado e executado com sucesso, confirmando que o `PluginManager` pode descobrir e analisar manifestos de plugins.

---

#### **2. Fase 2: Integração com o Core do Editor**

**Propósito:** Conectar o sistema de plugins ao ciclo de vida principal do Helix, permitindo que o editor inicialize o gerenciador de plugins e que os comandos de plugins sejam despachados.

**Alterações e Progresso:**

*   **Inicialização do `PluginManager` no `helix-term`:**
    *   Em `helix-term/src/main.rs`, o `PluginManager` agora é instanciado no `main_impl` antes da criação da `Application`.
    *   Um canal MPSC (`tokio::sync::mpsc::unbounded_channel`) é criado, e o `sender` é passado para o `PluginManager::new`. O `receiver` é posteriormente atribuído ao `editor.editor_events.1`.
*   **Integração na Struct `Application`:**
    *   Em `helix-term/src/application.rs`, o campo `plugin_manager: helix_plugin::manager::PluginManager` foi adicionado à struct `Application`.
    *   O construtor `Application::new` foi modificado para aceitar o `plugin_manager` como um argumento e armazená-lo.
*   **Sistema de Eventos para Comunicação com Plugins:**
    *   Em `helix-view/src/editor.rs`, o `enum EditorEvent` foi estendido com duas novas variantes:
        *   `EditorEvent::PluginCommand(String, Vec<String>, Option<u32>)`: Para despachar comandos de plugin do `helix-term` para o `Application`, incluindo um `request_id` opcional para respostas.
        *   `EditorEvent::RegisterPluginCommand(String, String, usize)`: Para permitir que plugins registrem comandos no editor, incluindo o nome do comando, o nome da função de callback e o índice do plugin.
        *   `EditorEvent::PluginResponse(u32, String)`: Para enviar respostas de volta aos plugins.
        *   `EditorEvent::PluginEvent(String, String)`: Para despachar eventos do editor para os plugins.
    *   Um novo canal MPSC (`editor_events: (UnboundedSender<EditorEvent>, UnboundedReceiver<EditorEvent>)`) foi adicionado à struct `Editor` para gerenciar a comunicação de eventos.
    *   Um método `dispatch_editor_event(&mut self, event: EditorEvent)` foi adicionado ao `Editor` para enviar eventos através deste canal.
    *   O `tokio::select!` no `Editor::wait_event` foi atualizado para escutar o novo canal `editor_events`.
*   **Mecanismo de Despacho de Comandos de Plugin:**
    *   Em `helix-term/src/commands.rs`, o `enum MappableCommand` foi estendido com a variante `Plugin { name: String, args: Vec<String> }`.
    *   O método `MappableCommand::from_str` foi modificado para que, se um comando não for encontrado entre os comandos estáticos ou typable, ele seja interpretado como um `MappableCommand::Plugin`.
    *   Os métodos `MappableCommand::name()` e `MappableCommand::doc()` foram atualizados para lidar com a nova variante `Plugin`.
    *   O método `MappableCommand::execute()` foi modificado para despachar um `EditorEvent::PluginCommand` (usando `cx.editor.dispatch_editor_event`) quando um `MappableCommand::Plugin` é executado.
*   **Execução de Comandos no `PluginManager`:**
    *   Em `helix-plugin/src/manager.rs`, um método `execute_command(&mut self, name: &str, args: &[String])` foi adicionado. Este método é responsável por encontrar o plugin que registrou o comando e chamar a função de callback apropriada no host do plugin, passando os argumentos.

---

#### **3. Fase 3: Expansão da API e Integração Completa dos Hosts**

**Propósito:** Aprimorar a API de comunicação entre o editor e os plugins, permitindo que os plugins registrem comandos dinamicamente e interajam com o editor de forma mais rica.

**Alterações e Progresso:**

*   **API de Plugins (`helix-plugin/src/api.rs`):**
    *   A struct `HelixApi` foi aprimorada. Agora, seu construtor `new` recebe o `UnboundedSender<EditorEvent>` e um `plugin_idx` (identificador do plugin).
    *   A função `show_message(message: String)` foi implementada para enviar um `EditorEvent::PluginCommand` ao editor.
    *   A função `register_command(command_name: String, callback_function_name: String)` foi implementada para enviar um `EditorEvent::RegisterPluginCommand` ao editor, incluindo o `plugin_idx` para identificar o plugin de origem.
    *   A função `get_buffer_content(doc_id: u32, request_id: u32)` foi adicionada, enviando um `EditorEvent::PluginCommand` com um `request_id` para o editor.
    *   Novas funções `insert_text(doc_id: u32, position: u32, text: String)` e `delete_text(doc_id: u32, start: u32, end: u32)` foram adicionadas, enviando `EditorEvent::PluginCommand`s para o editor.
    *   A função `subscribe_to_event(event_name: String, callback_function_name: String)` foi adicionada, enviando um `EditorEvent::PluginCommand` para o editor.
*   **Integração da API no `WasmHost` (`helix-plugin/src/host/wasm.rs`):**
    *   O construtor `WasmHost::new` agora recebe uma instância de `HelixApi`.
    *   A `HelixApi` é armazenada como estado (`Store<HelixApi>`) no `wasmtime::Store`.
    *   As funções `show_message`, `register_command`, `get_buffer_content`, `insert_text`, `delete_text` e `subscribe_to_event` da `HelixApi` são expostas aos plugins WASM através de `linker.func_wrap`, permitindo que os plugins WASM chamem essas funções do host.
    *   A função `call_function(&mut self, name: &str, args: &[String])` foi atualizada para passar argumentos para as funções WASM, incluindo alocação e desalocação de memória WASM para strings.
*   **Integração da API no `LuaHost` (`helix-plugin/src/host/lua.rs`):
    *   O construtor `LuaHost::new` agora recebe uma instância de `HelixApi`.
    *   A `HelixApi` é registrada como um `UserData` e exposta como um objeto global `helix` no ambiente Lua. Isso permite que scripts Lua chamem `helix.show_message()`, `helix.register_command()`, `helix.get_buffer_content()`, `helix.insert_text()`, `helix.delete_text()` e `helix.subscribe_to_event()`.
    *   O método `call_function(&mut self, name: &str, args: &[String])` foi atualizado para passar argumentos para as funções Lua.
*   **Gerenciamento de Hosts e Comandos no `PluginManager` (`helix-plugin/src/manager.rs`):**
    *   Um `enum PluginHost` foi introduzido para encapsular `WasmHost` e `LuaHost`, permitindo o gerenciamento genérico de diferentes tipos de hosts.
    *   A struct `LoadedPlugin` agora armazena o `PluginHost` genérico e o `PluginManifest`.
    *   A função `discover_plugins_in` foi atualizada para:
        *   Criar uma `HelixApi` para cada plugin, associando-a ao `plugin_idx` correto.
        *   Instanciar o `WasmHost` ou `LuaHost` apropriado, passando a `HelixApi`.
        *   Chamar a função `on_load` nos plugins (WASM e Lua) após o carregamento, se exportada.
        *   Um `HashMap<String, (String, usize)>` (`registered_commands`) foi adicionado à struct `PluginManager` para mapear nomes de comandos para o nome da função de callback e o índice do plugin carregado.
        *   Um `next_request_id` e `pending_requests` (`HashMap`) foram adicionados para gerenciar solicitações assíncronas e suas respostas.
        *   Um exemplo de registro de comando (`my-plugin:test-command`) foi adicionado para fins de teste.
    *   A função `execute_command` foi aprimorada para procurar o comando no `registered_commands` e, se encontrado, chamar a função de callback correspondente no `PluginHost` apropriado, passando os argumentos.
    *   Um novo método `register_command(&mut self, command_name: String, callback_function_name: String, plugin_idx: usize)` foi adicionado ao `PluginManager` para registrar comandos dinamicamente.
    *   Um novo método `handle_plugin_response(&mut self, request_id: u32, response_data: String)` foi adicionado para processar respostas de plugins.
    *   Métodos `get_next_request_id` e `add_pending_request` foram adicionados para gerenciar o fluxo de solicitações/respostas.
*   **Tratamento de `RegisterPluginCommand` e `PluginResponse` no `Application`:**
    *   Em `helix-term/src/application.rs`, o `handle_editor_event` foi atualizado para processar `EditorEvent::RegisterPluginCommand` e `EditorEvent::PluginResponse`, chamando os métodos apropriados no `PluginManager`.
    *   O `Application` agora tem um `next_request_id` para gerar IDs de solicitação.

---

#### **4. Fase 4: Documentação e Ecossistema**

**Propósito:** Fornecer recursos para desenvolvedores de plugins e organizar o ecossistema de plugins.

**Alterações e Progresso:**

*   **Documentação de Plugins:**
    *   O arquivo `book/src/plugins.md` foi atualizado para refletir as novas funções da API (`insert_text`, `delete_text`, `subscribe_to_event`, `get_buffer_content` com `request_id`) e a mecânica de `request_id`/`PluginResponse`.
    *   Os exemplos de plugins em Rust (WASM) e Lua foram atualizados para demonstrar a passagem de argumentos e o uso das novas funções da API.
*   **Templates de Projeto para Plugins:**
    *   Os templates em `helix-plugin/templates/rust-wasm-plugin` e `helix-plugin/templates/lua-plugin` foram atualizados para incluir exemplos de uso das novas funções da API e a estrutura para lidar com argumentos e respostas.

---

#### **5. Sistema Básico de Eventos (Assinatura/Despacho)**

**Propósito:** Implementar um mecanismo para que os plugins possam assinar e reagir a eventos específicos do editor.

**Alterações e Progresso:**

*   **`EditorEvent::PluginEvent`:** Adicionado ao `helix-view/src/editor.rs` para despachar eventos do editor para os plugins.
*   **`HelixApi::subscribe_to_event`:** Adicionado ao `helix-plugin/src/api.rs` para permitir que plugins assinem eventos.
*   **Integração nos Hosts:** `WasmHost` e `LuaHost` foram atualizados para expor `subscribe_to_event`.
*   **`Application::event_subscribers`:** Adicionado um `HashMap` em `helix-term/src/application.rs` para gerenciar os plugins inscritos em eventos.
*   **`Application::handle_editor_event`:** Atualizado para despachar `PluginEvent` para os plugins inscritos.

---

#### **6. Tratamento de Erros e Relatórios (Robustez)**

**Propósito:** Aprimorar o tratamento de erros e o feedback ao usuário para falhas de plugins.

**Alterações e Progresso:**

*   **Logs Detalhados:** `log::error!` e `log::warn!` são usados extensivamente para reportar falhas de carregamento, execução e comunicação de plugins.
*   **Captura de Erros de Host:** Erros de `wasmtime` e `mlua` são capturados e convertidos em `anyhow::Error` para um tratamento consistente.

---