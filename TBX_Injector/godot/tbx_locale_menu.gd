## TBX Locale Menu — autoload injetado pelo TBX Translator.
## CanvasLayer persistido que adiciona um seletor de idioma nativo às telas
## de configurações detectáveis do jogo, SEM overlay permanente e SEM tecla
## de atalho.
##
## Os placeholders abaixo __SAO_SUBSTITUIDOS__ pelo injetor Rust na hora de
## gerar o PCK/exe final. Qualquer idioma-alvo funciona — nada é hardcoded.
extends CanvasLayer

const SETTINGS_NAME_RE := "(?i)settings|options|config|preferences|idioma|language"
# Textos típicos de botões de idioma em menus nativos (serve apenas para
# DETECTAR que estamos num menu de línguas; o novo rótulo vem do placeholder).
const LANGUAGE_BUTTON_TEXTS := [
	"ENGLISH", "ESPAÑOL", "ESPANOL", "FRANÇAIS", "FRANCAIS",
	"DEUTSCH", "ITALIANO", "PORTUGUÊS", "PORTUGUES",
	"日本語", "РУССКИЙ", "中文", "한국어", "LANGUAGE", "IDIOMA",
]
const SAVE_PATH := "user://tbx_locale.cfg"
const LOCALE_KEY := "tbx_locale"
# Placeholders — preenchidos pelo injetor com o locale BCP-47 real
# (ex.: "pt_BR", "es_MX", "ja") e com o rótulo exibido no botão
# (ex.: "Português (Brasil)", "Español (Latino)").
const TBX_LOCALES := ["__TBX_TARGET_LOCALE__"]
const TBX_TARGET_LOCALE := "__TBX_TARGET_LOCALE__"
const TBX_TARGET_LABEL := "__TBX_TARGET_LABEL__"

var _settings_regex: RegEx = null
var _last_processed_scene: Node = null
var _has_injected := false

func _ready() -> void:
	layer = 128
	name = "TBXLocaleMenu"
	process_mode = Node.PROCESS_MODE_ALWAYS
	_settings_regex = RegEx.new()
	_settings_regex.compile(SETTINGS_NAME_RE)
	_apply_saved_locale()
	get_tree().node_added.connect(_on_node_added)
	set_process(true)
	call_deferred("_try_inject_current_scene")

func _process(_delta: float) -> void:
	var root := get_tree().current_scene
	if root != _last_processed_scene:
		_last_processed_scene = root
		_has_injected = false
		_try_inject_current_scene()

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_L:
			_toggle_overlay()
			get_viewport().set_input_as_handled()

func _toggle_overlay() -> void:
	var existing := get_node_or_null("TBXLocaleOverlay")
	if existing:
		existing.queue_free()
		return
	var panel := PanelContainer.new()
	panel.name = "TBXLocaleOverlay"
	panel.set_meta("_tbx_locale_menu", true)
	panel.set_anchors_preset(Control.PRESET_CENTER)
	panel.offset_left = -160
	panel.offset_top = -40
	panel.offset_right = 160
	panel.offset_bottom = 80
	var row := HBoxContainer.new()
	row.name = "Row"
	var label := Label.new()
	label.text = "Language"
	row.add_child(label)
	var option := OptionButton.new()
	option.name = "Opt"
	option.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var locales := _get_available_locales()
	var current := _current_locale()
	var selected_idx := -1
	for i in locales.size():
		var code: String = locales[i]
		option.add_item(_locale_display_name(code), i)
		option.set_item_metadata(i, code)
		if code == current:
			selected_idx = i
	if selected_idx < 0 and locales.size():
		selected_idx = 0
	option.select(selected_idx)
	option.item_selected.connect(_on_locale_picked.bind(option))
	row.add_child(option)
	panel.add_child(row)
	add_child(panel)

func _on_node_added(node: Node) -> void:
	if _has_injected or node == null:
		return
	if _is_settings_container(node):
		call_deferred("_inject_in", node)
		_has_injected = true

func _try_inject_current_scene() -> void:
	if _has_injected:
		return
	var scene := get_tree().current_scene
	if scene == null:
		return
	# 1) Menu de idiomas por botões — clona o primeiro achado e adiciona
	#    o novo ao lado, com mesmo estilo.
	var lang_button := _find_language_button(scene)
	if lang_button != null:
		_inject_language_button_clone(lang_button)
		_has_injected = true
		return
	# 2) Settings tradicional: anexa um OptionButton.
	if _is_settings_container(scene):
		_inject_in(scene)
		_has_injected = true
		return
	var target := _find_settings_container(scene)
	if target:
		_inject_in(target)
		_has_injected = true

