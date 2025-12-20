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

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use std::{
    collections::HashMap,
    fs::{self},
    io::BufReader,
    path::Path,
};
use uuid::Uuid;

use crate::{
    FileReader,
    decrypt::{DecryptedFile, decrypt_block},
};

use super::ControlSettings;
use super::book::ActionNode;
use super::book::Book;
use super::book::StageNode;
use super::book::Story;
use super::book::Transition;

// Node: 44 bytes
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Node {
    image_asset_index: i32,               /* Index in ri (-1 if not used) */
    sound_asset_index: i32,               /* Index in si (-1 if not used) */
    ok_transition_action_index: i32,      /* Index in li (-1 if not used) */
    ok_transition_options_count: i32,     /* -1 if not used */
    ok_transition_selected_option: i32,   /* -1 if not used */
    home_transition_action_index: i32,    /* Index in li (-1 if not used) */
    home_transition_options_count: i32,   /* -1 if not used */
    home_transition_selected_option: i32, /* -1 if not used */
    control_wheel_enabled: u16,           /* wheel enabled (0 for no) */
    control_ok_enabled: u16,              /* OK enabled (0 for no) */
    control_home_enabled: u16,            /* HOME enabled (0 for no) */
    control_pause_enabled: u16,           /* Pause enabled (0 for no) */
    control_autoplay_enabled: u16,        /* Autoplayback (0 for no) */
    padding: u16,
}

// Node Index: 512 bytes
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct NiHeader {
    format_version: u16,     /* File format version: always 1 */
    story_version: u16,      /* Story fs version */
    nodes_list_offset: u32,  /* Offset for the nodes (should be 0x200) */
    node_size: u32,          /* Node size (should be 0x2C) */
    stage_nodes_count: u32,  /* Number of stage nodes */
    image_assets_count: u32, /* Number of images */
    sound_assets_count: u32, /* Number of sounds */
    factory_disabled: u8,    /* Factory fs if different of 0 */
    padding: [u8; 487],
}

struct Ni {
    header: NiHeader,
    nodes: Vec<Node>,
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

impl Book {
    fn create_transition(
        li: &Li,
        stage_nodes: &Vec<StageNode>,
        action_nodes: &mut Vec<ActionNode>,
        actions: &mut HashMap<String, usize>,
        action_index: i32,
        options_count: i32,
        selected_option: i32,
    ) -> Option<Transition> {
        let index = action_index;
        let count = options_count;
        if index < 0 || count < 1 {
            return None;
        }

        let id = Uuid::new_v4().to_string();
        let mut options = Vec::new();

        for index in index..(index + count) {
            let stage_node_index = li.list[index as usize];
            options.push(stage_nodes[stage_node_index as usize].uuid.clone());
        }

        let action = ActionNode {
            id: id.clone(),
            options,
        };
        action_nodes.push(action);
        actions.insert(id.clone(), action_nodes.len() - 1);

        Some(Transition {
            action_node: id,
            option_index: selected_option as isize,
        })
    }

    fn gen_thumbnail(path: &Path, image: &String) -> Result<()> {
        let thumbnail = path.join("thumbnail.png");
        if fs::exists(&thumbnail)? {
            return Ok(());
        }

        let rf_image = path.join("rf").join(image);
        let image = FileReader::Encrypted(DecryptedFile::open(rf_image)?);
        let reader = BufReader::new(image);
        let img = image::load(reader, image::ImageFormat::Bmp)?;
        img.save_with_format(thumbnail, image::ImageFormat::Png)?;
        Ok(())
    }

    pub fn is_story_fs(path: &Path) -> bool {
        let story_li = path.join("li");
        let story_ni = path.join("ni");
        let story_ri = path.join("ri");
        let story_si = path.join("si");
        story_li.try_exists().unwrap_or_default()
            && story_ni.try_exists().unwrap_or_default()
            && story_ri.try_exists().unwrap_or_default()
            && story_si.try_exists().unwrap_or_default()
    }

