init python:
    def tbx_dump():
        try:
            import os
            import io
            try:
                string_types = (basestring,)
                text_type = unicode
            except NameError:
                string_types = (str,)
                text_type = str

            def as_text(value):
                if isinstance(value, text_type):
                    return value
                try:
                    return value.decode('utf-8', 'replace')
                except Exception:
                    try:
                        return text_type(value)
                    except Exception:
                        return text_type(repr(value))

            # Sanitizacao: o delimitador "|||" pode aparecer no texto, e
            # alguns dumps antigos usavam \n literal. O formato aceito pelo parser Rust e:
            #   arquivo|||tipo|||valor   (com ID opcional:  arquivo|||tipo|||id|||valor)
            def sanitize_field(value):
                v = as_text(value).replace('\n', '\\n').replace('\r', '')
                # nunca deixar o separador vazar para o parser
                v = v.replace('|||', '{{pipe3}}')
                return v

            path = os.path.join(config.basedir, 'game', 'tl', 'tbx_temp', 'dump.txt')
            try:
                os.makedirs(os.path.dirname(path))
            except Exception:
                pass
            with io.open(path, 'w', encoding='utf-8') as f:
                def write_entry(filename, kind, value, ident=None):
                    base = sanitize_field(filename) + '|||' + sanitize_field(kind) + '|||'
                    if ident:
                        base += sanitize_field(ident) + '|||'
                    f.write(base + sanitize_field(value) + '\n')

                def is_translatable(value):
                    if not isinstance(value, string_types):
                        return False
                    clean = value.strip()
                    if len(clean) < 2 or len(clean) > 800:
                        return False
                    if clean.startswith('#') or clean.startswith('gui/') or clean.startswith('http'):
                        return False
                    return True

                def extract_strings_from_obj(obj, seen=None, depth=0):
                    if seen is None: seen = set()
                    if id(obj) in seen: return set()
                    seen.add(id(obj))
                    texts = set()
                    if depth > 5: return texts
                    if isinstance(obj, string_types):
                        if is_translatable(obj):
                            texts.add(obj.strip())
                        return texts
                    if isinstance(obj, (list, tuple, set)):
                        for item in obj:
                            texts.update(extract_strings_from_obj(item, seen, depth+1))
                    elif isinstance(obj, dict):
                        for k, v in obj.items():
                            if isinstance(k, string_types) and is_translatable(k): texts.add(k.strip())
                            texts.update(extract_strings_from_obj(v, seen, depth+1))
                    else:
                        if hasattr(obj, '__dict__'):
                            for k, v in obj.__dict__.items():
                                if k not in ('filename', 'name', 'loc', 'code', 'source', 'location', 'linenumber', 'parameters', 'arguments', 'properties', 'expr'):
                                    texts.update(extract_strings_from_obj(v, seen, depth+1))
                        if hasattr(obj, '__slots__'):
                            for slot in obj.__slots__:
                                try:
                                    v = getattr(obj, slot)
                                    texts.update(extract_strings_from_obj(v, seen, depth+1))
                                except:
                                    pass
                    return texts

                # === PASSO 1: AST (funciona mesmo com scripts dentro de .rpa) ===
                # O RenPy carrega a AST de todos os scripts (incluindo rpa) em
                # renpy.game.script.all_stmts. Nos de Say possuem `identifier`
                # estavel, que usamos como ID primario de traducao (sistema nativo
                # "translate <lang> <id>:"), mesclado com old/new como fallback.
                for s in renpy.game.script.all_stmts:
                    try:
                        filename = s.filename.replace('\\', '/')
                        if 'renpy/common/' in filename: continue
                        if filename.startswith('renpy/'): continue
                        if filename.endswith('/tbx_dumper.rpy') or filename.endswith('/tbx_boot.rpy'): continue

                        # Pular traducoes existentes/internas (qualquer coisa em tl/)
                        # e qualquer arquivo do engine independente do caminho completo.
                        base_name_chk = filename.rsplit('/', 1)[-1]
                        if base_name_chk in ('common.rpym', 'common.rpy'): continue
                        if '/tl/' in filename: continue

                        if 'game/tl/' in filename:
                            rel_tl = filename.split('game/tl/', 1)[1]
                            rel_parts = rel_tl.split('/')
                            base_file = '/'.join(rel_parts[1:]) if len(rel_parts) > 1 else os.path.basename(filename)
                        else:
                            # garante caminho relativo mesmo em RPA
                            base_file = filename.split('/')[-1] if '/game/' not in filename \
                                else filename.split('/game/', 1)[1]
                        if not base_file: continue

                        ident = getattr(s, 'identifier', None)

                        if isinstance(s, renpy.ast.Say):
                            text = s.what if s.what else ''
                            if text.strip():
                                write_entry(base_file, 'dialogo', text, ident)
                        elif isinstance(s, renpy.ast.Menu):
                            for item in s.items:
                                if item[0] and item[0].strip():
                                    try:
                                        val = renpy.python.py_eval(item[0])
                                        if not isinstance(val, string_types): val = as_text(val)
                                    except:
                                        val = item[0]
                                        if val.startswith('"') and val.endswith('"'): val = val[1:-1]
                                        elif val.startswith("'") and val.endswith("'"): val = val[1:-1]
                                    write_entry(base_file, 'menu', val)
                        elif isinstance(s, renpy.ast.Screen):
                            texts = extract_strings_from_obj(s.screen)
                            for txt in texts:
                                if txt.strip():
                                    clean = txt
                                    if clean.startswith('"') and clean.endswith('"'): clean = clean[1:-1]
                                    elif clean.startswith("'") and clean.endswith("'"): clean = clean[1:-1]
                                    write_entry(base_file, 'interface', clean)
                        elif isinstance(s, (renpy.ast.Show, renpy.ast.Scene)):
                            if hasattr(s, 'imspec') and s.imspec and s.imspec[0]:
                                for part in s.imspec[0]:
                                    if part != 'text' and isinstance(part, string_types) and part.strip():
                                        write_entry(base_file, 'interface', part)
                        else:
                            # UserStatement (ex.: "voice", "queue", definicoes custom)
                            # e translate blocks que carregam old/what/text/prompt
                            for attr in ('old', 'what', 'text', 'prompt'):
                                val = getattr(s, attr, None)
                                if isinstance(val, string_types) and val.strip():
                                    write_entry(base_file, 'interface', val, ident)
                    except:
                        pass

                # === PASSO 2: UI padrao do engine ===
                standard_ui = [
                    'Start', 'Start Game', 'Load', 'Load Game', 'Save', 'Save Game',
                    'Preferences', 'Options', 'Settings', 'Prefs', 'History', 'Log',
                    'Skip', 'Auto', 'Q.Save', 'Q.Load', 'Quick Save', 'Quick Load',
                    'Return', 'Back', 'Main Menu', 'Help', 'About', 'Quit', 'Exit',
                    'Menu', 'Continue', 'Replay', 'Display', 'Window', 'Fullscreen',
                    'Rollback Side', 'Disable', 'Left', 'Right', 'Unseen Text',
                    'After Choices', 'Transitions', 'All', 'None', 'Text Speed',
                    'Auto-Forward Time', 'Music Volume', 'Sound Volume', 'Voice Volume',
                    'Mute All', 'Language', 'Accessibility', 'Are you sure you want to quit?',
                    'Are you sure you want to return to the main menu?',
                    'Are you sure you want to overwrite your save?',
                    'Loading will lose unsaved progress. Are you sure you want to do this?',
                    'Are you sure you want to delete this save?',
                    'Yes', 'No', 'OK', 'Cancel', 'Empty Slot', 'Empty Slot.',
                    'Stop Skipping', 'Keep Skipping', 'Keyboard Shortcuts', 'Mouse', 'Gamepad',
                    'Previous', 'Next', 'Page {}', 'Automatic saves', 'Quick saves',
                    'Advance dialogue and activate the interface.', 'Rolls back to earlier dialogue.',
                    'Bypasses dialogue while held.', 'Toggles dialogue skipping.',
                    'Takes a screenshot.', 'Hides the interface.', 'Opens the accessibility menu.',
                    'Confirm', 'Hide', 'Title Screen', 'Please wait...', 'Click to continue.',
                    'Audio', 'Graphics', 'Text', 'Video Volume', 'Play', 'Stop', 'Mute', 'Volume',
                    'Up', 'Down', 'Joypad', 'Joystick', 'Save/Load', 'Return to main menu', 'Quit Game',
                    'Auto-Forward', 'Dialogue', 'Master Volume'
                ]
                for s in standard_ui:
                    write_entry('screens.rpy', 'interface', s)

                # === PASSO 3: varredura .rpy usando o loader nativo do RenPy ===
                # Funciona tambem para scripts embalados em .rpa: renpy.list_files()
                # lista arquivos virtuais e renpy.loader.load() le o conteudo.
                import re
                seen_files = set()
                try:
                    candidates = [fn for fn in renpy.list_files() if fn.endswith('.rpy')]
                except Exception:
                    candidates = []

                for vpath in candidates:
                    norm = vpath.replace('\\', '/')
                    if '/tl/' in norm or norm.endswith('/tbx_dumper.rpy') or norm.endswith('/tbx_boot.rpy'):
                        continue
                    # Arquivos do engine (screens/UI padrao do RenPy), nao do jogo
                    bn = norm.rsplit('/', 1)[-1]
                    if bn in ('common.rpym', 'common.rpy') or norm.startswith('renpy/common/'):
                        continue
                    if norm in seen_files:
                        continue
                    seen_files.add(norm)
                    base_file = norm.split('/')[-1] if '/game/' not in norm \
                        else norm.split('/game/', 1)[1]
                    try:
                        content = renpy.loader.load(vpath).read().decode('utf-8', 'replace')
                    except Exception:
                        continue

                    try:
                        import base64
                        pat = base64.b64decode(b'X1woXHMqKFtcJ1x4MjJdKSguKj8pXDFccypcKQ==').decode('utf-8')
                        for quote, match in re.findall(pat, content):
                            if match.strip(): write_entry(base_file, 'interface', match)
                        for m in re.finditer(r'(?i)\b__\(\s*([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                        for m in re.finditer(r'(?i)\btext\s+([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                        for m in re.finditer(r'(?i)\btextbutton\s+([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                        for m in re.finditer(r'(?i)\blabel\s+([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                        for m in re.finditer(r'(?i)\btooltip\s+([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                        for m in re.finditer(r'(?i)\bname\s*(?:=|:)\s*([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                        for m in re.finditer(r'(?i)\bdescription\s*(?:=|:)\s*([\'"])(.*?)\1', content):
                            if m.group(2).strip(): write_entry(base_file, 'interface', m.group(2))
                    except Exception:
                        pass

                # Nao varrer renpy.display.screen.screens aqui: o registro
                # tambem contem ferramentas de desenvolvedor do RenPy
                # (warper/spline/action editors), que nao fazem parte do jogo.
        except Exception as e:
            import traceback
            try:
                with io.open(os.path.join(config.basedir, 'game', 'error.log'), 'w', encoding='utf-8') as err:
                    err.write(as_text(traceback.format_exc()))
            except Exception:
                pass
        finally:
            import sys
            sys.exit(0)
    config.start_callbacks.append(tbx_dump)
