# TPG Translator - API Module Documentation

**Creator:** samwns

## Overview

The `api.rs` module handles communication with external translation services.
Currently supports Google Translate (free GTX endpoint) and has a stub for
Google Apps Script (Turbo) integration.

## Functions

### `translate_batch`
```rust
pub async fn translate_batch(
    client: &Client,
    text_block: &str,
    _url: &str,
    from_lang: &str,
    to_lang: &str,
) -> Result<Vec<String>, String>
```
Translates a newline-delimited block of text line-by-line using the Google
Translate GTX endpoint. Each line is URL-encoded and sent as an individual
request. Returns a vector of translated strings in the same order.

**Error behavior:** On HTTP 429 (rate limit), returns `Err("IP Blocked")`.
On individual line failures, falls back to the original text.

### `get_lang_code`
```rust
pub fn get_lang_code(name: &str) -> &'static str
```
Maps human-readable language names (e.g. `"Português"`) to ISO 639-1 codes
(e.g. `"pt"`). Returns `"auto"` for unrecognized names.

## Supported Languages

| Name                  | Code   |
|-----------------------|--------|
| Alemão                | de     |
| Chinês (Simplificado) | zh-CN  |
| Coreano               | ko     |
| Espanhol              | es     |
| Francês               | fr     |
| Inglês                | en     |
| Italiano              | it     |
| Japonês               | ja     |
| Português             | pt     |
| Russo                 | ru     |
| Detectar Automaticamente | auto |
