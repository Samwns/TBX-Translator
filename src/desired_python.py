init python:
    def tpg_dump():
        try:
            import os
            path = os.path.join(config.basedir, 'game', 'tl', 'tpg_temp', 'dump.txt')
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, 'w', encoding='utf-8') as f:
                def extract_strings_from_obj(obj, seen=None, depth=0):
                    if seen is None: seen = set()
                    if id(obj) in seen: return set()
                    seen.add(id(obj))
                    texts = set()
                    if depth > 20: return texts
                    if isinstance(obj, str):
                        if len(obj) > 1 and len(obj) < 800: texts.add(obj)
                        return texts
                    if isinstance(obj, (list, tuple, set)):
                        for item in obj:
                            texts.update(extract_strings_from_obj(item, seen, depth+1))
                    elif isinstance(obj, dict):
                        for k, v in obj.items():
                            if isinstance(k, str) and len(k) > 1 and len(k) < 800: texts.add(k)
                            texts.update(extract_strings_from_obj(v, seen, depth+1))
                    else:
                        if hasattr(obj, '__dict__'):
                            for k, v in obj.__dict__.items():
                                if k not in ('filename', 'name', 'loc', 'code', 'source', 'location', 'linenumber'):
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
                        if filename.endswith('/tpg_dumper.rpy') or filename.endswith('/tpg_boot.rpy'): continue
                        if 'game/tl/tpg_temp/' in filename or 'game/tl/tpg_temp_portuguese/' in filename: continue
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
                                f.write(base_file + '|||dialogo|||' + text.replace('\n', '\\n').replace('\r', '') + '\n')
                        elif isinstance(s, renpy.ast.Menu):
                            for item in s.items:
                                if item[0] and item[0].strip():
                                    try:
                                        val = renpy.python.py_eval(item[0])
                                        if not isinstance(val, str): val = str(val)
                                    except:
                                        val = item[0]
                                        if val.startswith('"') and val.endswith('"'): val = val[1:-1]
                                        elif val.startswith("'") and val.endswith("'"): val = val[1:-1]
                                    f.write(base_file + '|||menu|||' + val.replace('\n', '\\n').replace('\r', '') + '\n')
                        elif isinstance(s, renpy.ast.Screen):
                            texts = extract_strings_from_obj(s.screen)
                            for txt in texts:
                                if txt.strip():
                                    clean = txt
                                    if clean.startswith('"') and clean.endswith('"'): clean = clean[1:-1]
                                    elif clean.startswith("'") and clean.endswith("'"): clean = clean[1:-1]
                                    f.write(base_file + '|||interface|||' + clean.replace('\n', '\\n').replace('\r', '') + '\n')
                        elif isinstance(s, (renpy.ast.Show, renpy.ast.Scene)):
                            if hasattr(s, 'imspec') and s.imspec and s.imspec[0]:
                                for part in s.imspec[0]:
                                    if part != 'text' and isinstance(part, str) and part.strip():
                                        f.write(base_file + '|||interface|||' + part.replace('\n', '\\n').replace('\r', '') + '\n')
                        elif isinstance(s, renpy.ast.Python):
                            pass
                        else:
                            for attr in ('old', 'what', 'text', 'prompt'):
                                val = getattr(s, attr, None)
                                if isinstance(val, str) and val.strip():
                                    f.write(base_file + '|||interface|||' + val.replace('\n', '\\n').replace('\r', '') + '\n')
                    except:
                        pass

                standard_ui = ['Start', 'Load', 'Preferences', 'Quit', 'Main Menu', 'Return', 'Save', 'About', 'Help', 'Settings', 'History', 'Skip', 'Auto', 'Q.Save', 'Q.Load', 'Prefs', 'Options', 'Language', 'Menu', 'Back', 'Yes', 'No', 'Empty Slot', 'Are you sure you want to quit?', 'Are you sure you want to return to the main menu?', 'Window', 'Fullscreen', 'Transitions', 'All', 'None', 'Stop Skipping', 'Keep Skipping', 'Auto-Forward Time', 'Text Speed', 'Music Volume', 'Sound Volume', 'Voice Volume']
                for s in standard_ui:
                    f.write('screens.rpy|||interface|||' + s + '\n')

                import re
                for root, dirs, files in os.walk(os.path.join(config.basedir, 'game')):
                    for file in files:
                        if file.endswith('.rpy'):
                            try:
                                with open(os.path.join(root, file), 'r', encoding='utf-8') as rf:
                                    content = rf.read()
                                    import base64
                                    pat = base64.b64decode(b'X1woXHMqKFtcJ1x4MjJdKSguKj8pXDFccypcKQ==').decode('utf-8')
                                    matches = re.findall(pat, content)
                                    for quote, match in matches:
                                        if match.strip():
                                            f.write(file + '|||interface|||' + match.replace('\n', '\\n').replace('\r', '') + '\n')
                            except:
                                pass

                if hasattr(renpy.display, 'screen') and hasattr(renpy.display.screen, 'screens'):
                    all_str = set()
                    for key, screen_obj in renpy.display.screen.screens.items():
                        all_str.update(extract_strings_from_obj(screen_obj))
                    for txt in all_str:
                        if txt.strip():
                            clean = txt
                            if clean.startswith('"') and clean.endswith('"'): clean = clean[1:-1]
                            elif clean.startswith("'") and clean.endswith("'"): clean = clean[1:-1]
                            f.write('screens.rpy|||interface|||' + clean.replace('\n', '\\n').replace('\r', '') + '\n')
        except Exception as e:
            import traceback
            with open(os.path.join(config.basedir, 'game', 'error.log'), 'w') as err:
                err.write(traceback.format_exc())
        finally:
            import sys
            sys.exit(0)
    config.start_callbacks.append(tpg_dump)
