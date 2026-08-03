using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Encodings.Web;
using System.Text.Unicode;
using AssetsTools.NET;
using AssetsTools.NET.Extra;
using AssetsTools.NET.Texture;
using Mono.Cecil;
using Mono.Cecil.Cil;
using System.Text.RegularExpressions;

namespace unity_static_extractor
{
    class Program
    {
        static void Main(string[] args)
        {
            if (args.Length < 2)
            {
                Console.WriteLine("Usage: unity_static_extractor <extract/inject> <data_folder> [output_or_input.json]");
                return;
            }

            string mode = args[0].ToLower();
            string dataFolder = args[1];
            string jsonFile = args.Length > 2 ? args[2] : "extracted_texts.json";

            if (mode == "extract")
            {
                Extract(dataFolder, jsonFile);
            }
            else if (mode == "font-scan")
            {
                ScanFonts(dataFolder);
            }
            else if (mode == "font-inject" && args.Length >= 5)
            {
                ReplaceFont(dataFolder, args[2], args[3], args[4]);
            }
            else if (mode == "font-export" && args.Length >= 4)
            {
                ExportFont(dataFolder, args[2], args[3]);
            }
            else if (mode == "tmp-atlas-export" && args.Length >= 5)
            {
                ExportTmpAtlas(dataFolder, args[2], args[3], args[4]);
            }
            else if (mode == "inject")
            {
                Console.WriteLine("[C#] Direct asset injection is disabled with the UABEA reader. Use the XUnity/BepInEx injection flow.");
            }
            else
            {
                Console.WriteLine("Invalid mode.");
            }
        }

        // Pre-compiled regexes for performance
        private static readonly Regex _reUuid = new Regex(@"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", RegexOptions.IgnoreCase | RegexOptions.Compiled);
        private static readonly Regex _reMd5 = new Regex(@"^[0-9a-f]{32}$", RegexOptions.IgnoreCase | RegexOptions.Compiled);
        private static readonly Regex _reLineId = new Regex(@"^line:[0-9a-f]{6,}$", RegexOptions.IgnoreCase | RegexOptions.Compiled);
        private static readonly Regex _reInputPath = new Regex(@"^<[A-Za-z]+>/", RegexOptions.Compiled);
        private static readonly Regex _reInputDevice = new Regex(@"^<[A-Za-z]+>$", RegexOptions.Compiled);
        private static readonly Regex _reRangeStr = new Regex(@"^[0-9A-F]+-[0-9A-F]+,[0-9A-F]+-[0-9A-F]+$", RegexOptions.Compiled);
        private static readonly Regex _reSemicolon = new Regex(@"^;[A-Za-z]", RegexOptions.Compiled);
        private static readonly Regex _reCamel = new Regex(@"^[a-z]+[A-Z][a-zA-Z]+$", RegexOptions.Compiled);
        private static readonly Regex _rePascal = new Regex(@"^[A-Z][a-zA-Z]+[A-Z][a-zA-Z]+$", RegexOptions.Compiled);
        private static readonly Regex _reNamespace = new Regex(@"^([a-zA-Z0-9]+\.)+[a-zA-Z0-9]+$", RegexOptions.Compiled);
        private static readonly Regex _rePath = new Regex(@"^[a-zA-Z0-9_]+/[a-zA-Z0-9_]+", RegexOptions.Compiled);
        private static readonly Regex _reStarAction = new Regex(@"^\*/\{", RegexOptions.Compiled);
        private static readonly Regex _rePunctuationDump = new Regex(@"^[\p{P}\p{S}\p{M}\p{Z}\uFEFF]+$", RegexOptions.Compiled);

        static bool IsValidText(string text)
        {
            if (string.IsNullOrWhiteSpace(text)) return false;
            
            string s = text.Trim();
            
            // 1. Must have at least 2 letters
            if (s.Length < 2) return false;
            if (s.Count(char.IsLetter) < 2) return false;

            // 2. UUIDs (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
            if (_reUuid.IsMatch(s)) return false;

            // 3. MD5 hashes (32 hex chars)
            if (_reMd5.IsMatch(s)) return false;

            // 4. Dialogue line IDs (line:f2c2fd08)
            if (_reLineId.IsMatch(s)) return false;

            // 5. Input system device paths (<Keyboard>/w, <Gamepad>/leftStick, etc.)
            if (_reInputPath.IsMatch(s)) return false;

            // 6. Bare device tags (<Keyboard>, <Mouse>, <Gamepad>)
            if (_reInputDevice.IsMatch(s)) return false;

            // 7. Wildcard action paths (*/{Submit}, */{Cancel})
            if (_reStarAction.IsMatch(s)) return false;

            // 8. Semicolon-prefixed group strings (;Gamepad, ;Keyboard&Mouse)
            if (_reSemicolon.IsMatch(s)) return false;

            // 9. Unicode range strings (0-7F,400-4FF)
            if (_reRangeStr.IsMatch(s)) return false;

            // 10. File/asset paths
            if (_rePath.IsMatch(s)) return false;
            if (s.Contains("://")) return false;
            if (s.EndsWith(".asset", StringComparison.OrdinalIgnoreCase)) return false;
            if (s.EndsWith(".prefab", StringComparison.OrdinalIgnoreCase)) return false;

            // 11. Code patterns (no spaces = likely code)
            if (!s.Contains(" "))
            {
                if (_reCamel.IsMatch(s)) return false;    // camelCase
                if (_rePascal.IsMatch(s)) return false;   // PascalCase
                if (s.Contains("_")) return false;         // snake_case
                if (_reNamespace.IsMatch(s)) return false; // Namespace.Class
                if (s.Contains("&")) return false;         // Keyboard&Mouse
            }

            // 12. Specific internal Unity patterns (NOT rich text that players see)
            // KEEP: <color=>, <size=>, <i> — these are valid UI rich text used in dialogues!
            if (s.StartsWith("</")) return false;         // closing tags only (not useful alone)
            if (s.StartsWith("<line-")) return false;
            // NOTE: {0} and {1} are Yarn Spinner variables — KEEP THEM! They are real dialogue!
            if (s.StartsWith("#{0:X2}")) return false;    // hex format specifier (code only)
            if (s.StartsWith("InstallBindings:")) return false;
            if (s.StartsWith("CustomCharacter.")) return false;
            if (s.StartsWith("CharacterCustomiser.")) return false;
            if (s.Contains("LineBreaking")) return false;
            if (s.EndsWith("SDF")) return false;
            if (s.EndsWith(" SDF")) return false;
            if (s.StartsWith("UnityEngine.")) return false;
            if (s.StartsWith("com.unity.")) return false;
            if (s.StartsWith("Light Layer ")) return false;
            
            // 13. Internal code strings passing through
            if (s.Contains(", Assembly-CSharp")) return false;
            if (s.StartsWith("Normalize(")) return false;
            if (s.StartsWith("ScaleVector2(")) return false;
            if (s.StartsWith("MultiTap(")) return false;
            if (s.Contains("1DAxis")) return false;
            if (s.StartsWith("-language=")) return false;
            
            // 14. Punctuation or font dumps (only symbols/punctuation)
            if (_rePunctuationDump.IsMatch(s)) return false;

            // 15. JSON configs and embedded data payloads
            if (s.StartsWith("{") && s.Contains("\"name\"") && s.EndsWith("}")) return false;
            if (s.StartsWith("{") && s.Contains("\"version\"") && s.Contains("\"actions\"")) return false;
            if (s.StartsWith("{\"")) return false;
            if (s.StartsWith("color_name,hex,r,g,b")) return false;
            if (s.StartsWith("dev.yarnspinner.")) return false;

            return true;
        }

