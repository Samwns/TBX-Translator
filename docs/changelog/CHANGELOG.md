# Changelog — TBX Translator

Todas as mudanças notáveis deste projeto são documentadas aqui.
Formato baseado em [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Não lançado] — 2026-08-14

### Adicionado

- Site oficial responsivo no GitHub Pages, com downloads diretos da release mais recente, documentação integrada e seleção de idioma.
- Atalhos para o site oficial e para as releases do GitHub na barra superior do aplicativo.
- Geração automática das notas da release e da mensagem do Discord, com publicação opcional por webhook.
- Atalhos compactos e separados para o Discord e o Ko-fi no canto inferior direito.
- Tela de novidades aberta automaticamente uma única vez depois de cada atualização, com confirmação persistente por versão.
- Ícones SVG para as ações da interface, eliminando quadrados causados por emojis e glifos ausentes na fonte.
- Cantos arredondados e sombra na janela principal, preservando a barra de título personalizada e o fundo transparente.
- Atualizador interno para Windows portátil/instalado, AppImage, Debian, Fedora e Arch, com changelog das Releases do GitHub, progresso, SHA-256 e reinício automático.
- Central de atualizações separada, acessível por um botão compacto com indicador de notificação ao lado da versão do aplicativo.
- Efeitos sonoros para interações da interface, conclusão, cancelamento, erro e nova atualização, com opção para desativá-los nas configurações.
- Suporte completo à extração e injeção de traduções em jogos Godot, incluindo catálogos nativos, PO multilinha, diálogos binários e scripts de história.
- Tradução em pacotes com concorrência global controlada, cache compartilhado e retentativas progressivas.
- Execuções independentes para Ren'Py, Unity e Godot, cada uma com progresso, cancelamento e logs próprios.
- Integração automática do idioma Ren'Py em menus dinâmicos ou listas estáticas, com seletor complementar para jogos sem menu de idiomas.

### Alterado

- README principal agora usa inglês por padrão e mantém uma versão completa em português brasileiro.
- README reorganizado, com downloads atuais, links da comunidade e versão dinâmica baseada na release mais recente.
- As releases agora exibem primeiro o resumo das mudanças e depois os downloads.
- Textos da interface em português foram refinados, incluindo botões de iniciar, voltar, salvar e navegar pelos logs.
- O dicionário de termos padrão passa a traduzir `Back` como `Voltar` e `Home`/`Play` como `Iniciar` em português.
- O ícone original de maleta foi restaurado e unificado na janela e nos pacotes Windows, AppImage e Linux nativo.
- A busca de atualizações saiu das Configurações; a verificação agora é automática e os controles ficam concentrados na nova tela.
- O Ren'Py não força mais `config.language` nem altera diretamente `_preferences.language`; a escolha passa a ser feita pelo jogador com a ação oficial `Language(...)`.
- Variáveis, interpolações e tags Ren'Py são removidas da carga enviada à API e recolocadas nas posições originais.
- Os 104 idiomas da interface foram completados com as mensagens da central de atualizações e dos efeitos sonoros.
- O número do build das GitHub Actions agora é incorporado ao executável.
- O seletor de engines agora usa pílulas independentes, maiores, espaçadas e animadas, sem alterar o fluxo de tradução existente.

### Corrigido

- Troca de idioma do site agora atualiza também os downloads dinâmicos e remove o estado anterior da tradução automática.
- Consulta da release mais recente no site passou a ignorar cache, mantendo versão e arquivos sincronizados com o GitHub.
- Scanner de fontes Ren'Py agora encontra fontes desempacotadas diretamente, evitando timeouts em diferentes versões do motor.
- Abas Ren'Py, Unity e Godot do injetor de fontes agora exibem seus respectivos ícones.
- Limpeza automática de diretórios temporários e scripts que ainda usavam o prefixo legado `tpg_`.
- Captura indevida de textos das ferramentas internas de desenvolvimento do Ren'Py.
- Preservação de BBCode, espaços e tags durante traduções Godot.
- Seleção de arquivos no Linux para pacotes AppImage e Debian.

