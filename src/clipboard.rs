use std::io::Write;

/// OSC 52 escape sequence for writing `data` to the terminal's clipboard.
///
/// Format: `ESC ] 52 ; c ; <base64(data)> ST` where ST is `\x1b\\` (canonical)
/// or `\x07` (BEL, widely accepted).
pub fn osc52_sequence(data: &str) -> String {
    let encoded = base64_encode(data.as_bytes());
    format!("\x1b]52;c;{encoded}\x1b\\")
}

/// Wrap an escape sequence for tmux DCS passthrough.
///
/// tmux は OSC 52 を既定で外側へ透過させないため、`ESC P tmux ; <escaped> ESC \`
/// の DCS で包む必要がある。内側の ESC は二重化する。
/// 受信側 tmux の `allow-passthrough on` が必要 (tmux >= 3.3 / 3.4+ で既定 on)。
pub fn wrap_for_tmux(seq: &str) -> String {
    let escaped = seq.replace('\x1b', "\x1b\x1b");
    format!("\x1bPtmux;{escaped}\x1b\\")
}

/// Build the OSC 52 payload. tmux 配下 (`$TMUX` が定義されている) では DCS でラップし、
/// tmux のフィルタを通り抜けて外側のターミナルに届くようにする。
pub fn clipboard_payload(data: &str, inside_tmux: bool) -> String {
    let raw = osc52_sequence(data);
    if inside_tmux { wrap_for_tmux(&raw) } else { raw }
}

/// Write the OSC 52 clipboard sequence to the given writer (typically stdout).
pub fn write_osc52<W: Write>(writer: &mut W, data: &str) -> std::io::Result<()> {
    let inside_tmux = std::env::var_os("TMUX").is_some();
    writer.write_all(clipboard_payload(data, inside_tmux).as_bytes())?;
    writer.flush()
}

fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(CHARSET[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARSET[((n >> 12) & 0x3f) as usize] as char);
        out.push(CHARSET[((n >> 6) & 0x3f) as usize] as char);
        out.push(CHARSET[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(CHARSET[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARSET[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(CHARSET[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARSET[((n >> 12) & 0x3f) as usize] as char);
        out.push(CHARSET[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn osc52_sequence_wraps_base64() {
        let s = osc52_sequence("hi");
        assert_eq!(s, "\x1b]52;c;aGk=\x1b\\");
    }

    #[test]
    fn osc52_sequence_handles_url() {
        let s = osc52_sequence("https://example.com/issues/1");
        assert!(s.starts_with("\x1b]52;c;"));
        assert!(s.ends_with("\x1b\\"));
    }

    #[test]
    fn wrap_for_tmux_doubles_escapes_and_wraps_in_dcs() {
        let inner = osc52_sequence("hi"); // \x1b]52;c;aGk=\x1b\\
        let wrapped = wrap_for_tmux(&inner);
        assert_eq!(
            wrapped,
            "\x1bPtmux;\x1b\x1b]52;c;aGk=\x1b\x1b\\\x1b\\"
        );
    }

    #[test]
    fn clipboard_payload_outside_tmux_is_raw() {
        let p = clipboard_payload("hi", false);
        assert_eq!(p, osc52_sequence("hi"));
    }

    #[test]
    fn clipboard_payload_inside_tmux_wraps() {
        let p = clipboard_payload("hi", true);
        assert!(p.starts_with("\x1bPtmux;"));
        assert!(p.ends_with("\x1b\\"));
        // 内側の ESC が二重化されていること
        assert!(p.contains("\x1b\x1b]52;c;aGk="));
    }
}
