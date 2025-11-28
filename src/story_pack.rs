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

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use std::{
    fs::{self},
    path::Path,
};

// Node: 44 bytes
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Node {
    image_asset_index: i32,              /* Index in ri (-1 if not used) */
    sound_asset_index: i32,              /* Index in si (-1 if not used) */
    ok_transition_action_index: i32,     /* Index in li (-1 if not used) */
    ok_transition_options_count: i32,    /* -1 if not used */
    ok_transition_selected_option: i32,  /* -1 if not used */
    home_transition_action_index: i32,   /* Index in li (-1 if not used) */
    home_transition_options_count: i32,  /* -1 if not used */
    home_transition_selected_count: i32, /* -1 if not used */
    control_wheel_enabled: u16,          /* wheel enabled (0 for no) */
    control_ok_enabled: u16,             /* OK enabled (0 for no) */
    control_home_enabled: u16,           /* HOME enabled (0 for no) */
    control_pause_enabled: u16,          /* Pause enabled (0 for no) */
    control_autoplay_enabled: u16,       /* Autoplayback (0 for no) */
    padding: u16,
}

// Node Index: 512 bytes
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct NiHeader {
    format_version: u16,     /* File format version: always 1 */
    story_version: u16,      /* Story pack version */
    nodes_list_offset: u32,  /* Offset for the nodes (should be 0x200) */
    node_size: u32,          /* Node size (should be 0x2C) */
    stage_nodes_count: u32,  /* Number of stage nodes */
    image_assets_count: u32, /* Number of images */
    sound_assets_count: u32, /* Number of sounds */
    factory_disabled: u8,    /* Factory pack if different of 0 */
    padding: [u8; 487],
}

pub struct Ni {
    pub header: NiHeader,
    pub nodes: Vec<Node>,
}

struct Li {
    list: Vec<u32>,
}

struct Ri {
    list: Vec<[u8; 12]>,
}

struct Si {
    list: Vec<[u8; 12]>,
}

pub struct StoryPack {
    ni: Ni,
    li: Li,
    ri: Ri,
    si: Si,
}

impl Ni {
    fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        let header: NiHeader = *bytemuck::from_bytes(&bytes[..512]);

        let nodes_offset = header.nodes_list_offset as usize;
        let node_bytes = &bytes[nodes_offset..];
        let nodes_slice: &[Node] = bytemuck::cast_slice(node_bytes);
        let nodes = nodes_slice[..header.stage_nodes_count as usize].to_vec();

        Ok(Ni { header, nodes })
    }
}

impl Li {
    fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        let decrypted = decrypt_block(&bytes);
        let list: Vec<u32> = bytemuck::cast_slice(&decrypted).to_vec();

        Ok(Li { list })
    }
}

impl Ri {
    fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        let decrypted = decrypt_block(&bytes);
        let list: Vec<[u8; 12]> = bytemuck::cast_slice(&decrypted).to_vec();

        Ok(Ri { list })
    }
}

impl Si {
    fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        let decrypted = decrypt_block(&bytes);
        let list: Vec<[u8; 12]> = bytemuck::cast_slice(&decrypted).to_vec();

        Ok(Si { list })
    }
}

/*
impl StoryPack {
    pub fn from_directory(path: &Path) -> Result<Self> {
        let ni = Ni::from_file(&path.join("ni"))?;
        let li = Li::from_file(&path.join("li"))?;
        let ri = Ri::from_file(&path.join("ri"))?;
        let si = Si::from_file(&path.join("si"))?;

        Ok(StoryPack { ni, li, ri, si })
    }
}
*/

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

fn decrypt_block(bytes: &Vec<u8>) -> Vec<u8> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_ni() {
        assert_eq!(std::mem::size_of::<Node>(), 44);
        assert_eq!(std::mem::size_of::<NiHeader>(), 512);

        let ni = Ni::from_file(Path::new(
            "/home/schroeterm/devel/books/2F0F3109BFAE4E0991D7CA0C2643948D/ni",
        ))
        .expect("cannot read ni file");

        assert_eq!(ni.header.format_version, 1);
        assert_eq!(ni.header.story_version, 1);
        assert_eq!(ni.header.stage_nodes_count, 17);
        assert_eq!(ni.header.image_assets_count, 9);
        assert_eq!(ni.header.sound_assets_count, 16);
        assert_eq!(ni.header.factory_disabled, 0);
    }

    #[test]
    fn load_li() {
        let li = Li::from_file(Path::new(
            "/home/schroeterm/devel/books/2F0F3109BFAE4E0991D7CA0C2643948D/li",
        ))
        .expect("cannot read ni file");

        assert_eq!(li.list.len(), 15);
    }

    #[test]
    fn load_ri() {
        let ri = Ri::from_file(Path::new(
            "/home/schroeterm/devel/books/2F0F3109BFAE4E0991D7CA0C2643948D/ri",
        ))
        .expect("cannot read ri file");

        for r in ri.list.iter().enumerate() {
            println!("{:?}", std::str::from_utf8(r.1))
        }

        assert_eq!(ri.list.len(), 9);
    }

    #[test]
    fn load_si() {
        let si = Si::from_file(Path::new(
            "/home/schroeterm/devel/books/2F0F3109BFAE4E0991D7CA0C2643948D/si",
        ))
        .expect("cannot read si file");

        for r in si.list.iter().enumerate() {
            println!("{:?}", std::str::from_utf8(r.1))
        }

        assert_eq!(si.list.len(), 16);
    }
}
