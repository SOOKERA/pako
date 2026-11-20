//! The vault's building blocks: authenticated encryption (XChaCha20-Poly1305, a fresh random
//! 24-byte nonce for every message, so writing the same block again never reuses one), sealing
//! to the vault's public key (X25519, so Pako can put things in while the vault is locked), and
//! the slow key from a PIN, password or recovery code (Argon2id).

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

/// A 256-bit key, wiped from memory when dropped.
pub type Key = Zeroizing<[u8; 32]>;

pub const NONCE: usize = 24;
pub const TAG: usize = 16;
/// What sealing to the public key adds in front: the one-time public key.
pub const SEAL_EXTRA: usize = 32 + NONCE + TAG;

pub fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    OsRng.fill_bytes(&mut b);
    b
}

pub fn random_key() -> Key {
    Zeroizing::new(random::<32>())
}

/// Encrypts `plain` with `key`: nonce || ciphertext+tag. `aad` is bound to it (checked, not stored).
pub fn encrypt(key: &[u8; 32], aad: &[u8], plain: &[u8]) -> Vec<u8> {
    let nonce = random::<NONCE>();
    let mut out = Vec::with_capacity(NONCE + plain.len() + TAG);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&encrypt_with(key, &nonce, aad, plain));
    out
}

/// Encrypts with a nonce the caller made fresh (random) for this message.
pub fn encrypt_with(key: &[u8; 32], nonce: &[u8; NONCE], aad: &[u8], plain: &[u8]) -> Vec<u8> {
    let cipher = XChaCha20Poly1305::new(key.into());
    // Only fails for messages far bigger than anything Pako encrypts in one piece.
    cipher.encrypt(XNonce::from_slice(nonce), Payload { msg: plain, aad }).expect("encrypt")
}

/// The other way round; None when the key is wrong or anything was changed.
pub fn decrypt(key: &[u8; 32], aad: &[u8], blob: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if blob.len() < NONCE + TAG {
        return None;
    }
    let (nonce, ct) = blob.split_at(NONCE);
    decrypt_with(key, nonce.try_into().ok()?, aad, ct)
}

pub fn decrypt_with(key: &[u8; 32], nonce: &[u8; NONCE], aad: &[u8], ct: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher.decrypt(XNonce::from_slice(nonce), Payload { msg: ct, aad }).ok().map(Zeroizing::new)
}

/// A new key pair for a vault.
pub fn new_pair() -> (StaticSecret, PublicKey) {
    let sk = StaticSecret::random_from_rng(OsRng);
    let pk = PublicKey::from(&sk);
    (sk, pk)
}

fn seal_key(shared: &[u8; 32], one_time: &[u8; 32], to: &[u8; 32]) -> Key {
    let mut h = blake3::Hasher::new_derive_key("Pako vault seal v1");
    h.update(shared);
    h.update(one_time);
    h.update(to);
    Zeroizing::new(*h.finalize().as_bytes())
}

/// Seals `plain` so only the holder of the vault's secret key can open it: one-time public key
/// || nonce || ciphertext+tag.
pub fn seal(to: &PublicKey, aad: &[u8], plain: &[u8]) -> Vec<u8> {
    let one_time = StaticSecret::random_from_rng(OsRng);
    let one_time_pk = PublicKey::from(&one_time);
    let shared = one_time.diffie_hellman(to);
    let key = seal_key(shared.as_bytes(), one_time_pk.as_bytes(), to.as_bytes());
    let mut out = Vec::with_capacity(SEAL_EXTRA + plain.len());
    out.extend_from_slice(one_time_pk.as_bytes());
    out.extend_from_slice(&encrypt(&key, aad, plain));
    out
}

pub fn unseal(sk: &StaticSecret, aad: &[u8], blob: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if blob.len() < SEAL_EXTRA {
        return None;
    }
    let one_time: [u8; 32] = blob[..32].try_into().ok()?;
    let one_time_pk = PublicKey::from(one_time);
    let shared = sk.diffie_hellman(&one_time_pk);
    if !shared.was_contributory() {
        return None;
    }
    let pk = PublicKey::from(sk);
    let key = seal_key(shared.as_bytes(), &one_time, pk.as_bytes());
    decrypt(&key, aad, &blob[32..])
}

/// How slow the key from a PIN, password or recovery code is to make (kept with each wrap, so
/// it can be raised later without breaking older vaults).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cost {
    /// Memory in KiB.
    pub m: u32,
    /// Passes.
    pub t: u32,
    /// Lanes.
    pub p: u32,
}

/// About half a second per try on the PC Pako is built on (256 MiB, 5 passes: 0.55 s
/// measured 2026-11-19; 64 MiB and 3 passes took only 0.07 s). Slower PCs take longer.
#[cfg(not(test))]
pub const COST: Cost = Cost { m: 256 * 1024, t: 5, p: 1 };
/// Tests stay quick; the code path is the same.
#[cfg(test)]
pub const COST: Cost = Cost { m: 256, t: 1, p: 1 };

/// The most a stored cost may ask for (a changed file could otherwise make Pako use gigabytes
/// or minutes on every try).
pub fn cost_ok(c: Cost) -> bool {
    (8..=1024 * 1024).contains(&c.m) && (1..=16).contains(&c.t) && (1..=4).contains(&c.p)
}

/// The key from a PIN, password or recovery code (Argon2id).
pub fn stretch(secret: &[u8], salt: &[u8], cost: Cost) -> Option<Key> {
    if !cost_ok(cost) {
        return None;
    }
    let params = Params::new(cost.m, cost.t, cost.p, Some(32)).ok()?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon.hash_password_into(secret, salt, out.as_mut()).ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_round_trip_and_tamper() {
        let key = random_key();
        let blob = encrypt(&key, b"aad", b"hello");
        assert_eq!(&decrypt(&key, b"aad", &blob).unwrap()[..], b"hello");
        assert!(decrypt(&key, b"other", &blob).is_none(), "aad is bound");
        let mut bad = blob.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(decrypt(&key, b"aad", &bad).is_none(), "changed bytes are refused");
        assert!(decrypt(&random_key(), b"aad", &blob).is_none(), "wrong key");
    }

    #[test]
    fn same_message_twice_uses_new_nonces() {
        let key = random_key();
        let a = encrypt(&key, b"", b"same");
        let b = encrypt(&key, b"", b"same");
        assert_ne!(a[..NONCE], b[..NONCE]);
        assert_ne!(a, b);
    }

    #[test]
    fn sealed_only_opens_with_the_secret_key() {
        let (sk, pk) = new_pair();
        let blob = seal(&pk, b"rec", b"private list entry");
        assert_eq!(&unseal(&sk, b"rec", &blob).unwrap()[..], b"private list entry");
        let (other, _) = new_pair();
        assert!(unseal(&other, b"rec", &blob).is_none());
        assert!(unseal(&sk, b"x", &blob).is_none());
        assert!(unseal(&sk, b"rec", &blob[..10]).is_none());
    }

    #[test]
    fn stretch_depends_on_secret_and_salt() {
        let a = stretch(b"1234", b"salt-salt-salt-1", COST).unwrap();
        assert_eq!(*a, *stretch(b"1234", b"salt-salt-salt-1", COST).unwrap());
        assert_ne!(*a, *stretch(b"1235", b"salt-salt-salt-1", COST).unwrap());
        assert_ne!(*a, *stretch(b"1234", b"salt-salt-salt-2", COST).unwrap());
    }
}
