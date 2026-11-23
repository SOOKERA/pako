//! A file in the vault: encrypted as it is written, in 64 KiB blocks, so a download can pause,
//! resume and write with several connections at different places, like an ordinary file.
//!
//! Layout: a 256-byte header (`PAKOVLT1`, the file's random id, its final size once finished,
//! and the file's own random key sealed to the vault's public key), then one fixed-size record
//! per block: nonce (24, fresh and random at EVERY write, so writing a block again never reuses
//! one) || plain length (u32) || last-block flag (u8) || ciphertext + tag. The file id, the
//! block's number, its length and the flag are bound to each record (associated data), so
//! records can't be moved, swapped, cut short or taken from another file.
//!
//! Only whole blocks are written to the file. Pieces of a block that isn't complete yet (where
//! two connections meet, or the end of what arrived so far) wait in memory and are put in a
//! small encrypted side file (`.pending`) at every flush, so what a flush reported as written
//! is really on disk.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use super::crypto::{self, Key, NONCE, TAG};

pub const BLOCK: u64 = 64 * 1024;
pub const HEADER: u64 = 256;
const REC_HEAD: u64 = NONCE as u64 + 4 + 1;
pub const REC: u64 = REC_HEAD + BLOCK + TAG as u64;
const MAGIC: &[u8; 8] = b"PAKOVLT1";
const UNKNOWN: u64 = u64::MAX;

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

#[cfg(windows)]
fn write_at(f: &File, mut data: &[u8], mut off: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !data.is_empty() {
        let n = f.seek_write(data, off)?;
        if n == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        data = &data[n..];
        off += n as u64;
    }
    Ok(())
}

#[cfg(not(windows))]
fn write_at(f: &File, data: &[u8], off: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    f.write_all_at(data, off)
}

