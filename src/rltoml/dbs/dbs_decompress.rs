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
fn remove_dbs_xor_layer(data: &mut [u8]) {
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

			if offset > out_pos {
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

	Ok(out)
}

/// Post-decompress decrypt layer. XORs each u32 with KEY_A or KEY_B,
/// chosen by a packed 25-bit `KEY_PATTERN` cycled in 5-entry windows
/// advancing every 16 u32s (period 80). Self-inverse.
fn decrypt_dbs(data: &mut [u8]) {
	for (i, chunk) in data.chunks_exact_mut(4).enumerate() {
		let p = i % 80;
		// map position p to a bit index in KEY_PATTERN:
		// blocks of 16 u32s each use a window of 5 bits,
		// cycling within the 25-bit pattern (5 windows of 5)
		let idx = ((p / 16) * 5 + (p % 16 % 5)) % 25;
		let key = if (KEY_PATTERN >> idx) & 1 != 0 { KEY_A } else { KEY_B };
		let dw = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
		let bytes = (dw ^ key).to_le_bytes();
		chunk[0] = bytes[0];
		chunk[1] = bytes[1];
		chunk[2] = bytes[2];
		chunk[3] = bytes[3];
	}

	let whole_u32_len = (data.len() / 4) * 4;
	for j in whole_u32_len..data.len() {
		let p = (j / 4) % 80;
		let idx = ((p / 16) * 5 + (p % 16 % 5)) % 25;
		let key = if (KEY_PATTERN >> idx) & 1 != 0 { KEY_A } else { KEY_B };
		data[j] ^= key.to_le_bytes()[j % 4];
	}
}

/// Full DBS-to-BIN pipeline: Remove XOR layer -> LZSS decompress -> decrypt.
/// Writes the resulting plaintext database to `output_path`.
pub fn dbs_to_bin(input_path: &str, output_path: &str, verbose: bool) -> Result<(), DbsError> {
	if verbose { eprintln!("Reading DBS file"); }
	let mut data = std::fs::read(input_path)?;

	if verbose { eprintln!("Removing XOR layer"); }
	remove_dbs_xor_layer(&mut data);

	if verbose { eprintln!("Decompressing"); }
	let decompressed = decompress_dbs(&data)?;
	let mut data = decompressed;

	if verbose { eprintln!("Decrypting"); }
	decrypt_dbs(&mut data);

	if verbose { eprintln!("Writing raw database binary"); }
	std::fs::write(output_path, &data)?;

	Ok(())
}
