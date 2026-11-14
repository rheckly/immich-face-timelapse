//! Minimal ZIP writer without compression ("stored"): the photos are JPEGs,
//! compressing them again gains nothing. No ZIP64, so at most 65 535 files
//! and 4 GB.

use chrono::{DateTime, Datelike, Timelike, Utc};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// CRC-32 (IEEE), as ZIP needs it.
fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *v = c;
        }
        t
    });
    !data
        .iter()
        .fold(!0u32, |c, &b| table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8))
}

/// MS-DOS time and date of a file.
fn dos_time(t: DateTime<Utc>) -> (u16, u16) {
    let year = t.year().clamp(1980, 2107) as u16;
    let date = ((year - 1980) << 9) | ((t.month() as u16) << 5) | t.day() as u16;
    let time = ((t.hour() as u16) << 11) | ((t.minute() as u16) << 5) | (t.second() as u16 / 2);
    (time, date)
}

/// Fields shared by the local and the central header, from "flags" to
/// "name length".
fn common(h: &mut Vec<u8>, time: u16, date: u16, crc: u32, size: u32, name_len: u16) {
    h.extend_from_slice(&0x0800u16.to_le_bytes()); // flags: names in UTF-8
    h.extend_from_slice(&0u16.to_le_bytes()); // method: stored
    h.extend_from_slice(&time.to_le_bytes());
    h.extend_from_slice(&date.to_le_bytes());
    h.extend_from_slice(&crc.to_le_bytes());
    h.extend_from_slice(&size.to_le_bytes()); // compressed
    h.extend_from_slice(&size.to_le_bytes()); // uncompressed
    h.extend_from_slice(&name_len.to_le_bytes());
}

/// Write `files` (name in the archive, file on disk) to `out`.
pub fn write_stored(out: &Path, files: &[(String, PathBuf)]) -> io::Result<()> {
    let too_big = || io::Error::other("ZIP would be larger than 4 GB");
    let count = u16::try_from(files.len()).map_err(|_| io::Error::other("too many files for a ZIP"))?;
    let mut w = BufWriter::new(std::fs::File::create(out)?);
    let mut offset: u64 = 0;
    let mut central = Vec::new();
    for (name, path) in files {
        let data = std::fs::read(path)?;
        let modified = std::fs::metadata(path)?
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        let (time, date) = dos_time(modified);
        let crc = crc32(&data);
        let size = u32::try_from(data.len()).map_err(|_| too_big())?;
        let at = u32::try_from(offset).map_err(|_| too_big())?;
        let n = name.as_bytes();
        let name_len = u16::try_from(n.len()).map_err(|_| io::Error::other("file name too long"))?;

        let mut h = Vec::with_capacity(30 + n.len());
        h.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        h.extend_from_slice(&20u16.to_le_bytes()); // version needed
        common(&mut h, time, date, crc, size, name_len);
        h.extend_from_slice(&0u16.to_le_bytes()); // extra length
        h.extend_from_slice(n);
        w.write_all(&h)?;
        w.write_all(&data)?;

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        common(&mut central, time, date, crc, size, name_len);
        // extra, comment, disk, internal attributes (2 each), external (4)
        central.extend_from_slice(&[0u8; 12]);
        central.extend_from_slice(&at.to_le_bytes());
        central.extend_from_slice(n);
        offset += (h.len() + data.len()) as u64;
    }
    let cd_at = u32::try_from(offset).map_err(|_| too_big())?;
    let cd_len = u32::try_from(central.len()).map_err(|_| too_big())?;
    u32::try_from(offset + u64::from(cd_len) + 22).map_err(|_| too_big())?;
    w.write_all(&central)?;
    let mut e = Vec::with_capacity(22);
    e.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    e.extend_from_slice(&[0u8; 4]); // disk numbers
    e.extend_from_slice(&count.to_le_bytes());
    e.extend_from_slice(&count.to_le_bytes());
    e.extend_from_slice(&cd_len.to_le_bytes());
    e.extend_from_slice(&cd_at.to_le_bytes());
    e.extend_from_slice(&0u16.to_le_bytes()); // comment length
    w.write_all(&e)?;
    w.into_inner().map_err(|e| e.into_error())?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_reference() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn writes_readable_layout() {
        let dir = std::env::temp_dir().join(format!("zip-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.jpg");
        std::fs::write(&a, b"hello").unwrap();
        let out = dir.join("out.zip");
        write_stored(&out, &[("a.jpg".into(), a)]).unwrap();
        let z = std::fs::read(&out).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        // local header + name + data, central header + name, end record
        assert_eq!(z.len(), 30 + 5 + 5 + 46 + 5 + 22);
        assert_eq!(&z[..4], b"PK\x03\x04");
        assert_eq!(&z[35..40], b"hello");
        let end = &z[z.len() - 22..];
        assert_eq!(&end[..4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([end[10], end[11]]), 1);
        let cd_at = u32::from_le_bytes([end[16], end[17], end[18], end[19]]) as usize;
        assert_eq!(cd_at, 40);
        assert_eq!(&z[cd_at..cd_at + 4], b"PK\x01\x02");
    }
}