/// Reads as much as is there (a block never written reads as zeros or nothing).
#[cfg(windows)]
fn read_at(f: &File, buf: &mut [u8], off: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt;
    let mut got = 0;
    while got < buf.len() {
        match f.seek_read(&mut buf[got..], off + got as u64) {
            Ok(0) => break,
            Ok(n) => got += n,
            // Reading past the end on Windows.
            Err(e) if e.raw_os_error() == Some(38) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(got)
}

#[cfg(not(windows))]
fn read_at(f: &File, buf: &mut [u8], off: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    let mut got = 0;
    while got < buf.len() {
        match f.read_at(&mut buf[got..], off + got as u64)? {
            0 => break,
            n => got += n,
        }
    }
    Ok(got)
}

/// What the header says.
#[derive(Debug, Clone)]
pub struct Header {
    pub id: [u8; 16],
    /// The final size, once finished.
    pub size: Option<u64>,
    pub sealed_key: Vec<u8>,
}

/// A file's key is bound to its id and to the private list entry it belongs to (the file's own
/// name in the vault), so a file can't be passed off as another entry's.
fn key_aad(id: &[u8; 16], owner: &str) -> Vec<u8> {
    let mut a = b"pako file key".to_vec();
    a.extend_from_slice(id);
    a.extend_from_slice(owner.as_bytes());
    a
}

/// The list entry a vault file belongs to: its name up to the first dot ("<id>.pkv", and
/// "<id>.pkv.new" while it's being replaced).
pub fn owner_of(path: &Path) -> String {
    let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    name.split('.').next().unwrap_or_default().to_string()
}

fn rec_aad(id: &[u8; 16], idx: u64, len: u32, last: bool) -> [u8; 29] {
    let mut a = [0u8; 29];
    a[..16].copy_from_slice(id);
    a[16..24].copy_from_slice(&idx.to_le_bytes());
    a[24..28].copy_from_slice(&len.to_le_bytes());
    a[28] = last as u8;
    a
}

pub fn read_header(f: &File) -> io::Result<Header> {
    let mut h = [0u8; HEADER as usize];
    if read_at(f, &mut h, 0)? < HEADER as usize || &h[..8] != MAGIC {
        return Err(bad(&crate::tr!("Not a Pako private file", "Ce n’est pas un fichier privé de Pako")));
    }
    if u32::from_le_bytes(h[8..12].try_into().unwrap()) != 1 || u32::from_le_bytes(h[12..16].try_into().unwrap()) as u64 != BLOCK {
        return Err(bad(&crate::tr!("A private file from a newer Pako", "Un fichier privé d’un Pako plus récent")));
    }
    let id: [u8; 16] = h[16..32].try_into().unwrap();
    let size = u64::from_le_bytes(h[32..40].try_into().unwrap());
    let klen = u16::from_le_bytes(h[40..42].try_into().unwrap()) as usize;
    if 42 + klen > HEADER as usize {
        return Err(bad(&crate::tr!("Damaged private file", "Fichier privé endommagé")));
    }
    Ok(Header { id, size: (size != UNKNOWN).then_some(size), sealed_key: h[42..42 + klen].to_vec() })
}

pub fn read_header_at(path: &Path) -> io::Result<Header> {
    read_header(&File::open(path)?)
}

/// The file's own key, with the vault's secret key.
pub fn file_key(sk: &StaticSecret, h: &Header, owner: &str) -> Option<Key> {
    let k = crypto::unseal(sk, &key_aad(&h.id, owner), &h.sealed_key)?;
    let arr: [u8; 32] = k[..].try_into().ok()?;
    Some(Zeroizing::new(arr))
}

fn write_header(f: &File, id: &[u8; 16], size: Option<u64>, sealed_key: &[u8]) -> io::Result<()> {
    let mut h = [0u8; HEADER as usize];
    h[..8].copy_from_slice(MAGIC);
    h[8..12].copy_from_slice(&1u32.to_le_bytes());
    h[12..16].copy_from_slice(&(BLOCK as u32).to_le_bytes());
    h[16..32].copy_from_slice(id);
    h[32..40].copy_from_slice(&size.unwrap_or(UNKNOWN).to_le_bytes());
    h[40..42].copy_from_slice(&(sealed_key.len() as u16).to_le_bytes());
    h[42..42 + sealed_key.len()].copy_from_slice(sealed_key);
    write_at(f, &h, 0)
}

fn last_index(size: u64) -> u64 {
    if size == 0 { 0 } else { (size - 1) / BLOCK }
}

/// The plain length of block `idx` in a file of `size` bytes.
fn len_in(size: u64, idx: u64) -> usize {
    let last = last_index(size);
    if idx < last { BLOCK as usize } else if idx == last { (size - idx * BLOCK) as usize } else { 0 }
}

/// Decrypts one record as found on disk: None when the block was never written (or is damaged).
fn read_record(f: &File, key: &[u8; 32], id: &[u8; 16], idx: u64) -> io::Result<Option<(Zeroizing<Vec<u8>>, bool)>> {
    let mut buf = vec![0u8; REC as usize];
    let got = read_at(f, &mut buf, HEADER + idx * REC)?;
    if got < REC_HEAD as usize + TAG {
        return Ok(None);
    }
    let nonce: [u8; NONCE] = buf[..NONCE].try_into().unwrap();
    let len = u32::from_le_bytes(buf[NONCE..NONCE + 4].try_into().unwrap());
    let last = buf[NONCE + 4];
    if nonce == [0u8; NONCE] || len as u64 > BLOCK || last > 1 {
        return Ok(None);
    }
    let end = REC_HEAD as usize + len as usize + TAG;
    if got < end {
        return Ok(None);
    }
    let aad = rec_aad(id, idx, len, last == 1);
    Ok(crypto::decrypt_with(key, &nonce, &aad, &buf[REC_HEAD as usize..end]).map(|p| (p, last == 1)))
}

/// Pieces of a block not complete yet: its bytes and which ranges of it are filled.
struct Partial {
    data: Zeroizing<Vec<u8>>,
    have: Vec<(u32, u32)>,
}

impl Partial {
    fn new() -> Self {
        Partial { data: Zeroizing::new(vec![0u8; BLOCK as usize]), have: Vec::new() }
    }

    fn put(&mut self, at: usize, bytes: &[u8]) {
        self.data[at..at + bytes.len()].copy_from_slice(bytes);
        self.have.push((at as u32, (at + bytes.len()) as u32));
        self.have.sort();
        let mut merged: Vec<(u32, u32)> = Vec::new();
        for (a, b) in self.have.drain(..) {
            match merged.last_mut() {
                Some(m) if a <= m.1 => m.1 = m.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        self.have = merged;
    }

    /// Filled from 0 to at least `len`.
    fn covers(&self, len: usize) -> bool {
        len == 0 || self.have.first().is_some_and(|&(a, b)| a == 0 && b as usize >= len)
    }
}

/// A vault file being written (a download in progress).
pub struct Writer {
    file: File,
    id: [u8; 16],
    key: Key,
    sealed_key: Vec<u8>,
    /// The final size when known (the last block is marked as last when it is written).
    size: Mutex<Option<u64>>,
    pending: Mutex<BTreeMap<u64, Partial>>,
    pending_path: PathBuf,
}

fn pending_path(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".pending");
    PathBuf::from(p)
}

impl Writer {
    /// A new, empty vault file at `path` (replacing any). Its key is sealed to the vault's
    /// public key, so it can be made while the vault is locked.
    pub fn create(path: &Path, vault: &PublicKey, size: Option<u64>) -> io::Result<Writer> {
        let id = crypto::random::<16>();
        let key = crypto::random_key();
        let sealed_key = crypto::seal(vault, &key_aad(&id, &owner_of(path)), &key[..]);
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(path)?;
        write_header(&file, &id, None, &sealed_key)?;
        let _ = std::fs::remove_file(pending_path(path));
        let w = Writer { file, id, key, sealed_key, size: Mutex::new(None), pending: Mutex::new(BTreeMap::new()), pending_path: pending_path(path) };
        w.lay_out(size)?;
        Ok(w)
    }

    /// Opens an unfinished vault file again (to resume), with its key.
    pub fn open(path: &Path, key: Key) -> io::Result<Writer> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let h = read_header(&file)?;
        let pending_path = pending_path(path);
        let w = Writer { file, id: h.id, key, sealed_key: h.sealed_key, size: Mutex::new(None), pending: Mutex::new(BTreeMap::new()), pending_path };
        w.load_pending()?;
        Ok(w)
    }

    pub fn id(&self) -> [u8; 16] {
        self.id
    }

    /// The key (kept by the caller to open the file again while the vault is locked).
    pub fn key(&self) -> Key {
        self.key.clone()
    }

    /// The handle (for marking it sparse).
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Starts over with nothing written: room for `size` bytes (sparse), or none yet.
    pub fn lay_out(&self, size: Option<u64>) -> io::Result<()> {
        self.pending.lock().unwrap().clear();
        let _ = std::fs::remove_file(&self.pending_path);
        *self.size.lock().unwrap() = size;
        self.file.set_len(HEADER)?;
        if let Some(s) = size {
            self.file.set_len(HEADER + (last_index(s) + 1) * REC)?;
        }
        Ok(())
    }

    /// How long the file will be, for checking a resumed layout still fits.
    pub fn set_size(&self, size: Option<u64>) {
        *self.size.lock().unwrap() = size;
    }

    fn want(&self, idx: u64) -> (usize, bool) {
        match *self.size.lock().unwrap() {
            Some(s) => (len_in(s, idx), idx == last_index(s)),
            None => (BLOCK as usize, false),
        }
    }

    fn write_record(&self, idx: u64, plain: &[u8], last: bool) -> io::Result<()> {
        let nonce = crypto::random::<NONCE>();
        let aad = rec_aad(&self.id, idx, plain.len() as u32, last);
        let ct = crypto::encrypt_with(&self.key, &nonce, &aad, plain);
        let mut rec = Vec::with_capacity(REC_HEAD as usize + ct.len());
        rec.extend_from_slice(&nonce);
        rec.extend_from_slice(&(plain.len() as u32).to_le_bytes());
        rec.push(last as u8);
        rec.extend_from_slice(&ct);
        write_at(&self.file, &rec, HEADER + idx * REC)
    }

    /// Writes `data` at `offset` of the plain file (any size, any place).
    pub fn write_at(&self, mut data: &[u8], mut offset: u64) -> io::Result<()> {
        while !data.is_empty() {
            let idx = offset / BLOCK;
            let at = (offset % BLOCK) as usize;
            let n = data.len().min(BLOCK as usize - at);
            self.put(idx, at, &data[..n])?;
            data = &data[n..];
            offset += n as u64;
        }
        Ok(())
    }

    fn put(&self, idx: u64, at: usize, bytes: &[u8]) -> io::Result<()> {
        let (want, last) = self.want(idx);
        if want == 0 {
            return Err(bad(&crate::tr!("Wrote past the end of the file", "Écriture au-delà de la fin du fichier")));
        }
        let bytes = &bytes[..bytes.len().min(want.saturating_sub(at))];
        // Held while writing too, so a whole block and a piece of the same block can never cross
        // (encrypting 64 KiB takes microseconds; downloads don't notice).
        let mut pending = self.pending.lock().unwrap();
        if at == 0 && bytes.len() == want && !pending.contains_key(&idx) {
            return self.write_record(idx, bytes, last);
        }
        if !pending.contains_key(&idx) {
            // Written before (a resumed part that starts inside it): start from what's there.
            let mut p = Partial::new();
            if let Some((plain, _)) = read_record(&self.file, &self.key, &self.id, idx)? {
                p.put(0, &plain);
            }
            pending.insert(idx, p);
        }
        let p = pending.get_mut(&idx).unwrap();
        p.put(at, bytes);
        if p.covers(want) {
            let p = pending.remove(&idx).unwrap();
            self.write_record(idx, &p.data[..want], last)?;
        }
        Ok(())
    }

    fn pending_bytes(&self) -> Zeroizing<Vec<u8>> {
        let pending = self.pending.lock().unwrap();
        let mut out = Zeroizing::new(Vec::new());
        for (idx, p) in pending.iter() {
            out.extend_from_slice(&idx.to_le_bytes());
            out.extend_from_slice(&(p.have.len() as u32).to_le_bytes());
            for &(a, b) in &p.have {
                out.extend_from_slice(&a.to_le_bytes());
                out.extend_from_slice(&b.to_le_bytes());
                out.extend_from_slice(&p.data[a as usize..b as usize]);
            }
        }
        out
    }

    fn pending_aad(&self) -> Vec<u8> {
        let mut a = b"pako pending".to_vec();
        a.extend_from_slice(&self.id);
        a
    }

    fn load_pending(&self) -> io::Result<()> {
        let Ok(blob) = std::fs::read(&self.pending_path) else { return Ok(()) };
        let plain = crypto::decrypt(&self.key, &self.pending_aad(), &blob).ok_or_else(|| bad(&crate::tr!("The unfinished private file's last pieces are damaged", "Les derniers morceaux du fichier privé inachevé sont endommagés")))?;
        let mut pending = self.pending.lock().unwrap();
        let mut r = &plain[..];
        let take = |r: &mut &[u8], n: usize| -> io::Result<Vec<u8>> {
            if r.len() < n {
                return Err(bad(&crate::tr!("Damaged pieces", "Morceaux endommagés")));
            }
            let (a, b) = r.split_at(n);
            *r = b;
            Ok(a.to_vec())
        };
        while !r.is_empty() {
            let idx = u64::from_le_bytes(take(&mut r, 8)?.try_into().unwrap());
            let n = u32::from_le_bytes(take(&mut r, 4)?.try_into().unwrap());
            let mut p = Partial::new();
            for _ in 0..n {
                let a = u32::from_le_bytes(take(&mut r, 4)?.try_into().unwrap()) as usize;
                let b = u32::from_le_bytes(take(&mut r, 4)?.try_into().unwrap()) as usize;
                if a > b || b as u64 > BLOCK {
                    return Err(bad(&crate::tr!("Damaged pieces", "Morceaux endommagés")));
                }
                let bytes = Zeroizing::new(take(&mut r, b - a)?);
                p.put(a, &bytes);
            }
            // Written whole since (after that flush): the block on disk is the newer one.
            if read_record(&self.file, &self.key, &self.id, idx)?.is_some_and(|(b, _)| b.len() as u64 == BLOCK) {
                continue;
            }
            pending.insert(idx, p);
        }
        Ok(())
    }

    /// Puts everything written so far on disk (call before saving progress).
    pub fn sync(&self) -> io::Result<()> {
        let plain = self.pending_bytes();
        if plain.is_empty() {
            let _ = std::fs::remove_file(&self.pending_path);
        } else {
            let blob = crypto::encrypt(&self.key, &self.pending_aad(), &plain);
            crate::paths::write_atomic(&self.pending_path, &blob)?;
        }
        self.file.sync_data()
    }

    /// The download is complete at `total` bytes: writes the last pieces, marks the last block
    /// and records the size. Fails if a piece is missing.
    pub fn finish(&self, total: u64) -> io::Result<()> {
        *self.size.lock().unwrap() = Some(total);
        let last = last_index(total);
        let pending = std::mem::take(&mut *self.pending.lock().unwrap());
        let mut wrote_last = false;
        for (idx, p) in pending.iter() {
            if *idx > last {
                continue;
            }
            let want = len_in(total, *idx);
            if !p.covers(want) {
                return Err(bad(&crate::tr!("A piece of the file is missing", "Un morceau du fichier manque")));
            }
            self.write_record(*idx, &p.data[..want], *idx == last)?;
            wrote_last |= *idx == last;
        }
        if !wrote_last {
            let want = len_in(total, last);
            match read_record(&self.file, &self.key, &self.id, last)? {
                Some((plain, flag)) if plain.len() >= want => {
                    if !flag || plain.len() != want {
                        self.write_record(last, &plain[..want], true)?;
                    }
                }
                None if total == 0 => self.write_record(0, &[], true)?,
                _ => return Err(bad(&crate::tr!("The end of the file is missing", "La fin du fichier manque"))),
            }
        }
        write_header(&self.file, &self.id, Some(total), &self.sealed_key)?;
        self.file.set_len(HEADER + (last + 1) * REC)?;
        self.file.sync_all()?;
        let _ = std::fs::remove_file(&self.pending_path);
        Ok(())
    }
}

/// Deletes a vault file and its side file.
pub fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
    let pending = pending_path(path);
    let _ = std::fs::remove_file(&pending);
    // The copy `write_atomic` keeps of the one before.
    let _ = std::fs::remove_file(pending.with_extension("bak"));
}

/// A finished vault file, read back (decrypted block by block, every block checked).
pub struct Reader {
    file: File,
    id: [u8; 16],
    key: Key,
    size: u64,
    cache: Mutex<Option<(u64, Zeroizing<Vec<u8>>)>>,
}

impl Reader {
    pub fn open(path: &Path, key: Key) -> io::Result<Reader> {
        let file = File::open(path)?;
        let h = read_header(&file)?;
        let size = h.size.ok_or_else(|| bad(&crate::tr!("This private file isn't finished", "Ce fichier privé n’est pas terminé")))?;
        let r = Reader { file, id: h.id, key, size, cache: Mutex::new(None) };
        // Checked at once: the last block must be there and marked last (a file cut short, or
        // a size changed to 0, is refused before anything is read).
        r.block(last_index(size))?;
        Ok(r)
    }

    pub fn len(&self) -> u64 {
        self.size
    }

    /// One block, decrypted and checked (its place, length and whether it is the last).
    pub fn block(&self, idx: u64) -> io::Result<Zeroizing<Vec<u8>>> {
        if let Some((i, b)) = &*self.cache.lock().unwrap() {
            if *i == idx {
                return Ok(b.clone());
            }
        }
        let want = len_in(self.size, idx);
        let last = idx == last_index(self.size);
        let (plain, flag) = read_record(&self.file, &self.key, &self.id, idx)?.ok_or_else(|| bad(&crate::tr!("A piece of this private file is missing or damaged", "Un morceau de ce fichier privé manque ou est endommagé")))?;
        if plain.len() != want || flag != last {
            return Err(bad(&crate::tr!("A piece of this private file is missing or damaged", "Un morceau de ce fichier privé manque ou est endommagé")));
        }
        *self.cache.lock().unwrap() = Some((idx, plain.clone()));
        Ok(plain)
    }

    /// Reads into `buf` from `offset`; returns how many bytes (0 at the end).
    pub fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let mut got = 0;
        while got < buf.len() {
            let pos = offset + got as u64;
            if pos >= self.size {
                break;
            }
            let idx = pos / BLOCK;
            let at = (pos % BLOCK) as usize;
            let b = self.block(idx)?;
            let n = (b.len() - at).min(buf.len() - got);
            buf[got..got + n].copy_from_slice(&b[at..at + n]);
            got += n;
        }
        Ok(got)
    }

    /// Copies the whole plain file into `out` (an export or a temporary copy); `stop` is
    /// checked between blocks.
    pub fn copy_to(&self, out: &mut impl io::Write, stop: &dyn Fn() -> bool) -> io::Result<()> {
        let blocks = if self.size == 0 { 0 } else { last_index(self.size) + 1 };
        for idx in 0..blocks {
            if stop() {
                return Err(io::ErrorKind::Interrupted.into());
            }
            out.write_all(&self.block(idx)?)?;
        }
        Ok(())
    }
}