    pub(super) fn from_fs_directory(path: &Path) -> Result<Self> {
        let ni = Ni::from_file(&path.join("ni"))?;
        let li = Li::from_file(&path.join("li"))?;
        let ri = Ri::from_file(&path.join("ri"))?;
        let si = Si::from_file(&path.join("si"))?;
        let nm = path.join("nm");

        let format = format!("v{}", ni.header.format_version);
        let version = 1;
        let night_mode_available = fs::exists(&nm)?;
        let mut stage_nodes = Vec::new();
        let mut action_nodes = Vec::new();
        let mut stages = HashMap::new();
        let mut actions = HashMap::new();

        let mut uuid = path
            .file_name()
            .context("Missing folder name")?
            .to_string_lossy()
            .to_string();
        let mut square_one = true;
        let start_node_uuid = Some(uuid.clone());

        for node in &ni.nodes {
            let image = ri.list.get(node.image_asset_index as usize).map(|bytes| {
                String::from_utf8_lossy(bytes)
                    .to_string()
                    .replace("\\", "/")
            });

            let audio = si.list.get(node.sound_asset_index as usize).map(|bytes| {
                String::from_utf8_lossy(bytes)
                    .to_string()
                    .replace("\\", "/")
            });

            let ok_transition = None;
            let home_transition = None;

            let control_settings = ControlSettings {
                autoplay: node.control_autoplay_enabled != 0,
                home: node.control_home_enabled != 0,
                ok: node.control_ok_enabled != 0,
                pause: node.control_pause_enabled != 0,
                wheel: node.control_wheel_enabled != 0,
            };

            if square_one {
                /* Generate a thumbnail with the first image if available.
                 * This one is useful when we want to list the books because
                 * the title is not available.
                 */
                if let Some(ref image) = image {
                    Self::gen_thumbnail(path, image)?;
                }
            } else {
                uuid = Uuid::new_v4().to_string();
            }

            let stage = StageNode {
                uuid: uuid.clone(),
                square_one: Some(square_one),
                image,
                audio,
                ok_transition,
                home_transition,
                control_settings,
            };

            stage_nodes.push(stage);
            stages.insert(uuid.clone(), stage_nodes.len() - 1);
            square_one = false
        }

        for i in 0..stage_nodes.len() {
            let node = ni.nodes[i];

            let ok_transition = Self::create_transition(
                &li,
                &stage_nodes,
                &mut action_nodes,
                &mut actions,
                node.ok_transition_action_index,
                node.ok_transition_options_count,
                node.ok_transition_selected_option,
            );
            let home_transition = Self::create_transition(
                &li,
                &stage_nodes,
                &mut action_nodes,
                &mut actions,
                node.home_transition_action_index,
                node.home_transition_options_count,
                node.home_transition_selected_option,
            );

            let stage_node = &mut stage_nodes[i];
            stage_node.ok_transition = ok_transition;
            stage_node.home_transition = home_transition;
        }

        let story = Story {
            format,
            version,
            night_mode_available,
            stage_nodes,
            action_nodes,
        };

        let current_action_index = 0;
        let current_stage_node = start_node_uuid.clone();
        let current_action_node = None;

        Ok(Self {
            encrypted: true,
            images_path: path.join("rf").to_path_buf(),
            audio_path: path.join("sf").to_path_buf(),
            story,
            stages,
            actions,
            start_node_uuid,
            current_stage_node,
            current_action_node,
            current_action_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_ni() {
        assert_eq!(std::mem::size_of::<Node>(), 44);
        assert_eq!(std::mem::size_of::<NiHeader>(), 512);

        let ni = Ni::from_file(Path::new(
            "/home/schroeterm/devel/lunii/nathan/.content/2643948D/ni",
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
            "/home/schroeterm/devel/lunii/nathan/.content/2643948D/li",
        ))
        .expect("cannot read ni file");

        assert_eq!(li.list.len(), 15);
    }

    #[test]
    fn load_ri() {
        let ri = Ri::from_file(Path::new(
            "/home/schroeterm/devel/lunii/nathan/.content/2643948D/ri",
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
            "/home/schroeterm/devel/lunii/nathan/.content/2643948D/si",
        ))
        .expect("cannot read si file");

        for r in si.list.iter().enumerate() {
            println!("{:?}", std::str::from_utf8(r.1))
        }

        assert_eq!(si.list.len(), 16);
    }

    #[test]
    fn load_story_fs() {
        let pk = Path::new("/home/schroeterm/devel/lunii/nathan/.content/2643948D");
        let book = Book::from_fs_directory(pk).expect("story fs not found");
    }
}