        // Known internal field names that should NEVER be treated as translatable text
        private static readonly HashSet<string> _internalFieldNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "m_Name", "m_Script", "m_GameObject", "m_FileID", "m_PathID",
            "m_Father", "m_Children", "m_Shader", "m_Material", "m_Materials",
            "m_Texture", "m_Sprite", "m_Font", "m_FontAsset", "m_Mesh",
            "m_RootOrder", "m_LocalPosition", "m_LocalRotation", "m_LocalScale",
            "m_Tag", "m_Layer", "m_IsActive", "m_Enabled", "m_CastShadows",
            "m_ReceiveShadows", "m_LightProbeUsage", "m_CorrespondingSourceObject",
            "m_PrefabInstance", "m_PrefabAsset", "m_EditorHideFlags", "m_ObjectHideFlags",
            "m_ClassName", "m_Namespace", "m_AssemblyName",
        };

        // Unity serializes both legacy UI.Text and TextMeshPro/TMP_Text as a
        // MonoBehaviour. Their displayed value is m_Text (some custom UI
        // components use text). Do not recursively collect every string: that
        // turns configuration, class names and identifiers into translations.
        private static readonly HashSet<string> _displayTextFieldNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "m_Text", "text", "m_DisplayText", "m_TranslatedText",
            "m_OriginalText", "m_LocalizedText", "m_Tooltip", "m_Description",
            // Frequently used by ScriptableObject dialogue/localisation systems.
            // These still pass IsValidText, so IDs and empty placeholders do not
            // become translation entries.
            "m_Title", "title", "m_Body", "body", "m_Message", "message",
            "m_Dialogue", "dialogue", "m_Subtitle", "subtitle", "m_Label",
            "label", "m_Prompt", "prompt", "m_Question", "question",
            "m_Choice", "choice", "m_Response", "response", "m_Content", "content"
        };
        private static readonly HashSet<string> _textAssetJsonFieldNames = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "text", "value", "localized", "translation", "translated",
            "dialogue", "line"
        };

        static void ExtractJsonTextValues(JsonElement element, HashSet<string> results, bool acceptedField = false)
        {
            if (element.ValueKind == JsonValueKind.String)
            {
                var value = element.GetString() ?? "";
                if (acceptedField && IsValidText(value)) results.Add(value);
                return;
            }
            if (element.ValueKind == JsonValueKind.Object)
                foreach (var property in element.EnumerateObject())
                    ExtractJsonTextValues(property.Value, results, _textAssetJsonFieldNames.Contains(property.Name));
            else if (element.ValueKind == JsonValueKind.Array)
                foreach (var item in element.EnumerateArray())
                    ExtractJsonTextValues(item, results, acceptedField);
        }

        static bool IsLikelyTextAsset(string name)
        {
            var lower = name.ToLowerInvariant();
            return lower.Contains("dialog") || lower.Contains("localiz") ||
                lower.Contains("string") || lower.Contains("subtitle") ||
                lower.Contains("yarn") || lower.Contains("story") ||
                lower.Contains("language") || lower.Contains("locale") ||
                lower.Contains("text");
        }

        static void ExtractTextAsset(string name, string script, HashSet<string> results)
        {
            if (string.IsNullOrWhiteSpace(script)) return;
            try
            {
                using var document = JsonDocument.Parse(script);
                ExtractJsonTextValues(document.RootElement, results);
                return;
            }
            catch (JsonException) { }

            // TextAsset files are commonly CSV, TSV, Ink/Yarn or a plain list,
            // and their asset names are often hashes.  Do not discard those just
            // because their name is generic; only add lines that look like text.
            foreach (var line in script.Split('\n'))
            {
                var trimmed = line.Trim();
                if (IsValidText(trimmed)) results.Add(trimmed);
            }
        }

        /// <summary>
        /// Recursively extracts only fields with a known player-facing text
        /// name. TextAsset is handled separately through m_Script.
        /// </summary>
        static void ExtractStringsFromField(AssetTypeValueField field, HashSet<string> results, int depth = 0)
        {
            if (depth > 8) return; // Prevent infinite recursion

            // Check if this field itself has a string value
            if (field.Value != null && field.Value.ValueType == AssetValueType.String)
            {
                string fieldName = field.FieldName ?? "";
                if (_displayTextFieldNames.Contains(fieldName))
                {
                    string val = field.AsString;
                    if (IsValidText(val)) results.Add(val);
                }
                return;
            }

            // Recurse into child fields
            if (field.Children != null)
            {
                foreach (var child in field.Children)
                {
                    if (child == null || child.IsDummy) continue;
                    ExtractStringsFromField(child, results, depth + 1);
                }
            }
        }

        // UABEA/AssetsTools.NET can open serialized files embedded in bundles.
        // Keeping this in the native scanner means bundle support does not depend
        // on Python; UnityPy below is an additional best-effort reader.
        static void ExtractBundle(AssetsManager manager, string bundlePath, HashSet<string> allTexts)
        {
            try
            {
                var bundle = manager.LoadBundleFile(bundlePath, true);
                int index = 0;
                foreach (var entry in bundle.file.BlockAndDirInfo.DirectoryInfos)
                {
                    try
                    {
                        if (entry.Name.EndsWith(".resS", StringComparison.OrdinalIgnoreCase) ||
                            entry.Name.EndsWith(".resource", StringComparison.OrdinalIgnoreCase))
                        {
                            index++;
                            continue;
                        }
                        var inst = manager.LoadAssetsFileFromBundle(bundle, index, true);
                        if (inst == null) { index++; continue; }
                        manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
                        foreach (var info in inst.file.GetAssetsOfType(114))
                        {
                            try
                            {
                                var field = manager.GetBaseField(inst, info);
                                if (field != null) ExtractStringsFromField(field, allTexts);
                            }
                            catch { }
                        }
                        foreach (var info in inst.file.GetAssetsOfType(49))
                        {
                            try
                            {
                                var field = manager.GetBaseField(inst, info);
                                var name = field?["m_Name"];
                                var script = field?["m_Script"];
                                if (script != null && !script.IsDummy && script.Value != null)
                                    ExtractTextAsset(name?.AsString ?? "", script.AsString, allTexts);
                            }
                            catch { }
                        }
                    }
                    catch { }
                    index++;
                }
                bundle.file.Reader.Close();
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[C#] Could not read bundle {Path.GetFileName(bundlePath)}: {ex.Message}");
            }
        }

        // Font handling uses the exact UABEA/AssetsTools.NET representation:
        // a Font asset (class 128) stores an importable TrueType/OpenType file
        // in m_FontData.Array.  TMP SDF assets intentionally are not offered
        // here: replacing only their source file does not rebuild its atlas.
        static string ResolveDataFolder(string root)
        {
            if (Directory.Exists(root) && Path.GetFileName(root).EndsWith("_Data", StringComparison.OrdinalIgnoreCase))
                return root;
            if (!Directory.Exists(root)) return root;
            return Directory.GetDirectories(root, "*_Data", SearchOption.TopDirectoryOnly).FirstOrDefault() ?? root;
        }

        static IEnumerable<string> SerializedAssetFiles(string root)
        {
            if (!Directory.Exists(root)) yield break;
            foreach (var file in Directory.GetFiles(root, "*", SearchOption.AllDirectories))
            {
                var name = Path.GetFileName(file);
                if (name.EndsWith(".resS", StringComparison.OrdinalIgnoreCase) ||
                    name.EndsWith(".resource", StringComparison.OrdinalIgnoreCase)) continue;
                if (name.EndsWith(".assets", StringComparison.OrdinalIgnoreCase) ||
                    name.StartsWith("sharedassets", StringComparison.OrdinalIgnoreCase) ||
                    name.StartsWith("level", StringComparison.OrdinalIgnoreCase))
                    yield return file;
            }
        }

        static AssetsManager CreateManager(string dataFolder)
        {
            var manager = new AssetsManager();
            string tpkPath = Path.Combine(AppContext.BaseDirectory, "classdata.tpk");
            manager.LoadClassPackage(File.Exists(tpkPath) ? tpkPath : "classdata.tpk");
            string managed = Path.Combine(dataFolder, "Managed");
            if (Directory.Exists(managed)) manager.MonoTempGenerator = new MonoCecilTempGenerator(managed);
            return manager;
        }

        static bool TryGetEmbeddedFont(AssetTypeValueField baseField, out AssetTypeValueField fontData)
        {
            fontData = null!;
            try
            {
                var data = baseField["m_FontData.Array"];
                if (data == null || data.IsDummy || data.Value == null) return false;
                // UABEA's FontPlugin marks this vector as ByteArray before
                // reading/writing it; doing the same supports older Unity files.
                data.TemplateField.ValueType = AssetValueType.ByteArray;
                fontData = data;
                return true;
            }
            catch { return false; }
        }

        static bool HasField(AssetTypeValueField field, string name)
        {
            try { var value = field[name]; return value != null && !value.IsDummy; }
            catch { return false; }
        }

        static void ScanFontsInAssets(AssetsManager manager, AssetsFileInstance inst, string label, HashSet<string> emitted, ref int count)
        {
            manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
            foreach (var info in inst.file.GetAssetsOfType((int)AssetClassID.Font))
            {
                try
                {
                    var field = manager.GetBaseField(inst, info);
                    if (field == null || !TryGetEmbeddedFont(field, out var bytes) || bytes.AsByteArray.Length == 0) continue;
                    var name = field["m_Name"].AsString.Replace('|', ' ');
                    if (emitted.Add($"EMBEDDED|{label}|{name}|{info.PathId}"))
                    {
                        Console.WriteLine($"[FONT_SCAN] EMBEDDED|{label}|{name}|{info.PathId}"); count++;
                    }
                }
                catch { }
            }

            // TMP_FontAsset is a MonoBehaviour in player builds. It stores a
            // baked SDF atlas, not a raw TTF/OTF; list it so the user sees the
            // actual source of the on-screen font, but don't offer unsafe import.
            foreach (var info in inst.file.GetAssetsOfType(114))
            {
                try
                {
                    var field = manager.GetBaseField(inst, info);
                    if (field == null || (!HasField(field, "m_FaceInfo") && !HasField(field, "m_AtlasTexture"))) continue;
                    var name = field["m_Name"].AsString.Replace('|', ' ');
                    if (String.IsNullOrWhiteSpace(name)) continue;
                    if (emitted.Add($"TMP|{label}|{name}|{info.PathId}"))
                    {
                        Console.WriteLine($"[FONT_SCAN] TMP|{label}|{name}|{info.PathId}"); count++;
                    }
                }
                catch { }
            }
        }

        static void ScanFonts(string root)
        {
            string dataFolder = ResolveDataFolder(root);
            var manager = CreateManager(dataFolder);
            int count = 0;
            var emitted = new HashSet<string>();
            foreach (var file in SerializedAssetFiles(dataFolder).Distinct())
            {
                try
                {
                    var inst = manager.LoadAssetsFile(file, true);
                    Console.WriteLine($"[C#] Unity version: {inst.file.Metadata.UnityVersion}");
                    var relative = Path.GetRelativePath(dataFolder, file).Replace('\\', '/');
                    ScanFontsInAssets(manager, inst, relative, emitted, ref count);
                    inst.file.Reader.Close();
                }
                catch { }
            }

            // TextMeshPro assets are commonly packed in Addressables/bundles.
            // Read each entry with the same UABEA parser used by the text scanner.
            var extensions = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { ".bundle", ".unity3d", ".ab", ".assetbundle" };
            foreach (var bundlePath in Directory.GetFiles(dataFolder, "*", SearchOption.AllDirectories)
                .Where(p => extensions.Contains(Path.GetExtension(p))))
            {
                try
                {
                    var bundle = manager.LoadBundleFile(bundlePath, true);
                    for (int index = 0; index < bundle.file.BlockAndDirInfo.DirectoryInfos.Length; index++)
                    {
                        try
                        {
                            var inst = manager.LoadAssetsFileFromBundle(bundle, index, true);
                            if (inst == null) continue;
                            var entry = bundle.file.BlockAndDirInfo.DirectoryInfos[index].Name.Replace('|', ' ');
                            var label = $"{Path.GetRelativePath(dataFolder, bundlePath).Replace('\\', '/')}::{entry}";
                            ScanFontsInAssets(manager, inst, label, emitted, ref count);
                        }
                        catch { }
                    }
                    bundle.file.Reader.Close();
                }
                catch { }
            }
            Console.WriteLine($"[C#] UABEA found {count} embedded Font assets.");
        }

        static string SafeFileName(string value)
        {
            foreach (var invalid in Path.GetInvalidFileNameChars()) value = value.Replace(invalid, '_');
            return String.IsNullOrWhiteSpace(value) ? "unity-font" : value;
        }

        static void ExportFont(string root, string fontLocator, string outputDirectory)
        {
            string dataFolder = ResolveDataFolder(root);
            var pieces = fontLocator.Split('|');
            if (pieces.Length != 2 || !long.TryParse(pieces[1], out var pathId))
            {
                Console.WriteLine("[ERROR] Identificador de fonte inválido."); return;
            }
            string assetPath = Path.Combine(dataFolder, pieces[0].Replace('/', Path.DirectorySeparatorChar));
            if (!File.Exists(assetPath)) { Console.WriteLine("[ERROR] Asset da fonte não encontrado."); return; }
            try
            {
                var manager = CreateManager(dataFolder);
                var inst = manager.LoadAssetsFile(assetPath, true);
                manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
                var info = inst.file.GetAssetsOfType((int)AssetClassID.Font).FirstOrDefault(i => i.PathId == pathId);
                var field = info == null ? null : manager.GetBaseField(inst, info);
                if (field == null || !TryGetEmbeddedFont(field, out var fontData) || fontData.AsByteArray.Length == 0)
                    { Console.WriteLine("[ERROR] Esta fonte não possui dados TTF/OTF incorporados."); return; }
                var bytes = fontData.AsByteArray;
                var extension = bytes.Length >= 4 && bytes[0] == 0x4f && bytes[1] == 0x54 && bytes[2] == 0x54 && bytes[3] == 0x4f ? "otf" : "ttf";
                Directory.CreateDirectory(outputDirectory);
                var output = Path.Combine(outputDirectory, $"{SafeFileName(field["m_Name"].AsString)}-{pathId}.{extension}");
                File.WriteAllBytes(output, bytes);
                inst.file.Reader.Close();
                Console.WriteLine("[SUCCESS] " + output);
            }
            catch (Exception ex) { Console.WriteLine("[ERROR] " + ex); }
        }

        static byte[]? ReadTextureBytes(TextureFile texture, AssetsFileInstance inst)
        {
            if (texture.m_StreamData.size == 0 || String.IsNullOrEmpty(texture.m_StreamData.path)) return texture.pictureData;
            var path = texture.m_StreamData.path.StartsWith("archive:/")
                ? Path.Combine(Path.GetDirectoryName(inst.path) ?? "", Path.GetFileName(texture.m_StreamData.path))
                : Path.Combine(Path.GetDirectoryName(inst.path) ?? "", texture.m_StreamData.path);
            if (!File.Exists(path)) return null;
            using var stream = File.OpenRead(path);
            stream.Position = (long)texture.m_StreamData.offset;
            return new BinaryReader(stream).ReadBytes((int)texture.m_StreamData.size);
        }

        // TMP player builds keep a baked grayscale SDF atlas instead of the
        // original font file. Export it as portable PPM so GTK can display an
        // honest preview without a platform-specific texture decoder.
        static void ExportTmpAtlas(string root, string assetRelativePath, string tmpPathIdText, string outputDirectory)
        {
            if (!long.TryParse(tmpPathIdText, out var tmpPathId)) { Console.WriteLine("[ERROR] TMP pathId inválido."); return; }
            string dataFolder = ResolveDataFolder(root);
            string assetPath = Path.Combine(dataFolder, assetRelativePath.Replace('/', Path.DirectorySeparatorChar));
            try
            {
                var manager = CreateManager(dataFolder);
                var inst = manager.LoadAssetsFile(assetPath, true);
                manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
                var tmpInfo = inst.file.GetAssetsOfType(114).FirstOrDefault(i => i.PathId == tmpPathId);
                var tmp = tmpInfo == null ? null : manager.GetBaseField(inst, tmpInfo);
                if (tmp == null) { Console.WriteLine("[ERROR] Asset TMP não encontrado."); return; }
                var ptr = tmp["m_AtlasTexture"];
                if (ptr.IsDummy)
                {
                    var atlases = tmp["m_AtlasTextures"];
                    if (!atlases.IsDummy)
                    {
                        var array = atlases["Array"];
                        if (!array.IsDummy && array.Children.Count > 0) ptr = array.Children[0];
                    }
                }
                if (ptr.IsDummy) { Console.WriteLine("[ERROR] Referência do atlas TMP não encontrada."); return; }
                var ext = manager.GetExtAsset(inst, ptr);
                if (ext.info == null || ext.file == null) { Console.WriteLine("[ERROR] Atlas Texture2D não encontrado."); return; }
                var textureField = manager.GetBaseField(ext.file, ext.info);
                if (textureField == null) { Console.WriteLine("[ERROR] Não foi possível ler o atlas."); return; }
                var dataField = textureField["image data"];
                if (dataField.IsDummy) dataField = textureField["m_ImageData"];
                if (dataField.IsDummy) { Console.WriteLine("[ERROR] Dados de imagem do atlas não encontrados."); return; }
                dataField.TemplateField.ValueType = AssetValueType.ByteArray;
                var texture = TextureFile.ReadTextureFile(textureField);
                var raw = ReadTextureBytes(texture, ext.file);
                int pixels = texture.m_Width * texture.m_Height;
                if (raw == null || pixels <= 0 || raw.Length < pixels) { Console.WriteLine("[ERROR] Atlas comprimido ou sem dados legíveis."); return; }
                Directory.CreateDirectory(outputDirectory);
                string output = Path.Combine(outputDirectory, $"tmp-atlas-{tmpPathId}.ppm");
                using (var stream = File.Create(output))
                using (var writer = new BinaryWriter(stream))
                {
                    writer.Write(Encoding.ASCII.GetBytes($"P6\n{texture.m_Width} {texture.m_Height}\n255\n"));
                    int stride = raw.Length >= pixels * 4 ? 4 : raw.Length >= pixels * 2 ? 2 : 1;
                    for (int i = 0; i < pixels; i++)
                    {
                        byte value = raw[i * stride + (stride == 4 ? 3 : 0)];
                        writer.Write(value); writer.Write(value); writer.Write(value);
                    }
                }
                Console.WriteLine("[SUCCESS] " + output);
            }
            catch (Exception ex) { Console.WriteLine("[ERROR] " + ex); }
        }

        static void ReplaceFont(string root, string fontLocator, string expectedName, string fontFile)
        {
            string dataFolder = ResolveDataFolder(root);
            if (!File.Exists(fontFile)) { Console.WriteLine("[ERROR] Arquivo de fonte não encontrado."); return; }
            var pieces = fontLocator.Split('|');
            if (pieces.Length != 2 || !long.TryParse(pieces[1], out var pathId))
            {
                Console.WriteLine("[ERROR] Identificador de fonte inválido."); return;
            }
            string assetPath = Path.Combine(dataFolder, pieces[0].Replace('/', Path.DirectorySeparatorChar));
            if (!File.Exists(assetPath)) { Console.WriteLine("[ERROR] Asset da fonte não encontrado."); return; }

            var manager = CreateManager(dataFolder);
            try
            {
                var inst = manager.LoadAssetsFile(assetPath, true);
                manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
                var info = inst.file.GetAssetsOfType((int)AssetClassID.Font).FirstOrDefault(i => i.PathId == pathId);
                if (info == null) { Console.WriteLine("[ERROR] Fonte não encontrada no asset."); return; }
                var field = manager.GetBaseField(inst, info);
                if (field == null || !TryGetEmbeddedFont(field, out var fontData)) { Console.WriteLine("[ERROR] Esta fonte não possui m_FontData incorporado."); return; }
                if (!string.IsNullOrEmpty(expectedName) && !String.Equals(field["m_Name"].AsString, expectedName, StringComparison.Ordinal))
                    { Console.WriteLine("[ERROR] A fonte selecionada mudou; escaneie novamente."); return; }

                fontData.AsByteArray = File.ReadAllBytes(fontFile);
                byte[] newData = field.WriteToByteArray();
                var replacer = new AssetsReplacerFromMemory(info.PathId, info.TypeId, 0xffff, newData);
                string backup = assetPath + ".tbx-font-backup";
                if (!File.Exists(backup)) File.Copy(assetPath, backup);
                string temporary = assetPath + ".tbx-font-temp";
                using (var stream = File.Create(temporary))
                using (var writer = new AssetsFileWriter(stream))
                    inst.file.Write(writer, 0, new List<AssetsReplacer> { replacer });
                inst.file.Reader.Close();
                File.Move(temporary, assetPath, true);
                Console.WriteLine("[SUCCESS] Fonte incorporada com UABEA. Backup: " + backup);
            }
            catch (Exception ex) { Console.WriteLine("[ERROR] " + ex.Message); }
        }

        // A sizable part of Unity UI is created in code (including dialogue
        // choices and warning screens), rather than serialized in .assets.
        // UABEA handles serialized data; Mono.Cecil is used only for managed
        // game assemblies so those player-facing ldstr values are not missed.
        static void ExtractManagedStrings(string managedFolder, HashSet<string> results)
        {
            if (!Directory.Exists(managedFolder)) return;
            int added = 0;
            foreach (var assemblyPath in Directory.GetFiles(managedFolder, "*.dll", SearchOption.TopDirectoryOnly))
            {
                var name = Path.GetFileNameWithoutExtension(assemblyPath);
                // Plug-ins contain editor diagnostics, licenses and framework
                // messages by the thousands. Game scripts are conventionally
                // compiled into these two assemblies, including IL2CPP hybrid
                // projects that still ship a managed front-end.
                if (!name.Equals("Assembly-CSharp", StringComparison.OrdinalIgnoreCase) &&
                    !name.Equals("Assembly-CSharp-firstpass", StringComparison.OrdinalIgnoreCase)) continue;
                try
                {
                    using var module = ModuleDefinition.ReadModule(assemblyPath, new ReaderParameters { ReadSymbols = false });
                    foreach (var type in module.GetTypes())
                    foreach (var method in type.Methods)
                    {
                        if (!method.HasBody) continue;
                        foreach (var instruction in method.Body.Instructions)
                        {
                            if (instruction.OpCode != OpCodes.Ldstr || instruction.Operand is not string value) continue;
                            if (IsValidText(value) && results.Add(value)) added++;
                        }
                    }
                }
                catch { /* stripped/unsupported assemblies are non-fatal */ }
            }
            Console.WriteLine($"[C#] Added {added} player-facing strings from managed game code.");
        }


        static void Extract(string dataFolder, string jsonFile)
        {
            Console.WriteLine($"[C#] EXTRACT mode started for: {dataFolder}");
            HashSet<string> allTexts = new HashSet<string>();

            string managedFolder = Path.Combine(dataFolder, "Managed");

            Console.WriteLine("[C#] Extracting from .assets and .bundle files...");
            var manager = new AssetsManager();
            string tpkPath = Path.Combine(AppContext.BaseDirectory, "classdata.tpk");
            if (File.Exists(tpkPath)) manager.LoadClassPackage(tpkPath);
            else manager.LoadClassPackage("classdata.tpk"); 
            
            if (Directory.Exists(managedFolder)) {
                manager.MonoTempGenerator = new MonoCecilTempGenerator(managedFolder);
                Console.WriteLine("[C#] Enabled MonoDeserializer for precise UI extraction.");
            } 

            var allFiles = new List<string>();
            allFiles.AddRange(Directory.GetFiles(dataFolder, "*.assets", SearchOption.AllDirectories));
            
            var levelFiles = Directory.GetFiles(dataFolder, "level*", SearchOption.AllDirectories);
            foreach (var f in levelFiles) {
                if (!f.EndsWith(".resS", StringComparison.OrdinalIgnoreCase) && !f.EndsWith(".resource", StringComparison.OrdinalIgnoreCase))
                    allFiles.Add(f);
            }
            
            var sharedFiles = Directory.GetFiles(dataFolder, "sharedassets*", SearchOption.AllDirectories);
            foreach (var f in sharedFiles) {
                if (!f.EndsWith(".resS", StringComparison.OrdinalIgnoreCase) && !f.EndsWith(".resource", StringComparison.OrdinalIgnoreCase))
                    allFiles.Add(f);
            }

            foreach (var file in allFiles.Distinct())
            {
                try
                {
                    var inst = manager.LoadAssetsFile(file, true);
                    manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);

                    // Scan ALL MonoBehaviours (114) — let IsValidText filter the noise
                    foreach (var info in inst.file.GetAssetsOfType(114))
                    {
                        try
                        {
                            var baseField = manager.GetBaseField(inst, info);
                            if (baseField == null) continue;
                            // Recursively extract all string fields from the component
                            ExtractStringsFromField(baseField, allTexts);
                        }
                        catch { }
                    }

                    // Also process TextAssets (49) which might contain dialogue files
                    foreach (var info in inst.file.GetAssetsOfType(49))
                    {
                        try
                        {
                            var baseField = manager.GetBaseField(inst, info);
                            if (baseField == null) continue;

                            var nameF = baseField["m_Name"];
                            string assetName = (!nameF.IsDummy && nameF.Value != null) ? nameF.AsString : "";

                            var scriptF = baseField["m_Script"];
                            if (!scriptF.IsDummy && scriptF.Value != null) {
                                string val = scriptF.AsString;
                                ExtractTextAsset(assetName, val, allTexts);
                            }
                        }
                        catch { }
                    }
                }
                catch { }
            }

            // This includes conventional bundles and Unity Addressables bundles.
            // Addressables may be extensionless, so inspect extensionless files in
            // StreamingAssets/aa as well.
            var bundleExtensions = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
                { ".bundle", ".unity3d", ".ab", ".assetbundle" };
            var bundleFiles = Directory.GetFiles(dataFolder, "*", SearchOption.AllDirectories)
                .Where(path =>
                {
                    var extension = Path.GetExtension(path);
                    var normalized = path.Replace('\\', '/');
                    return bundleExtensions.Contains(extension) ||
                        (string.IsNullOrEmpty(extension) && normalized.IndexOf("/StreamingAssets/aa/", StringComparison.OrdinalIgnoreCase) >= 0);
                });
            foreach (var bundlePath in bundleFiles)
            {
                Console.WriteLine($"[C#] Scanning bundle: {Path.GetFileName(bundlePath)}");
                ExtractBundle(manager, bundlePath, allTexts);
            }

            ExtractManagedStrings(managedFolder, allTexts);

            var opts = new JsonSerializerOptions { WriteIndented = true, Encoder = JavaScriptEncoder.Create(UnicodeRanges.All) };
            File.WriteAllText(jsonFile, JsonSerializer.Serialize(allTexts.ToList(), opts));
            Console.WriteLine($"[C#] Extracted {allTexts.Count} strings to {jsonFile}");
        }

        // This legacy writer targets the older AssetsTools.NET API. Keep its
        // source available for a future UABEA-compatible rewrite, but do not
        // compile it against UABEA's current reader/writer API.
