use std::io::{self, Write};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const MAX_OSC52_BYTES: usize = 100_000;

#[derive(Default)]
pub struct Clipboard {
    text: String,
}

impl Clipboard {
    pub fn set(&mut self, text: String) {
        if let Some(sequence) = osc52_sequence(&text) {
            let mut stdout = io::stdout();
            let _ = stdout.write_all(sequence.as_bytes());
            let _ = stdout.flush();
        }
        self.text = text;
    }

    pub fn get(&self) -> &str {
        &self.text
    }
}

pub fn osc52_sequence(text: &str) -> Option<String> {
    if text.is_empty() || text.len() > MAX_OSC52_BYTES {
        return None;
    }
    Some(format!("\x1b]52;c;{}\x07", encode_base64(text.as_bytes())))
}

pub fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_reference_vectors() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
        assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn osc52_wraps_base64_payload() {
        assert_eq!(osc52_sequence("foo").as_deref(), Some("\x1b]52;c;Zm9v\x07"));
    }

    #[test]
    fn osc52_skips_empty_and_oversized_text() {
        assert!(osc52_sequence("").is_none());
        assert!(osc52_sequence(&"a".repeat(MAX_OSC52_BYTES + 1)).is_none());
    }
}
