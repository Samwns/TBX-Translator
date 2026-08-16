use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PckFileEntry {
    pub path: String,
    pub offset: u64,
    pub size: u64,
    #[allow(dead_code)]
    pub md5: [u8; 16],
    #[allow(dead_code)]
    pub flags: u32,
}

#[derive(Debug, Clone)]
pub struct PckArchive {
    #[allow(dead_code)]
    pub format_version: u32,
    #[allow(dead_code)]
    pub pack_flags: u32,
    #[allow(dead_code)]
    pub file_base: u64,
    pub files: Vec<PckFileEntry>,
}

pub fn read_pck_header<R: Read + Seek>(mut reader: R) -> Result<PckArchive, String> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(|e| format!("Falha ao ler magic inicial: {}", e))?;

    let mut header_offset = 0;
    if &magic != b"GDPC" {
        // Tenta encontrar GDPC embutido no final do arquivo (executáveis empacotados com PCK)
        reader.seek(SeekFrom::End(-4)).map_err(|e| format!("Falha ao posicionar busca final: {}", e))?;
        reader.read_exact(&mut magic).map_err(|e| format!("Falha ao ler magic final: {}", e))?;
        if &magic == b"GDPC" {
            reader.seek(SeekFrom::End(-12)).map_err(|e| format!("Falha ao ler tamanho de PCK embutido: {}", e))?;
            let mut offset_bytes = [0u8; 8];
            reader.read_exact(&mut offset_bytes).map_err(|e| format!("Falha ao ler bytes do offset: {}", e))?;
            let pck_size = u64::from_le_bytes(offset_bytes);
            let pck_start = reader.seek(SeekFrom::End(-12 - (pck_size as i64))).map_err(|e| format!("Falha ao posicionar início do PCK embutido: {}", e))?;
            reader.read_exact(&mut magic).map_err(|e| format!("Falha ao confirmar magic do PCK embutido: {}", e))?;
            if &magic != b"GDPC" {
                return Err("PCK embutido inválido".into());
            }
            header_offset = pck_start;
        } else {
            return Err("Arquivo não é um PCK válido do Godot (magic GDPC não encontrado)".into());
        }
    }

    let mut u32_buf = [0u8; 4];
    let mut u64_buf = [0u8; 8];

    reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler format_version do PCK: {}", e))?;
    let format_version = u32::from_le_bytes(u32_buf);

    reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler engine_major do PCK: {}", e))?; // major
    reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler engine_minor do PCK: {}", e))?; // minor
    reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler engine_patch do PCK: {}", e))?; // patch

    let mut pack_flags = 0;
    let mut file_base = 0;

    if format_version >= 2 {
        reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler pack_flags do PCK: {}", e))?;
        pack_flags = u32::from_le_bytes(u32_buf);
        reader.read_exact(&mut u64_buf).map_err(|e| format!("Erro ao ler file_base do PCK: {}", e))?;
        file_base = u64::from_le_bytes(u64_buf);
    }

    // rel_filebase flag for V2+
    let rel_filebase = (pack_flags & 2) != 0;
    if format_version == 4 || format_version == 3 || (format_version == 2 && rel_filebase) {
        file_base += header_offset;
    }

    if format_version == 4 || format_version == 3 {
        // V3/V4: Read directory offset and skip reserved part
        reader.read_exact(&mut u64_buf).map_err(|e| format!("Erro ao ler dir_offset do PCK: {}", e))?;
        let dir_offset = u64::from_le_bytes(u64_buf) + header_offset;

        let enc_directory = (pack_flags & 1) != 0;
        let sparse_bundle = (pack_flags & 4) != 0;

        if sparse_bundle && enc_directory && format_version == 4 {
            // skip salt
            let mut salt_data = [0u8; 32];
            reader.read_exact(&mut salt_data).map_err(|e| format!("Erro ao ler salt do PCK: {}", e))?;
        }

        reader.seek(SeekFrom::Start(dir_offset)).map_err(|e| format!("Erro ao buscar diretório em dir_offset: {}", e))?;
    } else if format_version == 2 {
        // V2: Directory after header, skip 16 ints (64 bytes)
        reader.seek(SeekFrom::Current(64)).map_err(|e| format!("Erro ao pular reserved header V2: {}", e))?;
    } else {
        // V1 (Godot 3): no pack_flags or file_base. Just 16 reserved 32-bit integers.
        reader.seek(SeekFrom::Current(64)).map_err(|e| format!("Erro ao pular reserved header V1: {}", e))?;
    }

    reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler file_count do PCK: {}", e))?;
    let file_count = u32::from_le_bytes(u32_buf);

    if file_count > 500_000 {
        return Err(format!("Quantidade de arquivos inválida ou corrompida no PCK ({file_count})"));
    }

    let mut files = Vec::with_capacity(file_count.min(10_000) as usize);
    for i in 0..file_count {
        reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler tamanho do caminho do arquivo #{i}: {e}"))?;
        let str_len = u32::from_le_bytes(u32_buf) as usize;

        if str_len > 4096 {
            return Err(format!("Tamanho de caminho anormal ({str_len} bytes) no arquivo #{i} do PCK"));
        }

        let mut str_bytes = vec![0u8; str_len];
        reader.read_exact(&mut str_bytes).map_err(|e| format!("Erro ao ler caminho do arquivo #{i}: {e}"))?;

        let mut end = str_len;
        while end > 0 && str_bytes[end - 1] == 0 {
            end -= 1;
        }
        let path = String::from_utf8_lossy(&str_bytes[..end]).into_owned();

        reader.read_exact(&mut u64_buf).map_err(|e| format!("Erro ao ler offset do arquivo #{i}: {e}"))?;
        let mut offset = u64::from_le_bytes(u64_buf);
        if format_version < 3 {
            offset += header_offset;
        }
        offset += file_base; // add file_base!

        reader.read_exact(&mut u64_buf).map_err(|e| format!("Erro ao ler tamanho do arquivo #{i}: {e}"))?;
        let size = u64::from_le_bytes(u64_buf);

        let mut md5 = [0u8; 16];
        reader.read_exact(&mut md5).map_err(|e| format!("Erro ao ler md5 do arquivo #{i}: {e}"))?;

        let mut flags = 0;
        if format_version >= 2 {
            reader.read_exact(&mut u32_buf).map_err(|e| format!("Erro ao ler flags do arquivo #{i}: {e}"))?;
            flags = u32::from_le_bytes(u32_buf);
        }

        files.push(PckFileEntry {
            path,
            offset,
            size,
            md5,
            flags,
        });
    }

    Ok(PckArchive {
        format_version,
        pack_flags,
        file_base,
        files,
    })
}