/// A Reader as a seekable stream (for the player and hashing).
pub struct Stream {
    pub reader: std::sync::Arc<Reader>,
    pub pos: u64,
}

impl io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl io::Seek for Stream {
    fn seek(&mut self, to: io::SeekFrom) -> io::Result<u64> {
        let len = self.reader.len() as i128;
        let next = match to {
            io::SeekFrom::Start(n) => n as i128,
            io::SeekFrom::End(d) => len + d as i128,
            io::SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if next < 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.pos = next as u64;
        Ok(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::crypto::new_pair;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pako-vault-test-{}-{}", std::process::id(), uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    fn data(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 31 % 251) as u8).collect()
    }

    fn read_all(path: &Path, sk: &StaticSecret) -> Vec<u8> {
        let h = read_header_at(path).unwrap();
        let r = Reader::open(path, file_key(sk, &h, &owner_of(path)).unwrap()).unwrap();
        let mut out = Vec::new();
        r.copy_to(&mut out, &|| false).unwrap();
        out
    }

    #[test]
    fn known_size_written_in_any_order_reads_back() {
        let (sk, pk) = new_pair();
        let p = tmp("a.pkv");
        let src = data(BLOCK as usize * 3 + 1234);
        let w = Writer::create(&p, &pk, Some(src.len() as u64)).unwrap();
        // Three "connections" with boundaries inside blocks, written in pieces, out of order.
        let cuts = [0usize, 70_000, 150_001, src.len()];
        for c in (0..3).rev() {
            let (a, b) = (cuts[c], cuts[c + 1]);
            for chunk in (a..b).step_by(9_000) {
                let e = (chunk + 9_000).min(b);
                w.write_at(&src[chunk..e], chunk as u64).unwrap();
            }
        }
        w.finish(src.len() as u64).unwrap();
        assert_eq!(read_all(&p, &sk), src);
        // Nothing readable in the file itself.
        let raw = std::fs::read(&p).unwrap();
        assert!(!raw.windows(64).any(|w| w == &src[1000..1064]));
    }

    #[test]
    fn unknown_size_streamed_then_finished() {
        let (sk, pk) = new_pair();
        for n in [0usize, 1, BLOCK as usize, BLOCK as usize * 2, BLOCK as usize * 2 + 7] {
            let p = tmp("s.pkv");
            let src = data(n);
            let w = Writer::create(&p, &pk, None).unwrap();
            for (i, c) in src.chunks(10_000).enumerate() {
                w.write_at(c, (i * 10_000) as u64).unwrap();
            }
            w.finish(n as u64).unwrap();
            assert_eq!(read_all(&p, &sk), src, "size {n}");
        }
    }

    #[test]
    fn pause_and_resume_keeps_the_half_blocks() {
        let (sk, pk) = new_pair();
        let p = tmp("r.pkv");
        let src = data(BLOCK as usize * 2 + 500);
        let total = src.len() as u64;
        let w = Writer::create(&p, &pk, Some(total)).unwrap();
        let key = w.key();
        // Two parts, each stopped in the middle of a block.
        w.write_at(&src[..40_000], 0).unwrap();
        w.write_at(&src[100_000..110_000], 100_000).unwrap();
        w.sync().unwrap();
        drop(w);
        // Pako restarts: reopen, go on from where each part stopped.
        let w = Writer::open(&p, key).unwrap();
        w.set_size(Some(total));
        w.write_at(&src[40_000..100_000], 40_000).unwrap();
        w.write_at(&src[110_000..], 110_000).unwrap();
        w.finish(total).unwrap();
        assert_eq!(read_all(&p, &sk), src);
    }

    #[test]
    fn rewriting_a_block_uses_a_new_nonce_and_still_reads() {
        let (sk, pk) = new_pair();
        let p = tmp("w.pkv");
        let src = data(BLOCK as usize);
        let w = Writer::create(&p, &pk, Some(BLOCK)).unwrap();
        w.write_at(&src, 0).unwrap();
        let first = std::fs::read(&p).unwrap()[HEADER as usize..HEADER as usize + NONCE].to_vec();
        // A retry writes part of it again.
        w.write_at(&src[100..2000], 100).unwrap();
        w.finish(BLOCK).unwrap();
        let second = std::fs::read(&p).unwrap()[HEADER as usize..HEADER as usize + NONCE].to_vec();
        assert_ne!(first, second);
        assert_eq!(read_all(&p, &sk), src);
    }

    #[test]
    fn swapped_or_cut_blocks_are_refused() {
        let (sk, pk) = new_pair();
        let p = tmp("x.pkv");
        let src = data(BLOCK as usize * 3);
        let w = Writer::create(&p, &pk, Some(src.len() as u64)).unwrap();
        w.write_at(&src, 0).unwrap();
        w.finish(src.len() as u64).unwrap();
        let raw = std::fs::read(&p).unwrap();
        let key = file_key(&sk, &read_header_at(&p).unwrap(), &owner_of(&p)).unwrap();
        // Swap blocks 0 and 1.
        let mut swapped = raw.clone();
        let (h, r) = (HEADER as usize, REC as usize);
        let b0 = raw[h..h + r].to_vec();
        let b1 = raw[h + r..h + 2 * r].to_vec();
        swapped[h..h + r].copy_from_slice(&b1);
        swapped[h + r..h + 2 * r].copy_from_slice(&b0);
        std::fs::write(&p, &swapped).unwrap();
        let rd = Reader::open(&p, key.clone()).unwrap();
        assert!(rd.block(0).is_err());
        // Cut short: the header says 2 blocks (or none); the last one isn't marked last.
        for size in [BLOCK * 2, 0] {
            let mut cut = raw.clone();
            cut[32..40].copy_from_slice(&size.to_le_bytes());
            std::fs::write(&p, &cut).unwrap();
            assert!(Reader::open(&p, key.clone()).is_err(), "size {size}");
        }
        // Renamed to another entry's name: its key no longer opens.
        std::fs::write(&p, &raw).unwrap();
        let other = p.with_file_name("someone-else.pkv");
        std::fs::copy(&p, &other).unwrap();
        assert!(file_key(&sk, &read_header_at(&other).unwrap(), &owner_of(&other)).is_none());
    }

    #[test]
    fn missing_piece_refuses_to_finish() {
        let (_sk, pk) = new_pair();
        let p = tmp("m.pkv");
        let w = Writer::create(&p, &pk, Some(BLOCK * 2)).unwrap();
        w.write_at(&data(1000), 0).unwrap();
        assert!(w.finish(BLOCK * 2).is_err());
    }

    #[test]
    fn stream_seeks() {
        use std::io::{Read, Seek, SeekFrom};
        let (sk, pk) = new_pair();
        let p = tmp("t.pkv");
        let src = data(200_000);
        let w = Writer::create(&p, &pk, Some(src.len() as u64)).unwrap();
        w.write_at(&src, 0).unwrap();
        w.finish(src.len() as u64).unwrap();
        let key = file_key(&sk, &read_header_at(&p).unwrap(), &owner_of(&p)).unwrap();
        let mut s = Stream { reader: std::sync::Arc::new(Reader::open(&p, key).unwrap()), pos: 0 };
        s.seek(SeekFrom::Start(130_000)).unwrap();
        let mut b = [0u8; 5000];
        s.read_exact(&mut b).unwrap();
        assert_eq!(&b[..], &src[130_000..135_000]);
    }
}
