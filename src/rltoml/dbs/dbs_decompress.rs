/*
 RlToml: DBS decompression utilities
 Copyright (C) 2026 luvlsco

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

/// DBS decompression error.
#[derive(Debug, thiserror::Error)]
pub enum DbsError {
    /// I/O error during read or write.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// Malformed compressed data.
    #[error("{0}")]
    InvalidFormat(String),
}

/// Pre-decompress dword XOR.
const XOR_KEY: u32 = 0x89f4622d;

/// Two alternating per-u32 keys used in the post-decompress
/// decrypt layer, selected by KEY_PATTERN.
const KEY_A: u32 = 0x7190c70e;
const KEY_B: u32 = 0x499bf135;

/// Packed 25-bit pattern derived from XOR key-stream analysis.
/// Bit N (0-based) selects between KEY_A (1) and KEY_B (0).
/// The pattern is cycled in 5-entry windows advancing by 5
/// every 16-u32 block, giving a period of 80 positions.
const KEY_PATTERN: u32 = 0x01825D99;

/// XORs every u32 from offset 4 with the fixed pre-decompress key.
/// The first u32 (offset 0-3) is left untouched. Self-inverse.
pub fn apply_xor_layer(data: &mut [u8]) {
    let len = data.len();
    for i in (4..len).step_by(4) {
        if i + 4 > len {
            break;
        }
        let dw = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        let bytes = (dw ^ XOR_KEY).to_le_bytes();
        data[i] = bytes[0];
        data[i + 1] = bytes[1];
        data[i + 2] = bytes[2];
        data[i + 3] = bytes[3];
    }
}

/// LZSS decompressor. Header is 12 bytes; bytes 8-11 hold the
/// decompressed size (`dlen`). When `dlen` is zero the payload
/// is stored uncompressed.
///
/// Compressed stream after the header uses bit flags (LSB first):
/// 1 = literal byte, 0 = back-reference. A back-reference is two
/// bytes encoding offset (12 bits, max 4095) and length (4 bits,
/// range 2-17).
fn decompress_dbs(data: &[u8]) -> Result<Vec<u8>, DbsError> {
    if data.len() < 12 {
        return Err(DbsError::InvalidFormat("file too short for compression header".into()));
    }

    let dlen = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;

    if dlen == 0 {
        return Ok(data[12..].to_vec());
    }

    let mut out = vec![0u8; dlen];
    let mut out_pos = 0usize;
    let mut i = 12usize;
    let mut bit_cnt = 0u8;
    let mut flags = 0u8;

    while i < data.len() && out_pos < dlen {
        if bit_cnt == 0 {
            flags = data[i];
            i += 1;
            bit_cnt = 8;
            continue;
        }

        if (flags & 1) != 0 {
            out[out_pos] = data[i];
            out_pos += 1;
            i += 1;
        } else {
            if i + 1 >= data.len() {
                return Err(DbsError::InvalidFormat("unexpected end of compressed data".into()));
            }
            let b1 = data[i];
            let b2 = data[i + 1];
            i += 2;
            let offset = ((b2 as usize) << 4) | (b1 as usize >> 4);
            let len = (b1 & 0xF) as usize + 2;

            if offset == 0 || offset > out_pos || len > dlen - out_pos {
                return Err(DbsError::InvalidFormat("invalid LZ77 back-reference offset".into()));
            }

            for _ in 0..len {
                out[out_pos] = out[out_pos - offset];
                out_pos += 1;
            }
        }

        bit_cnt -= 1;
        flags >>= 1;
    }

    if out_pos != dlen {
        return Err(DbsError::InvalidFormat(
            "compressed data ended before the declared length".into(),
        ));
    }

    Ok(out)
}

/// Post-decompress decrypt layer. XORs each u32 with KEY_A or KEY_B,
/// chosen by a packed 25-bit `KEY_PATTERN` cycled in 5-entry windows
/// advancing every 16 u32s (period 80). Self-inverse.
pub fn encrypt_dbs(data: &mut [u8]) {
    for (j, byte) in data.iter_mut().enumerate() {
        *byte ^= key_for((j / 4) % 80).to_le_bytes()[j % 4];
    }
}

/// Full DBS-to-BIN pipeline: Remove XOR layer -> LZSS decompress -> decrypt.
/// Writes the resulting plaintext database to `output_path`.
pub fn dbs_to_bin(input_path: &str, output_path: &str, verbose: bool) -> Result<(), DbsError> {
    if verbose {
        eprintln!("Reading DBS file");
    }
    let mut data = std::fs::read(input_path)?;

    if verbose {
        eprintln!("Removing XOR layer");
    }
    apply_xor_layer(&mut data);

    if verbose {
        eprintln!("Decompressing");
    }
    let decompressed = decompress_dbs(&data)?;
    let mut data = decompressed;

    if verbose {
        eprintln!("Decrypting");
    }
    encrypt_dbs(&mut data);

    if verbose {
        eprintln!("Writing raw database binary");
    }
    std::fs::write(output_path, &data)?;

    Ok(())
}

/// Compresses decrypted internal DBS data into the three-word archive header
/// plus the flag-driven LZSS stream used by RealLive.
fn compress_dbs(data: &[u8]) -> Result<Vec<u8>, DbsError> {
    let mut stream = Vec::new();
    let mut chunk = Vec::with_capacity(9);
    let mut flags = 0u8;
    let mut count = 0u8;
    let mut pos = 0usize;
    while pos < data.len() {
        let (offset, length) = find_match(data, pos);
        if length >= 2 {
            chunk.push((((offset & 0xF) << 4) | (length - 2)) as u8);
            chunk.push((offset >> 4) as u8);
            pos += length;
        } else {
            flags |= 1 << count;
            chunk.push(data[pos]);
            pos += 1;
        }
        count += 1;
        if count == 8 {
            chunk.insert(0, flags);
            stream.extend_from_slice(&chunk);
            chunk.clear();
            flags = 0;
            count = 0;
        }
    }
    if count > 0 {
        chunk.insert(0, flags);
        stream.extend_from_slice(&chunk);
    }

    let compressed_length = u32::try_from(
        stream
            .len()
            .checked_add(8)
            .ok_or_else(|| DbsError::InvalidFormat("compressed DBS is too large".into()))?,
    )
    .map_err(|_| DbsError::InvalidFormat("compressed DBS is too large".into()))?;
    let decompressed_length = u32::try_from(data.len()).map_err(|_| DbsError::InvalidFormat("DBS data is too large".into()))?;
    let mut output = Vec::with_capacity(stream.len() + 12);
    output.extend_from_slice(&0u32.to_le_bytes());
    output.extend_from_slice(&compressed_length.to_le_bytes());
    output.extend_from_slice(&decompressed_length.to_le_bytes());
    output.extend_from_slice(&stream);
    Ok(output)
}

fn find_match(data: &[u8], pos: usize) -> (usize, usize) {
    let max_length = (data.len() - pos).min(17);
    if max_length < 2 || pos == 0 {
        return (0, 0);
    }

    let window_start = pos.saturating_sub(0xFFF);
    let mut best = (0usize, 0usize);
    for candidate in window_start..pos {
        let offset = pos - candidate;
        let mut length = 0usize;
        while length < max_length && data[candidate + (length % offset)] == data[pos + length] {
            length += 1;
        }
        if length > best.1 {
            best = (offset, length);
            if length == max_length {
                break;
            }
        }
    }
    best
}

/// Writes decrypted internal DBS data as an obfuscated `.dbs` archive.
pub fn write_bin_as_dbs(data: &[u8], output_path: &str, verbose: bool) -> Result<(), DbsError> {
    if verbose {
        println!("Encrypting DBS data");
    }
    let mut encrypted = data.to_vec();
    encrypt_dbs(&mut encrypted);

    if verbose {
        println!("Compressing DBS data");
    }
    let mut output = compress_dbs(&encrypted)?;
    apply_xor_layer(&mut output);

    if verbose {
        println!("Writing DBS file");
    }
    std::fs::write(output_path, output)?;

    Ok(())
}

fn key_for(p: usize) -> u32 {
    // map position p to a bit index in KEY_PATTERN:
    // blocks of 16 u32s each use a window of 5 bits,
    // cycling within the 25-bit pattern (5 windows of 5)
    let idx = ((p / 16) * 5 + (p % 16 % 5)) % 25;
    if (KEY_PATTERN >> idx) & 1 != 0 { KEY_A } else { KEY_B }
}

#[cfg(test)]
mod tests {
    use super::{apply_xor_layer, compress_dbs, decompress_dbs, encrypt_dbs};

    #[test]
    fn compression_round_trip() {
        let data = b"ABCDABCDABCDABCD database data ".repeat(16);
        let mut encrypted = data.clone();
        encrypt_dbs(&mut encrypted);
        let mut archive = compress_dbs(&encrypted).unwrap();
        assert_eq!(&archive[0..4], &[0, 0, 0, 0]);

        apply_xor_layer(&mut archive);
        apply_xor_layer(&mut archive);
        let mut decoded = decompress_dbs(&archive).unwrap();
        encrypt_dbs(&mut decoded);

        assert_eq!(decoded, data);
    }
}
