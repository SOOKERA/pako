//! The recovery code: 8 groups of 4 easy-to-read characters (no 0/O, 1/I/L). The first 7 groups
//! are random (28 characters from 31, about 138 bits); the last group is a check of the others,
//! so a typo is caught before anything is tried.

use zeroize::Zeroizing;

const ALPHA: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
const RANDOM: usize = 28;

fn check(random: &[u8]) -> [u8; 4] {
    let h = blake3::Hasher::new_derive_key("Pako recovery check v1").update(random).finalize();
    let b = h.as_bytes();
    [ALPHA[b[0] as usize % 31], ALPHA[b[1] as usize % 31], ALPHA[b[2] as usize % 31], ALPHA[b[3] as usize % 31]]
}

/// A new code, written in groups ("K7QM-3FXA-…").
pub fn new_code() -> Zeroizing<String> {
    let mut chars = Zeroizing::new(Vec::with_capacity(RANDOM + 4));
    while chars.len() < RANDOM {
        for b in super::crypto::random::<32>() {
            // 248 = 8 × 31: no value comes up more often than another.
            if b < 248 && chars.len() < RANDOM {
                chars.push(ALPHA[(b % 31) as usize]);
            }
        }
    }
    let c = check(&chars);
    chars.extend_from_slice(&c);
    let mut out = Zeroizing::new(String::with_capacity(39));
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push('-');
        }
        out.push(*ch as char);
    }
    out
}

/// The part the vault key is wrapped with (the random 28 characters), from what the user typed:
/// spaces, dashes and lower case are fine. None when it isn't a valid code (a typo).
pub fn secret_of(typed: &str) -> Option<Zeroizing<Vec<u8>>> {
    let chars: Zeroizing<Vec<u8>> = Zeroizing::new(
        typed.chars().filter(|c| !c.is_whitespace() && *c != '-').map(|c| if c.is_ascii() { c.to_ascii_uppercase() as u8 } else { 0 }).collect(),
    );
    if chars.len() != RANDOM + 4 || !chars.iter().all(|c| ALPHA.contains(c)) {
        return None;
    }
    if check(&chars[..RANDOM]) != chars[RANDOM..] {
        return None;
    }
    Some(Zeroizing::new(chars[..RANDOM].to_vec()))
}

/// The last group (typed back to confirm the code was kept).
pub fn last_group(code: &str) -> String {
    code.rsplit('-').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_back_and_catch_typos() {
        let code = new_code();
        assert_eq!(code.len(), 39);
        assert_eq!(code.split('-').count(), 8);
        assert!(!code.contains(['0', 'O', '1', 'I', 'L']));
        let s = secret_of(&code).unwrap();
        assert_eq!(s.len(), 28);
        // Lower case and spaces instead of dashes are fine.
        assert_eq!(secret_of(&code.to_lowercase().replace('-', " ")).unwrap()[..], s[..]);
        // One character changed: caught.
        let mut bad: Vec<char> = code.chars().collect();
        bad[0] = if bad[0] == 'A' { 'B' } else { 'A' };
        assert!(secret_of(&bad.into_iter().collect::<String>()).is_none());
        assert!(secret_of("1234").is_none());
        assert_eq!(last_group(&code).len(), 4);
    }

    #[test]
    fn codes_differ() {
        assert_ne!(*new_code(), *new_code());
    }
}
