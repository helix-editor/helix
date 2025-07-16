# Proposta de Design: Arquitetura de Plugins para o Helix Editor

Este documento descreve uma arquitetura para um sistema de plugins robusto, seguro e de alto desempenho para o editor Helix. A proposta visa permitir que a comunidade estenda as funcionalidades do editor, mantendo a estabilidade e a filosofia do projeto.

## 1. Filosofia e Objetivos

O sistema de plugins deve aderir aos mesmos princípios do Helix:

*   **Segurança:** O código do plugin nunca deve ser capaz de travar ou corromper o editor. Os plugins devem operar dentro de um *sandbox* estrito, com acesso controlado aos recursos do sistema e ao estado do editor.
*   **Desempenho:** O carregamento e a execução de plugins devem ter um impacto mínimo no tempo de inicialização e na latência de edição. O uso de runtimes JIT de alta performance é essencial.
*   **Ergonomia:** Criar plugins deve ser uma experiência agradável. A API deve ser bem documentada, idiomática e poderosa, com um ciclo de desenvolvimento rápido.
*   **Flexibilidade:** A arquitetura deve suportar tanto scripts simples para automação rápida quanto plugins complexos e de alto desempenho escritos em linguagens compiladas.

## 2. Arquitetura Proposta

A solução se baseia em um novo crate central, `helix-plugin`, que orquestra a descoberta, o carregamento e a execução de plugins. Propomos o suporte a dois tipos de plugins para cobrir diferentes casos de uso:

1.  **Plugins baseados em WebAssembly (WASM):** Para funcionalidades complexas e de alto desempenho. Permite que os desenvolvedores usem Rust, C++, Go, Zig, etc., compilando para `wasm32-wasi`. O runtime WASM fornece um sandbox de segurança de primeira classe.
2.  **Plugins baseados em Lua:** Para configurações, automações e prototipagem rápida. Lua é uma linguagem de script leve, rápida e fácil de embarcar em Rust, ideal para tarefas mais simples.

### Componentes Principais

#### a. Crate `helix-plugin`

Este novo crate será o coração do sistema.

*   **Responsabilidades:**
    *   Gerenciar o ciclo de vida completo dos plugins (descoberta, carregamento, recarregamento, descarregamento).
    *   Conter os runtimes para WASM (`wasmtime`) e Lua (`mlua`).
    *   Expor uma API de host segura e estável para os plugins.
    *   Atuar como uma ponte entre o núcleo do Helix e os plugins.

#### b. Manifesto do Plugin (`plugin.toml`)

Cada plugin deve incluir um arquivo de manifesto para metadados e configuração.

*   **Localização:** `~/.config/helix/plugins/meu-plugin/plugin.toml`
*   **Estrutura de Exemplo:**
    ```toml
    # Metadados básicos do plugin.
    name = "meu-plugin-incrivel"
    version = "0.1.0"
    authors = ["Seu Nome <seu@email.com>"]
    description = "Um plugin que faz algo incrível."

    # O ponto de entrada para o código do plugin.
    # O sistema determinará o tipo de plugin pela extensão.
    entrypoint = "main.wasm" # ou "main.lua"

    # (Opcional) Define quando o plugin deve ser carregado para economizar recursos.
    # Se omitido, o plugin é carregado na inicialização.
    [activation]
    on_command = ["meu-plugin:minha-acao"] # Carrega ao chamar um comando específico.
    on_language = ["rust", "toml"]        # Carrega quando um arquivo de uma linguagem é aberto.
    on_event = ["buffer_save"]            # Carrega em eventos específicos do editor.
    ```

#### c. A API do Plugin (`helix::api`)

Esta é a superfície de contato entre um plugin e o editor. Será uma fachada segura sobre os crates internos do Helix (`helix-core`, `helix-view`, etc.), garantindo que nenhum plugin possa acessar o estado interno de forma perigosa.

*   **Módulos da API (Exemplos):**
    *   `helix::api::editor`: Funções para interagir com buffers, seleções e o estado geral.
        *   `get_buffer_content(buf_id) -> Result<String, Error>`
        *   `get_selections(view_id) -> Result<Vec<Range>, Error>`
        *   `apply_transaction(view_id, transaction)`
    *   `helix::api::commands`: Para registrar novos comandos no editor.
        *   `register(name, callback)`
    *   `helix::api::events`: Para reagir a eventos do editor.
        *   `subscribe(event_name, callback)`
    *   `helix::api::ui`: Para interagir com a interface do usuário do Helix.
        *   `show_picker(items, callback)`
        *   `show_message(level, text)`

## 3. Requisitos de Implementação

#### a. Novo Crate: `helix-plugin`

