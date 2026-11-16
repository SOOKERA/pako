// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Release tool (run from a terminal, never by users):
//!   cargo run -- --release keygen            makes the signing key (once), prints the public key
//!   cargo run -- --release sign EXE VERSION [NOTES]   writes latest.json next to EXE
//!
//! The private key lives in %USERPROFILE%\.pako-signing\ (or PAKO_SIGNING_DIR), never in the
//! project. Losing it means users can't take updates signed with a new key: keep a backup.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::updates::{Manifest, signed_message, signed_message_v2};

fn key_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("PAKO_SIGNING_DIR") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".pako-signing")
}

fn key_file() -> PathBuf {
    key_dir().join("pako-signing.key")
}

pub fn main(args: Vec<String>) -> i32 {
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let r = match rest.as_slice() {
        ["keygen"] => keygen(),
        ["sign", exe, version] => sign(Path::new(exe), version, ""),
        ["sign", exe, version, notes] => sign(Path::new(exe), version, notes),
        _ => Err("Use: --release keygen | --release sign EXE VERSION [NOTES]".into()),
    };
    match r {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn keygen() -> Result<String, String> {
    let file = key_file();
    if file.exists() {
        return Err(format!("A signing key already exists at {} (not replaced).", file.display()));
    }
    std::fs::create_dir_all(key_dir()).map_err(|e| e.to_string())?;
    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let key = SigningKey::from_bytes(&seed);
    let public = B64.encode(key.verifying_key().to_bytes());
    std::fs::write(&file, B64.encode(seed)).map_err(|e| e.to_string())?;
    std::fs::write(key_dir().join("pako-signing.pub"), &public).map_err(|e| e.to_string())?;
    Ok(format!("Private key: {}\nPublic key (put in updates.rs): {public}", file.display()))
}

fn sign(exe: &Path, version: &str, notes: &str) -> Result<String, String> {
    let seed: [u8; 32] = B64
        .decode(std::fs::read_to_string(key_file()).map_err(|e| format!("No signing key ({e}). Run --release keygen first."))?.trim())
        .map_err(|e| e.to_string())?
        .try_into()
        .map_err(|_| "The signing key file is damaged.".to_string())?;
    let key = SigningKey::from_bytes(&seed);
    let bytes = std::fs::read(exe).map_err(|e| format!("Can't read {}: {e}", exe.display()))?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let size = bytes.len() as u64;
    // Both kinds: the first for Pako versions up to 0.1.1, the second (size and notes too) for newer ones.
    let signature = B64.encode(key.sign(signed_message(version, &sha256).as_bytes()).to_bytes());
    let signature2 = B64.encode(key.sign(signed_message_v2(version, &sha256, size, notes).as_bytes()).to_bytes());
    let m = Manifest { version: version.into(), sha256, size, notes: notes.into(), signature, signature2 };
    crate::updates::verify(&m).map_err(|e| format!("Signed, but the app's public key doesn't match this key: {e}"))?;
    let out = exe.with_file_name("latest.json");
    std::fs::write(&out, serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(format!("Wrote {}", out.display()))
}