#if LEGACY_DIRECT_INJECTION
        static void Inject(string dataFolder, string jsonFile)
        {
            if (!File.Exists(jsonFile))
            {
                Console.WriteLine($"[C#] File not found: {jsonFile}");
                return;
            }

            var jsonContent = File.ReadAllText(jsonFile);
            var dict = JsonSerializer.Deserialize<Dictionary<string, string>>(jsonContent);
            if (dict == null) return;

            Console.WriteLine($"[C#] Loaded {dict.Count} translations.");

            var manager = new AssetsManager();
            string tpkPath = Path.Combine(AppContext.BaseDirectory, "classdata.tpk");
            if (File.Exists(tpkPath)) manager.LoadClassPackage(tpkPath);
            else manager.LoadClassPackage("classdata.tpk"); 

            string managedFolder = Path.Combine(dataFolder, "Managed");
            if (Directory.Exists(managedFolder)) {
                manager.MonoTempGenerator = new MonoCecilTempGenerator(managedFolder);
                Console.WriteLine("[C#] Enabled MonoDeserializer for precise UI injection.");
            }
            if (Directory.Exists(managedFolder))
            {
                string asmPath = Path.Combine(managedFolder, "Assembly-CSharp.dll");
                if (File.Exists(asmPath))
                {
                    try
                    {
                        string backupAsm = asmPath + ".bak";
                        if (!File.Exists(backupAsm)) File.Copy(asmPath, backupAsm);
                        
                        var readerParams = new ReaderParameters { ReadWrite = false };
                        var module = ModuleDefinition.ReadModule(asmPath, readerParams);
                        bool modified = false;
                        foreach (var type in module.Types)
                        {
                            foreach (var method in type.Methods)
                            {
                                if (!method.HasBody) continue;
                                foreach (var instr in method.Body.Instructions)
                                {
                                    if (instr.OpCode == OpCodes.Ldstr && instr.Operand is string s && dict.TryGetValue(s, out string translated))
                                    {
                                        instr.Operand = translated;
                                        modified = true;
                                    }
                                }
                            }
                        }
                        if (modified) 
                        {
                            string tempAsm = asmPath + ".temp";
                            module.Write(tempAsm);
                            module.Dispose();
                            File.Delete(asmPath);
                            File.Move(tempAsm, asmPath);
                        }
                        else
                        {
                            module.Dispose();
                        }
                    }
                    catch (Exception ex) { Console.WriteLine($"[C#] Error writing dll: {ex.Message}"); }
                }
            }

            Console.WriteLine("[C#] Injecting into .assets files...");


            var allFiles = new List<string>();
            allFiles.AddRange(Directory.GetFiles(dataFolder, "*.assets", SearchOption.AllDirectories));
            
            var levelFiles = Directory.GetFiles(dataFolder, "level*", SearchOption.AllDirectories);
            foreach (var f in levelFiles) {
                if (!f.EndsWith(".resS", StringComparison.OrdinalIgnoreCase) && !f.EndsWith(".resource", StringComparison.OrdinalIgnoreCase))
                    allFiles.Add(f);
            }
            
            var sharedFiles = Directory.GetFiles(dataFolder, "sharedassets*", SearchOption.AllDirectories);
            foreach (var f in sharedFiles) {
                if (!f.EndsWith(".resS", StringComparison.OrdinalIgnoreCase) && !f.EndsWith(".resource", StringComparison.OrdinalIgnoreCase))
                    allFiles.Add(f);
            }

            foreach (var file in allFiles.Distinct())
            {
                try
                {
                    var inst = manager.LoadAssetsFile(file, true);
                    manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
                    bool fileModified = false;

                    foreach (var info in inst.file.GetAssetsOfType(114))
                    {
                        bool modified = false;

                        var baseField = manager.GetBaseField(inst, info);
                        if (baseField == null) continue;

                        bool isTextComponent = false;
                        var scriptPtr = baseField["m_Script"];
                        if (!scriptPtr.IsDummy && scriptPtr.Value != null)
                        {
                            var ext = manager.GetExtAsset(inst, scriptPtr);
                            if (ext.info != null)
                            {
                                var scriptField = manager.GetBaseField(ext.file, ext.info);
                                if (scriptField != null)
                                {
                                    var classNameF = scriptField["m_ClassName"];
                                    if (!classNameF.IsDummy && classNameF.Value != null)
                                    {
                                        string cname = classNameF.AsString;
                                        if (cname.Contains("Text") || cname.Contains("Label") || cname.Contains("Dialogue") || cname == "TMP_Text" || cname.Contains("Localization") || cname.Contains("Database") || cname.Contains("Data") || cname.Contains("String") || cname.Contains("Story")) 
                                        {
                                            isTextComponent = true;
                                        }
                                    }
                                }
                            }
                        }

                        if (!isTextComponent) continue;

                        var textF = baseField["m_Text"];
                        if (!textF.IsDummy && textF.Value != null && textF.Value.ValueType == AssetValueType.String) {
                            string val = textF.AsString;
                            if (dict.TryGetValue(val, out string translated)) {
                                textF.AsString = translated;
                                modified = true;
                            }
                        }
                        
                        var textLowerF = baseField["text"];
                        if (!textLowerF.IsDummy && textLowerF.Value != null && textLowerF.Value.ValueType == AssetValueType.String) {
                            string val = textLowerF.AsString;
                            if (dict.TryGetValue(val, out string translated)) {
                                textLowerF.AsString = translated;
                                modified = true;
                            }
                        }

                        if (modified)
                        {
                            info.SetNewData(baseField.WriteToByteArray());
                            fileModified = true;
                        }
                    }

                    foreach (var info in inst.file.GetAssetsOfType(49))
                    {
                        var baseField = manager.GetBaseField(inst, info);
                        if (baseField == null) continue;

                        var scriptF = baseField["m_Script"];
                        if (!scriptF.IsDummy && scriptF.Value != null) {
                            string val = scriptF.AsString;
                            if (dict.TryGetValue(val, out string translated)) {
                                scriptF.AsString = translated;
                                info.SetNewData(baseField.WriteToByteArray());
                                fileModified = true;
                            }
                        }
                    }

                    if (fileModified)
                    {
                        string backupPath = file + ".bak";
                        if (!File.Exists(backupPath)) File.Copy(file, backupPath);

                        string tempFile = file + ".temp";
                        using (var stream = File.Create(tempFile))
                        using (var writer = new AssetsFileWriter(stream))
                        {
                            inst.file.Write(writer);
                        }
                        inst.file.Reader.Close();
                        File.Delete(file);
                        File.Move(tempFile, file);
                        Console.WriteLine($"[C#] Injected into {Path.GetFileName(file)}");
                    }
                    else
                    {
                        inst.file.Reader.Close();
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[C#] Error modifying assets file {file}: {ex.Message}");
                }
            }

            Console.WriteLine("[C#] Injecting into .bundle files...");
            var bundleFiles = Directory.GetFiles(dataFolder, "*.bundle", SearchOption.AllDirectories);
            foreach (var file in bundleFiles)
            {
                try
                {
                    var bunInst = manager.LoadBundleFile(file, true);
                    bool bunModified = false;

                    int index = 0;
                    foreach (var dirInfo in bunInst.file.BlockAndDirInfo.DirectoryInfos)
                    {
                        if (dirInfo.Name.EndsWith(".resS") || dirInfo.Name.EndsWith(".resource")) {
                            index++;
                            continue;
                        }

                        var inst = manager.LoadAssetsFileFromBundle(bunInst, index, true);
                        if (inst != null)
                        {
                            manager.LoadClassDatabaseFromPackage(inst.file.Metadata.UnityVersion);
                            bool fileModified = false;

                            foreach (var info in inst.file.GetAssetsOfType(114))
                            {
                                bool modified = false;

                                var baseField = manager.GetBaseField(inst, info);
                                if (baseField == null) continue;

                                bool isTextComponent = false;
                                var scriptPtr = baseField["m_Script"];
                                if (!scriptPtr.IsDummy && scriptPtr.Value != null)
                                {
                                    var ext = manager.GetExtAsset(inst, scriptPtr);
                                    if (ext.info != null)
                                    {
                                        var scriptField = manager.GetBaseField(ext.file, ext.info);
                                        if (scriptField != null)
                                        {
                                            var classNameF = scriptField["m_ClassName"];
                                            if (!classNameF.IsDummy && classNameF.Value != null)
                                            {
                                                string cname = classNameF.AsString;
                                                if (cname.Contains("Text") || cname.Contains("Label") || cname.Contains("Dialogue") || cname == "TMP_Text" || cname.Contains("Localization") || cname.Contains("Database") || cname.Contains("Data") || cname.Contains("String") || cname.Contains("Story")) 
                                                {
                                                    isTextComponent = true;
                                                }
                                            }
                                        }
                                    }
                                }

                                if (!isTextComponent) continue;

                                var textF = baseField["m_Text"];
                                if (!textF.IsDummy && textF.Value != null && textF.Value.ValueType == AssetValueType.String) {
                                    string val = textF.AsString;
                                    if (dict.TryGetValue(val, out string translated)) {
                                        textF.AsString = translated;
                                        modified = true;
                                    }
                                }
                                
                                var textLowerF = baseField["text"];
                                if (!textLowerF.IsDummy && textLowerF.Value != null && textLowerF.Value.ValueType == AssetValueType.String) {
                                    string val = textLowerF.AsString;
                                    if (dict.TryGetValue(val, out string translated)) {
                                        textLowerF.AsString = translated;
                                        modified = true;
                                    }
                                }

                                if (modified)
                                {
                                    info.SetNewData(baseField.WriteToByteArray());
                                    fileModified = true;
                                }
                            }

                            foreach (var info in inst.file.GetAssetsOfType(49))
                            {
                                var baseField = manager.GetBaseField(inst, info);
                                if (baseField == null) continue;

                                var scriptF = baseField["m_Script"];
                                if (!scriptF.IsDummy && scriptF.Value != null) {
                                    string val = scriptF.AsString;
                                    if (dict.TryGetValue(val, out string translated)) {
                                        scriptF.AsString = translated;
                                        info.SetNewData(baseField.WriteToByteArray());
                                        fileModified = true;
                                    }
                                }
                            }

                            if (fileModified)
                            {
                                dirInfo.SetNewData(inst.file);
                                bunModified = true;
                            }
                        }
                        index++;
                    }

                    if (bunModified)
                    {
                        string backupPath = file + ".bak";
                        if (!File.Exists(backupPath)) File.Copy(file, backupPath);

                        string tempFile = file + ".temp";
                        using (var stream = File.Create(tempFile))
                        using (var writer = new AssetsFileWriter(stream))
                        {
                            bunInst.file.Write(writer);
                        }
                        bunInst.file.Reader.Close();
                        File.Delete(file);
                        File.Move(tempFile, file);
                        Console.WriteLine($"[C#] Injected into {Path.GetFileName(file)}");
                    }
                    else
                    {
                        bunInst.file.Reader.Close();
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[C#] Error modifying bundle file {file}: {ex.Message}");
                }
            }

            Console.WriteLine("[C#] Injection finished!");
        }
#endif
    }
}
