use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

/// Maximum allowed decompressed size for a ZIP entry (10 MB).
const MAX_DECOMPRESSED_SIZE: u64 = 10 * 1024 * 1024;

/// ZIP magic bytes ("PK").
const ZIP_MAGIC: [u8; 2] = [b'P', b'K'];

/// Check whether a file is a ZIP archive by reading its magic bytes.
pub fn is_zip<P: AsRef<Path>>(path: P) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut header = [0u8; 2];
    file.read_exact(&mut header).is_ok() && header == ZIP_MAGIC
}

/// Decompress the first entry of a ZIP archive into a `Cursor`.
///
/// Returns a reader over the decompressed content of the archive's first
/// entry. Per the FIGfont spec, only the first entry is relevant.
/// Decompressed size is capped at `MAX_DECOMPRESSED_SIZE` to prevent
/// zip bomb attacks.
pub fn decompress_zip<P: AsRef<Path>>(path: P) -> Result<Cursor<Vec<u8>>, std::io::Error> {
    let file = File::open(path)?;
    decompress_zip_reader(file)
}

/// Check whether a reader starts with ZIP magic bytes.
///
/// Reads exactly 2 bytes and seeks back to position 0 so the reader
/// remains usable for subsequent operations.
pub fn is_zip_reader<R: Read + Seek>(reader: &mut R) -> Result<bool, std::io::Error> {
    let mut header = [0u8; 2];
    let res = reader.read_exact(&mut header);
    reader.seek(SeekFrom::Start(0))?;
    match res {
        Ok(()) => Ok(header == ZIP_MAGIC),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e),
    }
}

/// Decompress the first entry of a ZIP archive from a reader.
///
/// The reader must implement `Seek` so the archive library can navigate
/// the ZIP central directory. Decompressed size is capped at
/// `MAX_DECOMPRESSED_SIZE` to prevent zip bomb attacks.
pub fn decompress_zip_reader<R: Read + Seek>(reader: R) -> Result<Cursor<Vec<u8>>, std::io::Error> {
    let mut archive = zip::ZipArchive::new(reader)?;
    let reader = archive.by_index(0)?;
    let mut buf = Vec::new();
    reader.take(MAX_DECOMPRESSED_SIZE).read_to_end(&mut buf)?;
    Ok(Cursor::new(buf))
}
