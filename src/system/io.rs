use anyhow::{bail, Context, Result};
use encoding_rs::{
    Encoding, BIG5, EUC_JP, EUC_KR, GBK, ISO_8859_2, ISO_8859_5, KOI8_R, SHIFT_JIS, UTF_16BE,
    UTF_16LE, UTF_8, WINDOWS_1250, WINDOWS_1251, WINDOWS_1252,
};
use ropey::Rope;
use std::{fs, path::Path};

pub struct LoadedFile {
    pub rope: Rope,
    pub encoding: &'static Encoding,
    pub bom: bool,
    pub line_ending: &'static str,
}

pub struct FileHandler;

impl FileHandler {
    pub fn supported_encodings() -> Vec<&'static Encoding> {
        vec![
            UTF_8,
            UTF_16LE,
            UTF_16BE,
            WINDOWS_1252,
            WINDOWS_1250,
            WINDOWS_1251,
            ISO_8859_2,
            ISO_8859_5,
            KOI8_R,
            SHIFT_JIS,
            EUC_JP,
            EUC_KR,
            GBK,
            BIG5,
        ]
    }

    pub fn read_file(path: &Path) -> Result<LoadedFile> {
        let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        let (text, encoding, bom) = Self::decode(&bytes);
        Ok(LoadedFile {
            line_ending: Self::detect_line_ending(&text),
            rope: Rope::from_str(&text),
            encoding,
            bom,
        })
    }

    pub fn write_file(
        path: &Path,
        rope: &Rope,
        encoding: &'static Encoding,
        bom: bool,
    ) -> Result<()> {
        let bytes = Self::encode(&rope.to_string(), encoding, bom)?;
        fs::write(path, bytes).with_context(|| format!("cannot write {}", path.display()))
    }

    fn decode(bytes: &[u8]) -> (String, &'static Encoding, bool) {
        if let Some((encoding, bom_len)) = Encoding::for_bom(bytes) {
            let (text, _) = encoding.decode_without_bom_handling(&bytes[bom_len..]);
            return (text.into_owned(), encoding, true);
        }

        match std::str::from_utf8(bytes) {
            Ok(text) => (text.to_owned(), UTF_8, false),
            Err(_) => {
                let (text, _) = WINDOWS_1252.decode_without_bom_handling(bytes);
                (text.into_owned(), WINDOWS_1252, false)
            }
        }
    }

    fn encode(content: &str, encoding: &'static Encoding, bom: bool) -> Result<Vec<u8>> {
        if encoding == UTF_16LE {
            let mut output = vec![0xFF, 0xFE];
            for unit in content.encode_utf16() {
                output.extend_from_slice(&unit.to_le_bytes());
            }
            return Ok(output);
        }

        if encoding == UTF_16BE {
            let mut output = vec![0xFE, 0xFF];
            for unit in content.encode_utf16() {
                output.extend_from_slice(&unit.to_be_bytes());
            }
            return Ok(output);
        }

        let (bytes, _, had_unmappable) = encoding.encode(content);
        if had_unmappable {
            bail!(
                "text contains characters {} cannot represent",
                encoding.name()
            );
        }

        let mut output = Vec::with_capacity(bytes.len() + 3);
        if bom && encoding == UTF_8 {
            output.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        output.extend_from_slice(&bytes);
        Ok(output)
    }

    fn detect_line_ending(text: &str) -> &'static str {
        match text.find('\n') {
            Some(index) if index > 0 && text.as_bytes()[index - 1] == b'\r' => "\r\n",
            _ => "\n",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("terracl-io-{}-{}", std::process::id(), name))
    }

    #[test]
    fn decodes_plain_utf8() {
        let (text, encoding, bom) = FileHandler::decode(b"abc");
        assert_eq!(text, "abc");
        assert_eq!(encoding, UTF_8);
        assert!(!bom);
    }

    #[test]
    fn decodes_utf8_bom() {
        let (text, encoding, bom) = FileHandler::decode(&[0xEF, 0xBB, 0xBF, b'a']);
        assert_eq!(text, "a");
        assert_eq!(encoding, UTF_8);
        assert!(bom);
    }

    #[test]
    fn decodes_utf16_little_endian_bom() {
        let (text, encoding, bom) = FileHandler::decode(&[0xFF, 0xFE, b'h', 0, b'i', 0]);
        assert_eq!(text, "hi");
        assert_eq!(encoding, UTF_16LE);
        assert!(bom);
    }

    #[test]
    fn decodes_utf16_big_endian_bom() {
        let (text, encoding, _) = FileHandler::decode(&[0xFE, 0xFF, 0, b'h', 0, b'i']);
        assert_eq!(text, "hi");
        assert_eq!(encoding, UTF_16BE);
    }

    #[test]
    fn invalid_utf8_falls_back_to_windows_1252() {
        let (text, encoding, bom) = FileHandler::decode(&[b'c', b'a', b'f', 0xE9]);
        assert_eq!(text, "caf\u{e9}");
        assert_eq!(encoding, WINDOWS_1252);
        assert!(!bom);
    }

    #[test]
    fn encodes_utf16_with_bom() {
        let bytes = FileHandler::encode("hi", UTF_16LE, false).unwrap();
        assert_eq!(bytes, vec![0xFF, 0xFE, b'h', 0, b'i', 0]);
        let bytes = FileHandler::encode("hi", UTF_16BE, false).unwrap();
        assert_eq!(bytes, vec![0xFE, 0xFF, 0, b'h', 0, b'i']);
    }

    #[test]
    fn encodes_utf8_bom_only_when_requested() {
        assert_eq!(FileHandler::encode("a", UTF_8, false).unwrap(), vec![b'a']);
        assert_eq!(
            FileHandler::encode("a", UTF_8, true).unwrap(),
            vec![0xEF, 0xBB, 0xBF, b'a']
        );
    }

    #[test]
    fn encodes_shift_jis() {
        let bytes = FileHandler::encode("\u{65e5}", SHIFT_JIS, false).unwrap();
        assert_eq!(bytes, vec![0x93, 0xFA]);
    }

    #[test]
    fn rejects_unmappable_characters() {
        assert!(FileHandler::encode("\u{65e5}\u{672c}", WINDOWS_1252, false).is_err());
    }

    #[test]
    fn detects_line_endings() {
        assert_eq!(FileHandler::detect_line_ending("a\r\nb"), "\r\n");
        assert_eq!(FileHandler::detect_line_ending("a\nb"), "\n");
        assert_eq!(FileHandler::detect_line_ending(""), "\n");
        assert_eq!(FileHandler::detect_line_ending("\nb"), "\n");
    }

    #[test]
    fn file_round_trip_keeps_encoding_and_line_endings() {
        let path = temp_path("round-trip.txt");
        let rope = Rope::from_str("h\u{e9}llo\r\nworld");
        FileHandler::write_file(&path, &rope, WINDOWS_1252, false).unwrap();
        let loaded = FileHandler::read_file(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(loaded.rope.to_string(), "h\u{e9}llo\r\nworld");
        assert_eq!(loaded.encoding, WINDOWS_1252);
        assert_eq!(loaded.line_ending, "\r\n");
        assert!(!loaded.bom);
    }

    #[test]
    fn failed_encoding_leaves_existing_file_untouched() {
        let path = temp_path("untouched.txt");
        fs::write(&path, "original").unwrap();
        let rope = Rope::from_str("\u{65e5}");
        assert!(FileHandler::write_file(&path, &rope, WINDOWS_1252, false).is_err());
        let content = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(content, "original");
    }

    #[test]
    fn reading_a_missing_file_fails() {
        assert!(FileHandler::read_file(&temp_path("missing.txt")).is_err());
    }

    #[test]
    fn encoding_list_starts_with_utf8_and_has_no_duplicates() {
        let encodings = FileHandler::supported_encodings();
        assert_eq!(encodings[0], UTF_8);
        for (index, encoding) in encodings.iter().enumerate() {
            assert!(!encodings[..index].contains(encoding));
        }
    }
}
