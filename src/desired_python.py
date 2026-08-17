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

            path = os.path.join(config.basedir, 'game', 'tl', 'tbx_temp', 'dump.txt')
            try:
                os.makedirs(os.path.dirname(path))
            except Exception:
                pass
            with io.open(path, 'w', encoding='utf-8') as f:
                def write_entry(filename, kind, value):
                    value = as_text(value).replace('\n', '\\n').replace('\r', '')
                    f.write(as_text(filename) + '|||' + as_text(kind) + '|||' + value + '\n')

                def extract_strings_from_obj(obj, seen=None, depth=0):
                    if seen is None: seen = set()
                    if id(obj) in seen: return set()
                    seen.add(id(obj))
                    texts = set()
                    if depth > 5: return texts
                    if isinstance(obj, string_types):
                        clean_obj = obj.strip()
                        if len(clean_obj) > 1 and len(clean_obj) < 800:
                            if not clean_obj.startswith('#') and not clean_obj.startswith('gui/') and not clean_obj.startswith('http'):
                                texts.add(obj)
                        return texts
                    if isinstance(obj, (list, tuple, set)):
                        for item in obj:
                            texts.update(extract_strings_from_obj(item, seen, depth+1))
                    elif isinstance(obj, dict):
                        for k, v in obj.items():
                            if isinstance(k, string_types) and len(k) > 1 and len(k) < 800: texts.add(k)
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

                for s in renpy.game.script.all_stmts:
                    try:
                        filename = s.filename.replace('\\', '/')
                        if 'renpy/common/' in filename: continue
                        if filename.endswith('/tbx_dumper.rpy') or filename.endswith('/tbx_boot.rpy'): continue
                        if 'game/tl/tbx_temp/' in filename or 'game/tl/tbx_temp_portuguese/' in filename: continue
                        if 'game/tl/' in filename:
                            rel_tl = filename.split('game/tl/', 1)[1]
                            rel_parts = rel_tl.split('/')
                            base_file = '/'.join(rel_parts[1:]) if len(rel_parts) > 1 else os.path.basename(filename)
                        else:
                            base_file = os.path.basename(filename)
                        if not base_file: continue
                        if isinstance(s, renpy.ast.Say):
                            text = s.what if s.what else ''
                            if text.strip():
                                write_entry(base_file, 'dialogo', text)
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
                        elif isinstance(s, renpy.ast.Python):
                            pass
                        else:
                            for attr in ('old', 'what', 'text', 'prompt'):
                                val = getattr(s, attr, None)
                                if isinstance(val, string_types) and val.strip():
                                    write_entry(base_file, 'interface', val)
                    except:
                        pass

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
                    'Takes a screenshot.', 'Hides the interface.', 'Opens the accessibility menu.'
                ]
                for s in standard_ui:
                    write_entry('screens.rpy', 'interface', s)

                import re
                for root, dirs, files in os.walk(os.path.join(config.basedir, 'game')):
                    for file in files:
                        if file.endswith('.rpy'):
                            try:
                                with io.open(os.path.join(root, file), 'r', encoding='utf-8') as rf:
                                    content = rf.read()
                                    import base64
                                    pat = base64.b64decode(b'X1woXHMqKFtcJ1x4MjJdKSguKj8pXDFccypcKQ==').decode('utf-8')
                                    for quote, match in re.findall(pat, content):
                                        if match.strip(): write_entry(file, 'interface', match)
                                    for m in re.finditer(r'(?i)\b__\(\s*([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                                    
                                    for m in re.finditer(r'(?i)\btext\s+([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                                    for m in re.finditer(r'(?i)\btextbutton\s+([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                                    for m in re.finditer(r'(?i)\blabel\s+([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                                    for m in re.finditer(r'(?i)\btooltip\s+([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                                    for m in re.finditer(r'(?i)\bname\s*(?:=|:)\s*([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                                    for m in re.finditer(r'(?i)\bdescription\s*(?:=|:)\s*([\'"])(.*?)\1', content):
                                        if m.group(2).strip(): write_entry(file, 'interface', m.group(2))
                            except:
                                pass

                # Do not crawl renpy.display.screen.screens here. That registry
                # also contains Ren'Py developer tools (warper/spline/action
                # editors), which are not part of the game. Game screens were
                # already collected from their AST nodes above.
        except Exception as e:
            import traceback
            with io.open(os.path.join(config.basedir, 'game', 'error.log'), 'w', encoding='utf-8') as err:
                err.write(as_text(traceback.format_exc()))
        finally:
            import sys
            sys.exit(0)
    config.start_callbacks.append(tbx_dump)
