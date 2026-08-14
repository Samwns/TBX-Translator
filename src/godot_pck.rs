use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PckFileEntry {
    pub path: String,
    pub offset: u64,
    pub size: u64,
    #[allow(dead_code)]
    pub md5: [u8; 16],
    #[allow(dead_code)]
    pub flags: u32,
}

#[derive(Debug)]
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
    reader.read_exact(&mut magic).map_err(|e| format!("Falha ao ler magic: {}", e))?;

    let mut header_offset = 0;
    if &magic != b"GDPC" {
        reader.seek(SeekFrom::End(-4)).map_err(|e| e.to_string())?;
        reader.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic == b"GDPC" {
            reader.seek(SeekFrom::End(-12)).map_err(|e| e.to_string())?;
            let mut offset_bytes = [0u8; 8];
            reader.read_exact(&mut offset_bytes).map_err(|e| e.to_string())?;
            let pck_size = u64::from_le_bytes(offset_bytes);
            let pck_start = reader.seek(SeekFrom::End(-12 - (pck_size as i64))).map_err(|e| e.to_string())?;
            reader.read_exact(&mut magic).map_err(|e| e.to_string())?;
            if &magic != b"GDPC" {
                return Err("PCK embutido inválido".into());
            }
            header_offset = pck_start;
        } else {
            return Err("Arquivo não é um PCK válido (sem GDPC)".into());
        }
    }

    let mut u32_buf = [0u8; 4];
    let mut u64_buf = [0u8; 8];

    reader.read_exact(&mut u32_buf).unwrap();
    let format_version = u32::from_le_bytes(u32_buf);

    reader.read_exact(&mut u32_buf).unwrap(); // major
    reader.read_exact(&mut u32_buf).unwrap(); // minor
    reader.read_exact(&mut u32_buf).unwrap(); // patch

    let mut pack_flags = 0;
    let mut file_base = 0;

    if format_version >= 2 {
        reader.read_exact(&mut u32_buf).unwrap();
        pack_flags = u32::from_le_bytes(u32_buf);
        reader.read_exact(&mut u64_buf).unwrap();
        file_base = u64::from_le_bytes(u64_buf);
    }

    // rel_filebase flag for V2+
    let rel_filebase = (pack_flags & 2) != 0;
    if format_version == 4 || format_version == 3 || (format_version == 2 && rel_filebase) {
        file_base += header_offset;
    }

    if format_version == 4 || format_version == 3 {
        // V3/V4: Read directory offset and skip reserved part
        reader.read_exact(&mut u64_buf).unwrap();
        let dir_offset = u64::from_le_bytes(u64_buf) + header_offset;

        let enc_directory = (pack_flags & 1) != 0;
        let sparse_bundle = (pack_flags & 4) != 0;

        if sparse_bundle && enc_directory && format_version == 4 {
            // skip salt
            let mut salt_data = [0u8; 32];
            reader.read_exact(&mut salt_data).unwrap();
        }

        reader.seek(SeekFrom::Start(dir_offset)).unwrap();
    } else if format_version == 2 {
        // V2: Directory after header, skip 16 ints (64 bytes)
        reader.seek(SeekFrom::Current(64)).unwrap();
    } else {
        // V1 (Godot 3): no pack_flags or file_base. Just 16 reserved 32-bit integers.
        reader.seek(SeekFrom::Current(64)).unwrap();
    }

    reader.read_exact(&mut u32_buf).unwrap();
    let file_count = u32::from_le_bytes(u32_buf);

    let mut files = Vec::new();
    for _ in 0..file_count {
        reader.read_exact(&mut u32_buf).unwrap();
        let str_len = u32::from_le_bytes(u32_buf) as usize;

        let mut str_bytes = vec![0u8; str_len];
        reader.read_exact(&mut str_bytes).unwrap();

        let mut end = str_len;
        while end > 0 && str_bytes[end - 1] == 0 {
            end -= 1;
        }
        let path = String::from_utf8_lossy(&str_bytes[..end]).into_owned();

        reader.read_exact(&mut u64_buf).unwrap();
        let mut offset = u64::from_le_bytes(u64_buf);
        if format_version < 3 {
            offset += header_offset;
        }
        offset += file_base; // add file_base!

        reader.read_exact(&mut u64_buf).unwrap();
        let size = u64::from_le_bytes(u64_buf);

        let mut md5 = [0u8; 16];
        reader.read_exact(&mut md5).unwrap();

        let mut flags = 0;
        if format_version >= 2 {
            reader.read_exact(&mut u32_buf).unwrap();
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

    file.write_all(&format_version.to_le_bytes()).unwrap();
    file.write_all(&4u32.to_le_bytes()).unwrap(); // major 4
    file.write_all(&1u32.to_le_bytes()).unwrap(); // minor 1
    file.write_all(&0u32.to_le_bytes()).unwrap(); // patch 0
    file.write_all(&pack_flags.to_le_bytes()).unwrap(); // flags

    // header_size calculation
    let mut header_size: u64 = 4 + 4 + 4 + 4 + 4 + 4 + 8 + 16 + 4;

    for (path, _) in files_to_add {
        let mut path_bytes = path.clone().into_bytes();
        path_bytes.push(0);
        let padding = 4 - (path_bytes.len() % 4);
        let padding = if padding == 4 { 0 } else { padding };
        let str_len = path_bytes.len() + padding;

        header_size += 4 + (str_len as u64) + 8 + 8 + 16 + 4;
    }

    let file_base = header_size;
    file.write_all(&file_base.to_le_bytes()).unwrap();
    file.write_all(&[0u8; 16]).unwrap(); // Padding reserved

    let file_count = files_to_add.len() as u32;
    file.write_all(&file_count.to_le_bytes()).unwrap();

    let mut current_offset = file_base;
    let mut file_data = Vec::new();

    for (path, data) in files_to_add {
        let mut path_bytes = path.clone().into_bytes();
        path_bytes.push(0);
        let padding = 4 - (path_bytes.len() % 4);
        let padding = if padding == 4 { 0 } else { padding };
        for _ in 0..padding { path_bytes.push(0); }

        let str_len = path_bytes.len() as u32;
        file.write_all(&str_len.to_le_bytes()).unwrap();
        file.write_all(&path_bytes).unwrap();

        file.write_all(&current_offset.to_le_bytes()).unwrap();

        let size = data.len() as u64;
        file.write_all(&size.to_le_bytes()).unwrap();

        // md5
        file.write_all(&[0u8; 16]).unwrap();

        // flags
        file.write_all(&0u32.to_le_bytes()).unwrap();

        current_offset += size;
        file_data.extend_from_slice(data);
    }

    file.seek(SeekFrom::Start(file_base)).unwrap();
    file.write_all(&file_data).unwrap();

    Ok(())
}
