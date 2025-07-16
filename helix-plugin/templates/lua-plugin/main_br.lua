-- my-plugin/main.lua

-- O objeto 'helix' é injetado no ambiente global do Lua
-- e fornece acesso à API do Helix.

-- Função chamada quando o plugin é carregado
function on_load()
    helix.show_message("Meu plugin Lua foi carregado!")
    helix.register_command("meu-plugin:lua-saudacao", "on_lua_saudacao_command")
    helix.subscribe_to_event("buffer_save", "on_lua_buffer_save_event")
end

-- Função de callback para o comando registrado
function on_lua_saudacao_command(...)
    local args = {...}
    local args_str = table.concat(args, ", ")
    helix.show_message("Olá do comando Lua! Argumentos: " .. args_str)
    helix.get_buffer_content(0, 456) -- Exemplo de solicitação de conteúdo do buffer
end

-- Função de callback para eventos do editor
function on_lua_buffer_save_event(event_data)
    helix.show_message("Evento buffer_save recebido: " .. event_data)
end

-- Função de callback para respostas de solicitações
function on_response(request_id, response_data)
    helix.show_message("Resposta para solicitação " .. request_id .. ": " .. response_data)
end
