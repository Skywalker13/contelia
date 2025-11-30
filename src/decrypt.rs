/* Contelia
 * Copyright (C) 2025  Mathieu Schroeter <mathieu@schroetersa.ch>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::fs::File;
use std::io::{Read, Result, Seek, SeekFrom};
use std::path::Path;

pub enum FileReader {
    Encrypted(DecryptedFile),
    Plain(File),
}

impl Read for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            FileReader::Encrypted(file) => file.read(buf),
            FileReader::Plain(file) => file.read(buf),
        }
    }
}

impl Seek for FileReader {
    fn seek(&mut self, pos: std::io::SeekFrom) -> Result<u64> {
        match self {
            FileReader::Encrypted(file) => file.seek(pos),
            FileReader::Plain(file) => file.seek(pos),
        }
    }
}

pub struct DecryptedFile {
    file: File,
    decrypted_header: Vec<u8>, // 512 bytes
    position: u64,
}

impl DecryptedFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;

        /* Read max 512 bytes */
        let mut header = vec![0u8; 512];
        let bytes_read = file.read(&mut header)?;
        header.truncate(bytes_read);

        let decrypted_header = decrypt_block(&header);
        file.seek(SeekFrom::Start(0))?;

        Ok(Self {
            file,
            decrypted_header,
            position: 0,
        })
    }
}

impl Read for DecryptedFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let start_pos = self.position as usize;
        let mut bytes_written = 0;

        /* Read from the decrypted space (max 512 bytes) */
        if start_pos < self.decrypted_header.len() {
            let end_pos = std::cmp::min(start_pos + buf.len(), self.decrypted_header.len());
            let len = end_pos - start_pos;
            buf[..len].copy_from_slice(&self.decrypted_header[start_pos..end_pos]);
            bytes_written = len;
            self.position += len as u64;
        }

        /* Read after 512 bytes (already plain) */
        if bytes_written < buf.len() {
            /* Set position to (512 + offset) */
            let file_offset = 512 + (self.position - self.decrypted_header.len() as u64);
            self.file.seek(SeekFrom::Start(file_offset))?;

            let n = self.file.read(&mut buf[bytes_written..])?;
            self.position += n as u64;
            bytes_written += n;
        }

        Ok(bytes_written)
    }
}

impl Seek for DecryptedFile {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.position as i64 + n,
            SeekFrom::End(n) => {
                /* Total size */
                let file_size = self.file.metadata()?.len();
                file_size as i64 + n
            }
        };

        self.position = new_pos.max(0) as u64;
        Ok(self.position)
    }
}

fn btea_decrypt(v: &mut [u32], k: &[u32; 4]) {
    let n = v.len();
    if n < 2 {
        return;
    }

    const DELTA: u32 = 0x9E3779B9;

    /* WARNING: Lunii is using 1+52/n instead of 6+52/n
     * See https://github.com/marian-m12l/studio/issues/292#issuecomment-1157586816
     */
    let rounds = 1 + 52 / n;
    let mut sum = (rounds as u32).wrapping_mul(DELTA);
    let mut y = v[0];

    for _ in 0..rounds {
        let e = (sum >> 2) & 3;

        for p in (1..n).rev() {
            let z = v[p - 1];
            let mx = (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
                ^ ((sum ^ y).wrapping_add(k[(((p as u32) & 3) ^ e) as usize] ^ z));
            y = v[p].wrapping_sub(mx);
            v[p] = y;
        }

        let z = v[n - 1];
        let mx = (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4)))
            ^ ((sum ^ y).wrapping_add(k[((0 & 3) ^ e) as usize] ^ z));
        y = v[0].wrapping_sub(mx);
        v[0] = y;

        sum = sum.wrapping_sub(DELTA);
    }
}

pub(super) fn decrypt_block(bytes: &Vec<u8>) -> Vec<u8> {
    use byteorder::{ByteOrder, LittleEndian};

    /* Original key (big-endian):
     * 0x91, 0xBD, 0x7A, 0x0A, 0xA7, 0x54, 0x40, 0xA9,
     * 0xBB, 0xD4, 0x9D, 0x6C, 0xE0, 0xDC, 0xC0, 0xE3,
     * See https://github.com/marian-m12l/studio/blob/028912d9ee06e77bff679abd31701aa493f5461a/core/src/main/java/studio/core/v1/utils/XXTEACipher.java
     */
    const KEY: [u32; 4] = [0x91BD7A0A, 0xA75440A9, 0xBBD49D6C, 0xE0DCC0E3];

    /* Only the first 512 bytes are encrypted */
    let block_size = std::cmp::min(512, bytes.len());
    let aligned_size = (block_size / 4) * 4;
    if aligned_size < 4 {
        return bytes.to_vec();
    }

    /* little-endian data */
    let int_count = aligned_size / 4;
    let mut v = vec![0u32; int_count];
    LittleEndian::read_u32_into(&bytes[0..aligned_size], &mut v);

    /* (max 128 u32) */
    let n = std::cmp::min(128, int_count);
    btea_decrypt(&mut v[0..n], &KEY);

    /* Convert to little-endian */
    let mut result = vec![0u8; aligned_size];
    LittleEndian::write_u32_into(&v, &mut result);

    if bytes.len() > aligned_size {
        result.extend_from_slice(&bytes[aligned_size..]);
    }

    result
}
