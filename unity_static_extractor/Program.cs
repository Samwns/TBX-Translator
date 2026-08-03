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
            "m_OriginalText", "m_LocalizedText", "m_Tooltip", "m_Description"
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

            // A generic CSV/bytes TextAsset may be color/font/configuration
            // data. Only use raw lines when its name indicates dialogue/text.
            if (!IsLikelyTextAsset(name)) return;
            foreach (var line in script.Split('\n'))
            {
                var trimmed = line.Trim();
                if (!trimmed.StartsWith("\"") && IsValidText(trimmed)) results.Add(trimmed);
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


        static void Extract(string dataFolder, string jsonFile)
        {
            Console.WriteLine($"[C#] EXTRACT mode started for: {dataFolder}");
            HashSet<string> allTexts = new HashSet<string>();

            // Managed IL literals have no field/type metadata. Scanning every
            // ldstr mixes exception messages, keys and code into translations,
            // so strict mode intentionally limits itself to serialized assets.
            Console.WriteLine("[C#] Strict mode: skipping Assembly-CSharp.dll literals.");
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