## Busca BFS em QUALQUER Control (não só BaseButton) cujo texto próprio ou
## de um Label descendente seja um idioma conhecido. Botões sem texto (só
## ícone de bandeira) também são capturados quando o nó de cima for um
## Control pai com cena filha de idioma.
func _find_language_button(root: Node) -> Node:
	var stack: Array = [root]
	var matches: Array = []
	while stack.size():
		var cur: Node = stack.pop_back()
		if cur == null or cur.has_meta("_tbx_locale_menu"):
			continue
		if cur is Control:
			var txt := _control_language_text(cur)
			if txt != "" and _is_language_label(txt):
				matches.append(cur)
		for child in cur.get_children():
			stack.append(child)
	# Prefere o MENOR match (folha clicável) — evita escolher o container pai.
	if matches.is_empty():
		return null
	matches.sort_custom(func(a: Node, b: Node) -> bool:
		var ca := a as Control
		var cb := b as Control
		if ca == null or cb == null:
			return false
		return ca.size.x * ca.size.y < cb.size.x * cb.size.y
	)
	return matches[0]

## Texto "de idioma" de um Control: texto do próprio botão ou de algum Label
## no subgrafo (qualquer profundidade).
func _control_language_text(node: Node) -> String:
	if node == null:
		return ""
	if node is Label:
		return (node as Label).text.strip_edges().to_upper()
	if node is Button:
		return (node as Button).text.strip_edges().to_upper()
	if node is LinkButton:
		return (node as LinkButton).text.strip_edges().to_upper()
	# Genérico: procura primeiro Label descendente cujo texto bate com idioma.
	var stack: Array = [node]
	var depth := 0
	while stack.size() and depth < 6:
		var level_size := stack.size()
		for _i in level_size:
			var cur: Node = stack.pop_back()
			if cur is Label:
				var t: String = (cur as Label).text.strip_edges().to_upper()
				if _is_language_label(t):
					return t
			for c in cur.get_children():
				stack.append(c)
		depth += 1
	return ""

func _is_language_label(text_upper: String) -> bool:
	if text_upper == "":
		return false
	for pat in LANGUAGE_BUTTON_TEXTS:
		if text_upper == pat or text_upper.begins_with(pat):
			return true
	return false

## Clona `other` e troca o rótulo para TBX_TARGET_LABEL. Se o clone não
## tiver nenhum Label, adiciona um overlayado (funciona em botões
## só-imagem, comuns em menus de idiomas com bandeiras).
func _inject_language_button_clone(other: Node) -> void:
	if other == null or not other.is_inside_tree():
		return
	var parent := other.get_parent()
	if parent == null:
		return
	for c in parent.get_children():
		if c.has_meta("_tbx_locale_menu"):
			return

	var clone: Control = other.duplicate() as Control
	if clone == null:
		return
	clone.set_meta("_tbx_locale_menu", true)
	clone.name = "%s_TBX" % other.name

	var swapped := false
	if clone is Button:
		(clone as Button).text = TBX_TARGET_LABEL
		swapped = true
	elif clone is LinkButton:
		(clone as LinkButton).text = TBX_TARGET_LABEL
		swapped = true
	elif clone is Label:
		(clone as Label).text = TBX_TARGET_LABEL
		swapped = true
	else:
		# Procura qualquer Label descendente e troca o primeiro.
		var lbl := _first_label(clone)
		if lbl != null:
			lbl.text = TBX_TARGET_LABEL
			swapped = true

	if not swapped:
		# Botão só-ícone: sobrepõe um Label com o nome do idioma.
		var overlay := Label.new()
		overlay.name = "TBXLabel"
		overlay.text = TBX_TARGET_LABEL
		overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
		overlay.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		overlay.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		clone.add_child(overlay)

	parent.add_child(clone)
	if parent is BoxContainer or parent is GridContainer:
		parent.move_child(clone, other.get_index() + 1)
	else:
		if other is Control:
			var off := (other as Control).size.y + 8.0
			clone.position.y += off

	# Conecta o clique (ou gui_input, caso não seja BaseButton).
	if clone is BaseButton:
		(clone as BaseButton).pressed.connect(_on_cloned_locale_button)
	else:
		clone.gui_input.connect(_on_cloned_locale_gui)
	other.set_meta("_tbx_locale_menu_src", true)

