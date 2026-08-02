pub fn t<'a>(key: &'a str, lang: &str) -> &'a str {
    let is_en = lang == "en_US";

    match key {
        // Toolbar
        "aba_traduzir" => if is_en { "Translate Game" } else { "Traduzir Jogo" },
        "aba_logs" => if is_en { "Execution Logs" } else { "Logs de Execução" },
        "aba_config" => if is_en { "Settings" } else { "Configurações" },
        "aba_tools" => if is_en { "Additional Tools" } else { "Ferramentas Adicionais" },

        // Main screen
        "selecione_pasta" => if is_en { "Select Game Folder (Ren'Py / Unity)..." } else { "Selecione a Pasta do Jogo (Ren'Py / Unity)..." },
        "iniciar_trad_renpy" => if is_en { "START REN'PY TRANSLATION" } else { "INICIAR TRADUÇÃO REN'PY" },
        "iniciar_trad_unity" => if is_en { "SETUP UNITY TRANSLATION" } else { "CONFIGURAR TRADUÇÃO UNITY" },
        "abrir_editor" => if is_en { "OPEN TEXT EDITOR" } else { "ABRIR EDITOR DE TEXTOS" },
        
        // Logs
        "logs_vazio" => if is_en { "Awaiting execution..." } else { "Aguardando execução..." },
        "log_progresso" => if is_en { "Wait, translation is running in background..." } else { "Aguarde, tradução rodando em segundo plano..." },

        // Settings
        "config_geral" => if is_en { "General Settings" } else { "Configurações Gerais" },
        "idioma_app" => if is_en { "App Interface Language (Requires Restart)" } else { "Idioma do Aplicativo (Requer Reiniciar)" },
        "pasta_trad" => if is_en { "Translation Folder Name (e.g., portuguese)" } else { "Nome da Pasta de Tradução (ex: portuguese)" },
        "idioma_orig" => if is_en { "Source Language" } else { "Idioma Original" },
        "idioma_alvo" => if is_en { "Target Language" } else { "Idioma Alvo" },
        "config_renpy" => if is_en { "Ren'Py Extra Settings" } else { "Configurações Extras Ren'Py" },
        "ativar_multi" => if is_en { "Enable Multi-Threaded Translation" } else { "Ativar Tradução Simultânea (Multi-Thread)" },
        "qtd_threads" => if is_en { "Process Count:" } else { "Qtd. Processos:" },
        "manter_estrtura" => if is_en { "Keep Original Structure (Separate files in tl folder)" } else { "Manter Estrutura Original (Separar arquivos na pasta tl)" },
        "proteger_var" => if is_en { "Ren'Py: protect bracket variables, e.g. [name] (always on)" } else { "Ren'Py: proteger variáveis entre colchetes, ex: [name] (sempre ativo)" },
        "trad_nomes" => if is_en { "Ren'Py: translate character names" } else { "Ren'Py: traduzir nomes dos personagens" },
        "aviso_ip" => if is_en { "Warning: Too many threads on Native API might cause IP blocks." } else { "Aviso: Muitas threads simultâneas na API Nativa podem causar bloqueio de IP." },
        "salvar_config" => if is_en { "SAVE SETTINGS" } else { "SALVAR CONFIGURAÇÕES" },

        // Tools
        "ferramentas_desc" => if is_en { "Extra options to improve translation and game compatibility." } else { "Opções extras para melhorar a tradução e compatibilidade dos jogos." },
        "btn_font" => if is_en { "Replace Game Font (Fixes missing characters)" } else { "Substituir Fonte do Jogo (Corrige acentos invisíveis)" },
        "erro_sem_pasta" => if is_en { "Please select the game folder on the main screen first." } else { "Por favor, selecione a pasta do jogo na tela principal primeiro." },

        // Font Injector
        "janela_fonte_titulo" => if is_en { "Font Injector" } else { "Injetor de Fontes" },
        "fonte_info" => if is_en { "Select a font (.ttf, .otf) to replace the game's default font." } else { "Selecione uma fonte (.ttf, .otf) para substituir a fonte padrão do jogo." },
        "caminho_fonte" => if is_en { "Font path..." } else { "Caminho da fonte..." },
        "procurar" => if is_en { "Browse..." } else { "Procurar..." },
        "teste_fonte" => if is_en { "Test your font (Type to test):" } else { "Teste sua fonte (Digite para testar):" },
        "injetar_fonte" => if is_en { "INJECT FONT INTO GAME" } else { "INJETAR FONTE NO JOGO" },
        "selecione_fonte" => if is_en { "Select Font" } else { "Selecione a Fonte" },
        "abrir" => if is_en { "Open" } else { "Abrir" },
        "cancelar" => if is_en { "Cancel" } else { "Cancelar" },
        "fonte_sucesso" => if is_en { "Font injected successfully!" } else { "Fonte injetada com sucesso!" },

        _ => key, // fallback to key name
    }
}