pub fn create_patch_pck(target_pck_path: &Path, files_to_add: &HashMap<String, Vec<u8>>) -> Result<(), String> {
    let mut file = File::create(target_pck_path).map_err(|e| format!("Erro criando PCK de patch: {}", e))?;

    let format_version = 2u32;
    let pack_flags = 0u32;

    file.write_all(b"GDPC").map_err(|e| e.to_string())?;

    file.write_all(&format_version.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&4u32.to_le_bytes()).map_err(|e| e.to_string())?; // major 4
    file.write_all(&1u32.to_le_bytes()).map_err(|e| e.to_string())?; // minor 1
    file.write_all(&0u32.to_le_bytes()).map_err(|e| e.to_string())?; // patch 0
    file.write_all(&pack_flags.to_le_bytes()).map_err(|e| e.to_string())?; // flags

    // header_size calculation:
    // magic(4) + format(4) + major(4) + minor(4) + patch(4) + flags(4) + file_base(8) + reserved(64) + file_count(4)
    let mut header_size: u64 = 4 + 4 + 4 + 4 + 4 + 4 + 8 + 64 + 4;

    for (path, _) in files_to_add {
        let mut path_bytes = path.clone().into_bytes();
        path_bytes.push(0);
        let padding = 4 - (path_bytes.len() % 4);
        let padding = if padding == 4 { 0 } else { padding };
        let str_len = path_bytes.len() + padding;

        header_size += 4 + (str_len as u64) + 8 + 8 + 16 + 4;
    }

    let file_base = header_size;
    file.write_all(&file_base.to_le_bytes()).map_err(|e| e.to_string())?;
    file.write_all(&[0u8; 64]).map_err(|e| e.to_string())?; // 16 inteiros reservados (64 bytes)

    let file_count = files_to_add.len() as u32;
    file.write_all(&file_count.to_le_bytes()).map_err(|e| e.to_string())?;

    let mut current_offset = file_base;
    let mut file_data = Vec::new();

    for (path, data) in files_to_add {
        let mut path_bytes = path.clone().into_bytes();
        path_bytes.push(0);
        let padding = 4 - (path_bytes.len() % 4);
        let padding = if padding == 4 { 0 } else { padding };
        for _ in 0..padding { path_bytes.push(0); }

        let str_len = path_bytes.len() as u32;
        file.write_all(&str_len.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&path_bytes).map_err(|e| e.to_string())?;

        file.write_all(&current_offset.to_le_bytes()).map_err(|e| e.to_string())?;

        let size = data.len() as u64;
        file.write_all(&size.to_le_bytes()).map_err(|e| e.to_string())?;

        // md5
        file.write_all(&[0u8; 16]).map_err(|e| e.to_string())?;

        // flags
        file.write_all(&0u32.to_le_bytes()).map_err(|e| e.to_string())?;

        current_offset += size;
        file_data.extend_from_slice(data);
    }

    file.seek(SeekFrom::Start(file_base)).map_err(|e| e.to_string())?;
    file.write_all(&file_data).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_rejects_empty_and_corrupt_data_without_panicking() {
        // Buffer vazio
        let empty = Cursor::new(Vec::new());
        assert!(read_pck_header(empty).is_err());

        // Dados aleatórios
        let random = Cursor::new(vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        assert!(read_pck_header(random).is_err());

        // Magic GDPC truncado
        let mut truncated = Vec::new();
        truncated.extend_from_slice(b"GDPC");
        truncated.extend_from_slice(&2u32.to_le_bytes()); // format 2
        let reader = Cursor::new(truncated);
        assert!(read_pck_header(reader).is_err());
    }

    #[test]
    fn test_patch_pck_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("tbx-pck-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        let patch_path = temp_dir.join("test_patch.pck");

        let mut files = HashMap::new();
        files.insert("res://test.txt".to_string(), b"Ola Mundo Godot".to_vec());
        files.insert("res://dialogue.json".to_string(), b"{\"msg\": \"ola\"}".to_vec());

        assert!(create_patch_pck(&patch_path, &files).is_ok());

        let mut file = File::open(&patch_path).unwrap();
        let archive = read_pck_header(&mut file).unwrap();
        assert_eq!(archive.files.len(), 2);
        assert!(archive.files.iter().any(|f| f.path == "res://test.txt"));
        assert!(archive.files.iter().any(|f| f.path == "res://dialogue.json"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