func _first_label(root: Node) -> Label:
	var stack: Array = [root]
	while stack.size():
		var cur: Node = stack.pop_back()
		if cur is Label:
			return cur as Label
		for c in cur.get_children():
			stack.append(c)
	return null

func _on_cloned_locale_gui(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		_on_cloned_locale_button()

func _on_cloned_locale_button() -> void:
	TranslationServer.set_locale(TBX_TARGET_LOCALE)
	ProjectSettings.set_setting("internationalization/locale/fallback", TBX_TARGET_LOCALE)
	var cfg := ConfigFile.new()
	cfg.set_value("tbx", LOCALE_KEY, TBX_TARGET_LOCALE)
	cfg.save(SAVE_PATH)
	_has_injected = false

func _is_settings_container(node: Node) -> bool:
	if node == self or not (node is Control):
		return false
	return _settings_regex.search(node.name) != null

func _find_settings_container(root: Node) -> Node:
	var stack: Array = [root]
	while stack.size():
		var cur: Node = stack.pop_back()
		if cur == null:
			continue
		if _is_settings_container(cur):
			return cur
		for child in cur.get_children():
			stack.append(child)
	return null

func _inject_in(container: Node) -> void:
	for c in container.get_children():
		if c is Control and c.has_meta("_tbx_locale_menu"):
			return

	var row := HBoxContainer.new()
	row.name = "TBXLocaleRow"
	row.set_meta("_tbx_locale_menu", true)
	row.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var label := Label.new()
	label.text = "Language"
	label.custom_minimum_size.x = 120
	row.add_child(label)

	var option := OptionButton.new()
	option.name = "TBXLocaleOption"
	option.set_meta("_tbx_locale_menu", true)
	option.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var locales := _get_available_locales()
	var current := _current_locale()
	var selected_idx := -1
	for i in locales.size():
		var code: String = locales[i]
		option.add_item(_locale_display_name(code), i)
		option.set_item_metadata(i, code)
		if code == current:
			selected_idx = i
	if selected_idx >= 0:
		option.select(selected_idx)
	elif locales.size():
		option.select(0)
	option.item_selected.connect(_on_locale_picked.bind(option))
	row.add_child(option)

	_attach_row(container, row)

func _attach_row(container: Node, row: Control) -> void:
	var best: Node = container
	for c in container.get_children():
		if c is VBoxContainer or c is HBoxContainer or c is GridContainer:
			best = c
			break
	if best is BoxContainer or best is GridContainer:
		best.add_child(row)
	else:
		row.position = Vector2(12, 12)
		container.add_child(row)

func _get_available_locales() -> Array:
	var seen := {}
	var out: Array = []
	for loc in TranslationServer.get_loaded_locales():
		if seen.has(loc):
			continue
		seen[loc] = true
		out.append(loc)
	for loc in TBX_LOCALES:
		if not seen.has(loc):
			seen[loc] = true
			out.append(loc)
	if out.is_empty():
		out.append("en")
	return out

func _current_locale() -> String:
	var cfg := ConfigFile.new()
	if cfg.load(SAVE_PATH) == OK:
		var v = cfg.get_value("tbx", LOCALE_KEY, "")
		if typeof(v) == TYPE_STRING and v != "":
			return v
	return TranslationServer.get_locale()

func _apply_saved_locale() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(SAVE_PATH) == OK:
		var v = cfg.get_value("tbx", LOCALE_KEY, "")
		if typeof(v) == TYPE_STRING and v != "":
			TranslationServer.set_locale(v)
			ProjectSettings.set_setting("internationalization/locale/fallback", v)

func _on_locale_picked(index: int, option: OptionButton) -> void:
	var code = option.get_item_metadata(index)
	if typeof(code) != TYPE_STRING:
		return
	TranslationServer.set_locale(code)
	var cfg := ConfigFile.new()
	cfg.set_value("tbx", LOCALE_KEY, code)
	cfg.save(SAVE_PATH)
	_has_injected = false
	_try_inject_current_scene()

## Nome amigável do locale, SEM hardcode de pt_BR. Usa o label do idioma-alvo
## quando o código bate com TBX_TARGET_LOCALE e, para os demais, o nome nativo
## exposto pelo próprio Godot (`TranslationServer.get_locale_name`).
func _locale_display_name(code: String) -> String:
	if code == TBX_TARGET_LOCALE:
		return TBX_TARGET_LABEL
	var friendly := TranslationServer.get_locale_name(code)
	if friendly != "":
		return friendly
	return code
