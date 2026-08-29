# TBX Locale Menu (Godot)

Autoload injetado pelo TBX Translator em exports Godot para adicionar uma
opção de idioma nativa na tela de configurações do jogo, sem overlay
permanente e sem tecla de atalho.

## Pipeline (resumo)

1. `bin_to_txt(project.binary)` — via `gdre_tools` (`--bin-to-txt`).
2. Editar o arquivo texto (`project.godot`):
   - Adicionar em `[autoload]`:
     ```
     TBXLocaleMenu="*res://tbx/tbx_locale_menu.gd"
     ```
   - Garantir `internationalization/locale/translations` incluindo o novo
     `.translation` (ex.: `res://locale.pt_BR.translation`).
3. `txt_to_bin` para gerar novo `project.binary`.
4. Compilar o script (`compile_gd("tbx_locale_menu.gd", "4.7.0")`) apenas se
   o export não aceitar `.gd` solto no PCK. Em 4.6+ o flare `.gd` no PCK é
   carregado; manter `.gd` é o default seguro.
5. `patch_embed` com os arquivos:
   - `res://tbx/tbx_locale_menu.gd` (conteúdo deste diretório)
   - `res://project.binary` (novo)
   - `res://locale.<alvo>.translation` (resultado de `compile_native_translation`)

## Comportamento em runtime

- Classe: `CanvasLayer`, `layer = 128`, `name = "TBXLocaleMenu"`.
- No `_ready`, lê `user://tbx_locale.cfg` e aplica o locale persistido
  via `TranslationServer.set_locale(...)` +
  `internationalization/locale/fallback`.
- Conecta `node_added` e monitora `current_scene` a cada frame; ao
  detectar um `Control` com nome batendo `(?i)settings|options|config|
  preferences`, injeta um `HBoxContainer` (`TBXLocaleRow`) + `OptionButton`
  (`TBXLocaleOption`) com `TranslationServer.get_loaded_locales()` e os
  idiomas adicionais embutidos (`pt_BR` por padrão via constante
  `TBX_LOCALES`).
- Persiste a escolha em `user://tbx_locale.cfg`.

## Convenções

- Nome dos nós injetados sempre prefixados com `TBX` — evita colisão.
- `set_meta("_tbx_locale_menu", true)` em ambos os nós permite detectar e
  não duplicar.
- Anexo preferencial a `VBoxContainer`/`HBoxContainer`/`GridContainer`
  internos para respeitar o layout existente; fallback em posição absoluta.