---

## [2.1.0] — 2026-07-31

### Adicionado
- **Varredura recursiva de MonoBehaviours** (`ExtractStringsFromField`) — percorre toda a árvore de campos de cada componente Unity, em vez de apenas `m_Text` e `text`.
- **Blacklist de campos internos** da Unity (`m_Name`, `m_Script`, `m_GameObject`, etc.) para evitar captura de metadata.
- **Proteção de variáveis Yarn Spinner** (`{0}`, `{1}`, `{2}`) durante a tradução com placeholders `TBXVAR0`, `TBXVAR1`.
- **Proteção de rich text tags** (`<color=#xxx>`, `</color>`, `<size=xxx>`) durante a tradução com placeholders `TBXTAG0`, `TBXTAG1`.
- **Instalação local do BepInEx** via ZIPs bundled (pastas `BepInEx/` e `XUnity_AutoTranslator_bepInEx/`), sem download da internet.
- **Seleção automática** do ZIP correto (Mono vs IL2CPP, v5 vs v6) baseada no backend detectado.
- **Geração automática** do `AutoTranslatorConfig.ini` quando o jogo não pode ser executado diretamente (Linux).
- **Atualização automática** do `Language=` e `FromLanguage=` no config existente via regex.
- **Aviso para Linux/Proton**: instrução `WINEDLLOVERRIDES="winhttp=n,b"` exibida no console.

### Corrigido
- **Filtro `IsValidText()` reescrito** — parou de bloquear diálogos Yarn Spinner (`{0}`, `{1}`) e rich text (`<color=`, `<size=`).
- **Filtro de classe removido** — não exige mais que o MonoBehaviour tenha nome "Text" ou "Label" para ser varrido.
- **Crash `char boundary`** no editor (`editor_ui.rs:209`) — `find_unescaped_equals()` agora usa `char_indices()` em vez de `chars().enumerate()`.

### Removido
- Função `download_and_extract_zip()` — substituída por `extract_local_zip()` que usa ZIPs locais.

### Métricas de Melhoria (Jogo GOMI 0.4)
| Métrica | v2.0.0 | v2.1.0 |
|---------|--------|--------|
| Strings extraídas | 282 | 3.825 |
| Diálogos Yarn Spinner | 0 | 160 |
| Rich text strings | 0 | 19 |

---

## [2.0.0] — 2026-07-26

### Adicionado
- Interface nativa GTK4 substituindo a implementação Java/JavaFX.
- Backend em Rust puro com `reqwest` para tradução assíncrona via HTTP.
- Sistema de configuração persistente `AppConfig` usando JSON.
- Extrator Ren'Py com injeção de script Python no runtime.
- Extrator Unity com varredura binária de `.assets` e scanning de arquivos de texto.
- Formato de saída compatível com XUnity AutoTranslator (`Original=Tradução`).
- Editor de traduções para revisar e editar arquivos `.rpy` / `.txt` / `.json`.
- Suporte a tradução em lotes com multi-threading.
- Auto-detecção de engine (Ren'Py vs Unity).
- Tema dark GTK CSS no estilo Catppuccin Mocha.
- Injetor de fontes customizadas (`font_injector.rs`) com preview visual.
- Internacionalização da interface (`i18n.rs`).

### Alterado
- Migração completa de Java + JavaFX para Rust + GTK4.
- Migração de build Maven para Cargo.
- Removida dependência Tauri/WebView (causava UI quebrada no Fedora).
- Projeto renomeado de "TPG Translator" para **"TBX - Translator"**.

### Corrigido
- UI quebrada no Fedora Linux (removida dependência webkit2gtk/libsoup).

---

## [1.0.0] — Legacy (Java/JavaFX)

### Adicionado
- Release inicial em Java/JavaFX.
- Janela frameless transparente com drag customizado.
- Extração para engines Ren'Py e Unity.
- Integração com Google Translate API.
- Editor de traduções com TableView.
- Persistência automática de configurações.
- Transições animadas na UI (fade + scale).
