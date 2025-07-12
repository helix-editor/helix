### Lista de Tarefas para a Primeira Versão Robusta do Módulo de Plugins

Esta lista prioriza a funcionalidade essencial e a robustez para uma versão inicial, deixando funcionalidades mais avançadas e ferramentas de desenvolvimento para iterações futuras.

#### **1. Ambiente de Compilação WASM (Crítico)**

*   **Tarefa:** Garantir que o ambiente de desenvolvimento do Helix possa compilar plugins Rust para o target `wasm32-wasi`.
    *   **Detalhes:** Isso envolve a instalação da toolchain `wasm32-wasi` do Rust (`rustup target add wasm32-wasi`) e a verificação de que o processo de build do Helix pode compilar o plugin de teste WASM.
    *   **Justificativa:** Atualmente, este é um bloqueador para testar e validar a parte WASM da implementação. Sem isso, a funcionalidade WASM é teórica.

#### **2. Comunicação Bidirecional Robusta (Retorno de Valores)**

*   **Tarefa:** Implementar um mecanismo robusto para que as funções do host (Helix) possam retornar valores para os plugins (WASM e Lua).
    *   **Detalhes (WASM):**
        *   Definir um protocolo de serialização/desserialização (e.g., JSON ou bincode) para dados complexos (strings, structs, etc.) na memória compartilhada entre o host e o plugin WASM.
        *   Implementar funções no `WasmHost` para ler dados retornados pelo plugin WASM.
        *   Modificar a `HelixApi::get_buffer_content` para realmente buscar e retornar o conteúdo do buffer para o plugin WASM.
    *   **Detalhes (Lua):**
        *   Garantir que `LuaHost` possa receber valores de retorno de funções Lua chamadas pelo host.
        *   Modificar a `HelixApi::get_buffer_content` para realmente buscar e retornar o conteúdo do buffer para o plugin Lua.
    *   **Justificativa:** Essencial para qualquer interação significativa onde o plugin precisa de dados do editor ou precisa retornar resultados de suas operações.

#### **3. Expansão da API Essencial (Leitura e Escrita)**

*   **Tarefa:** Expandir a `HelixApi` para expor as funcionalidades mais básicas e cruciais do editor que os plugins precisarão para serem úteis.
    *   **Detalhes:**
        *   **Leitura:** Métodos para obter informações sobre o estado atual do editor (e.g., `get_current_buffer_id()`, `get_selection_ranges(doc_id)`).
        *   **Escrita/Modificação:** Métodos para realizar operações básicas de edição (e.g., `insert_text(doc_id, position, text)`, `delete_text(doc_id, range)`, `set_selection(doc_id, selection)`).
        *   **UI Básica:** Métodos para exibir prompts simples ou interagir com pickers existentes (se aplicável e seguro).
    *   **Justificativa:** Sem uma API rica o suficiente, os plugins terão funcionalidade muito limitada.

#### **4. Gerenciamento Dinâmico de Comandos (Conclusão)**

*   **Tarefa:** Finalizar a implementação do registro e execução dinâmica de comandos.
    *   **Detalhes:**
        *   Garantir que o `PluginManager` possa gerenciar múltiplos plugins registrando comandos com o mesmo nome (e.g., usando namespaces ou prioridades).
        *   Implementar a capacidade de plugins desregistrarem comandos (se necessário).
        *   Refinar a passagem de argumentos para comandos de plugin, garantindo que os argumentos passados pelo usuário na linha de comando sejam corretamente parseados e entregues ao plugin.
    *   **Justificativa:** Permite que os plugins estendam o conjunto de comandos do editor de forma flexível.

#### **5. Sistema Básico de Eventos (Assinatura/Despacho)**

*   **Tarefa:** Implementar um mecanismo para que os plugins possam assinar e reagir a eventos específicos do editor.
    *   **Detalhes:**
        *   Definir um conjunto inicial de eventos do editor (e.g., `on_buffer_save`, `on_buffer_open`, `on_mode_change`).
        *   Implementar a lógica no `PluginManager` e nos hosts (WASM/Lua) para despachar esses eventos para os plugins que os assinaram.
    *   **Justificativa:** Muitos plugins precisam reagir a mudanças no estado do editor para funcionar (e.g., um plugin de formatação automática ao salvar, um plugin de linter ao mudar o buffer).

#### **6. Tratamento de Erros e Relatórios (Robustez)**

*   **Tarefa:** Aprimorar o tratamento de erros e o feedback ao usuário para falhas de plugins.
    *   **Detalhes:**
        *   Capturar e reportar erros de execução de plugins (WASM traps, erros de Lua) de forma que não travem o editor, mas informem o usuário.
        *   Fornecer mensagens de erro claras e úteis sobre falhas de carregamento ou execução de plugins.
    *   **Justificativa:** Essencial para a robustez e a experiência do usuário, evitando que plugins mal-comportados causem instabilidade no editor.