*   **Estrutura de Módulos:**
    *   `lib.rs`: Ponto de entrada do crate.
    *   `manager.rs`: `PluginManager`, responsável pelo ciclo de vida.
    *   `api.rs`: Definição da API pública do host.
    *   `host/mod.rs`, `host/wasm.rs`, `host/lua.rs`: Implementações dos runtimes WASM e Lua.
    *   `config.rs`: Lógica para ler e interpretar `plugin.toml`.

#### b. Dependências do Cargo (Cargo.toml)

*   **Runtime WASM:**
    *   `wasmtime`: Runtime JIT para `wasm32-wasi`.
*   **Runtime Lua:**
    *   `mlua`: Bindings seguros de alto nível para Lua.
*   **Utilitários:**
    *   `serde` & `serde_json`: Para serialização de dados entre host e plugins.
    *   `toml`: Para analisar os manifestos `plugin.toml`.
    *   `walkdir`: Para a descoberta eficiente de plugins no sistema de arquivos.
    *   `anyhow`: Para um tratamento de erros mais limpo e ergonômico.

#### c. Modificações no Código Existente

*   **`Cargo.toml` (Workspace):**
    *   Adicionar `helix-plugin` à lista de `members`.

*   **`helix-term` (Binário Principal):**
    *   **Inicialização:** Instanciar e inicializar o `PluginManager` no `main.rs`.
    *   **Event Loop:** Integrar o `PluginManager` ao loop de eventos principal para despachar eventos (teclas, comandos, etc.) para os plugins.

*   **`helix-event`:**
    *   Permitir que o `PluginManager` se registre como um "ouvinte" de eventos globais do editor.

*   **`helix-core` e `helix-view`:**
    *   **Exposição da API (Fachada Segura):** Esta é a parte mais crítica. Em vez de tornar as funções internas `pub`, criaremos funções de fachada na `helix::api` que realizam validações e expõem apenas a funcionalidade necessária. Isso impede que um plugin malformado ou mal-intencionado cause um `panic` ou corrompa o estado do editor.

*   **Dispatcher de Comandos (`helix-view/src/commands.rs`):**
    *   Modificar o dispatcher para que, se um comando não for encontrado internamente, ele consulte o `PluginManager` para verificar se um plugin registrou o comando.

## 4. Roteiro de Implementação Sugerido

1.  **Fase 1: Fundação e WASM Básico**
    *   Criar o crate `helix-plugin` e adicionar as dependências (`wasmtime`, `toml`, `walkdir`).
    *   Implementar a descoberta de plugins e a análise do `plugin.toml`.
    *   Implementar um host WASM básico que possa carregar e executar um arquivo `.wasm`.
    *   Definir uma API mínima: `helix::api::commands::register` e `helix::api::ui::show_message`.
    *   Criar um plugin "hello world" em Rust (compilado para WASM) para teste.

2.  **Fase 2: Integração com o Core**
    *   Integrar o `PluginManager` no `helix-term` e no loop de eventos.
    *   Modificar o dispatcher de comandos para chamar os comandos dos plugins.
    *   Expandir a API com acesso de leitura ao estado do editor (ex: `get_buffer_content`).

3.  **Fase 3: Host Lua e Expansão da API**
    *   Adicionar a dependência `mlua` e implementar o `LuaHost`.
    *   Expor a mesma `helix::api` para o ambiente Lua.
    *   Expandir a API com funcionalidades de escrita/modificação (ex: `apply_transaction`), garantindo que todas as operações sejam seguras e reversíveis (undo/redo).
    *   Criar um plugin de exemplo em Lua.

4.  **Fase 4: Documentação e Ecossistema**
    *   Escrever a documentação para desenvolvedores de plugins no `book/` do Helix.
    *   Documentar detalhadamente toda a `helix::api`.
    *   Criar templates de projeto para plugins em Rust e Lua.
    *   Desenvolver alguns plugins úteis e incluí-los no diretório `runtime/plugins`.

## 5. Experiência do Desenvolvedor de Plugins

#### Exemplo: Plugin "Hello World" (Rust/WASM)

```rust
// No plugin (lib.rs)
use helix::api;

// A macro `helix::plugin` cuidaria do boilerplate de exportação do WASM.
#[helix::plugin]
fn on_load() {
    api::commands::register("hello-plugin", |args| {
        api::ui::show_message(api::ui::Level::Info, "Olá, do meu plugin!");
    });
}
```

#### Exemplo: Plugin "Hello World" (Lua)

```lua
-- No plugin (main.lua)
local editor = helix.api.editor
local ui = helix.api.ui

helix.api.commands.register("hello-lua", function(args)
    ui.show_message("info", "Olá, do meu plugin Lua!")
end)
```

Esta arquitetura fornece uma base sólida para um ecossistema de plugins próspero, capacitando os usuários a adaptar o Helix às suas necessidades, mantendo os padrões de qualidade e desempenho do projeto.
