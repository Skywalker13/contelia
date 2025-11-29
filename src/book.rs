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
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};
use uuid::Uuid;

use bytemuck::{Pod, Zeroable};
use std::fs::{self};

const STORY_JSON: &str = "story.json";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    action_node: String,
    option_index: usize,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ControlSettings {
    pub wheel: bool,
    pub ok: bool,
    pub home: bool,
    pub pause: bool,
    pub autoplay: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StageNode {
    uuid: String,
    square_one: Option<bool>,
    image: Option<String>,
    audio: Option<String>,
    ok_transition: Option<Transition>,
    home_transition: Option<Transition>,
    control_settings: ControlSettings,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ActionNode {
    id: String,
    options: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Story {
    format: String,
    version: usize,
    night_mode_available: bool,
    stage_nodes: Vec<StageNode>,
    action_nodes: Vec<ActionNode>,
}

#[derive(Debug)]
pub struct Book {
    path: PathBuf,
    story: Story,
    stages: HashMap<String, usize>,
    actions: HashMap<String, usize>,
    start_node_uuid: Option<String>,

    current_stage_node: Option<String>,
    current_action_node: Option<String>,
    current_action_index: usize,
}

#[derive(Debug)]
pub struct Stage {
    pub square_one: bool,
    pub control_settings: ControlSettings,
    pub image: Option<String>,
    pub audio: Option<String>,
}

enum ActionButtons {
    Ok,
    Home,
}

enum ActionWheel {
    Right,
    Left,
}

impl Book {
    fn stage_node_get(&self) -> Option<&StageNode> {
        let uuid = self.current_stage_node.as_ref()?;
        let index = self.stages.get(uuid)?;
        self.story.stage_nodes.get(*index)
    }

    fn action_node_get(
        &self,
        button: &ActionButtons,
        stage_node: &StageNode,
    ) -> Option<&ActionNode> {
        let transition = match button {
            ActionButtons::Ok => stage_node.ok_transition.as_ref()?,
            ActionButtons::Home => stage_node.home_transition.as_ref()?,
        };
        let id = &transition.action_node;
        let index = self.actions.get(id)?;
        self.story.action_nodes.get(*index)
    }

    fn button(&mut self, button: ActionButtons) -> Option<()> {
        let stage_node = self.stage_node_get()?;
        let action_node = match self.action_node_get(&button, stage_node) {
            Some(node) => node,
            None => {
                self.stage_reset();
                return None;
            }
        };

        let transition = match button {
            ActionButtons::Ok => stage_node.ok_transition.as_ref()?,
            ActionButtons::Home => stage_node.home_transition.as_ref()?,
        };
        let option_index = transition.option_index;

        let action_node_id = action_node.id.clone();
        let next_stage_uuid = action_node.options.get(option_index)?.clone();

        self.current_action_node = Some(action_node_id);
        self.current_action_index = option_index;
        self.current_stage_node = Some(next_stage_uuid);

        Some(())
    }

    fn button_wheel(&mut self, direction: ActionWheel) -> Option<()> {
        let action_node = self
            .story
            .action_nodes
            .iter()
            .find(|node| Some(node.id.clone()) == self.current_action_node)?;

        let mut option_index = match direction {
            ActionWheel::Left => self.current_action_index as isize - 1,
            ActionWheel::Right => self.current_action_index as isize + 1,
        };
        if option_index >= action_node.options.len() as isize {
            option_index = 0;
        } else if option_index < 0 {
            option_index = action_node.options.len() as isize - 1;
        }
        let option_index = option_index as usize;
        let next_stage_uuid = action_node.options.get(option_index)?.clone();

        self.current_action_index = option_index;
        self.current_stage_node = Some(next_stage_uuid);

        Some(())
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let story_path = path.join(STORY_JSON);
        let file = File::open(story_path)?;
        let reader = BufReader::new(file);
        let story: Story = serde_json::from_reader(reader)?;

        /* The first node is like the cover of the book */
        let start_node_uuid = story
            .stage_nodes
            .iter()
            .find(|node| node.square_one == Some(true))
            .map(|node| node.uuid.clone());

        /* Get stage by uuid */
        let stages = story
            .stage_nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.uuid.clone(), i))
            .collect();

        /* Get action by id */
        let actions = story
            .action_nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.clone(), i))
            .collect();

        let current_action_index = 0;
        let current_stage_node = start_node_uuid.clone();
        let current_action_node = None;

        Ok(Book {
            path: path.to_path_buf(),
            story,
            stages,
            actions,
            start_node_uuid,
            current_stage_node,
            current_action_node,
            current_action_index,
        })
    }

    /// Reset the book to the start node
    pub fn stage_reset(&mut self) {
        self.current_action_index = 0;
        self.current_stage_node = self.start_node_uuid.clone();
        self.current_action_node = None;
    }

    /// Get the current image, audio and inputs from the stage
    pub fn stage_get(&self) -> Option<Stage> {
        let stage_node = self.stage_node_get()?;
        Some(Stage {
            square_one: stage_node.square_one.unwrap_or(false),
            control_settings: stage_node.control_settings.clone(),
            image: stage_node.image.clone(),
            audio: stage_node.audio.clone(),
        })
    }

    pub fn path_get(&self) -> &PathBuf {
        &self.path
    }

    /// Handle OK button
    pub fn button_ok(&mut self) -> Option<()> {
        self.button(ActionButtons::Ok)
    }

    /// Handle the HOME button
    pub fn button_home(&mut self) -> Option<()> {
        self.button(ActionButtons::Home)
    }

    /// Handle the WHEEL button
    pub fn button_wheel_right(&mut self) -> Option<()> {
        self.button_wheel(ActionWheel::Right)
    }

    /// Handle the WHEEL button
    pub fn button_wheel_left(&mut self) -> Option<()> {
        self.button_wheel(ActionWheel::Left)
    }
}

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

pub struct StoryPack {}

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
    pub fn from_directory(path: &Path) -> Result<Self> {
        let ni = Ni::from_file(&path.join("ni"))?;
        let li = Li::from_file(&path.join("li"))?;
        let ri = Ri::from_file(&path.join("ri"))?;
        let si = Si::from_file(&path.join("si"))?;

        let format = format!("v{}", ni.header.format_version);
        let version = 1;
        let night_mode_available = false; // FIXME: depends of .nm
        let mut stage_nodes = Vec::new();
        let mut action_nodes = Vec::new();
        let stages = HashMap::new();
        let actions = HashMap::new();

        let mut uuid = path
            .file_name()
            .context("Missing folder name")?
            .to_string_lossy()
            .to_string();
        let mut square_one = true;

        for node in &ni.nodes {
            let image = ri
                .list
                .get(node.image_asset_index as usize)
                .map(|bytes| String::from_utf8_lossy(bytes).to_string());

            let audio = si
                .list
                .get(node.sound_asset_index as usize)
                .map(|bytes| String::from_utf8_lossy(bytes).to_string());

            let ok_transition = None; // FIXME
            let home_transition = None; // FIXME

            let control_settings = ControlSettings {
                autoplay: node.control_autoplay_enabled != 0,
                home: node.control_home_enabled != 0,
                ok: node.control_ok_enabled != 0,
                pause: node.control_pause_enabled != 0,
                wheel: node.control_wheel_enabled != 0,
            };

            if !square_one {
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

            println!("{:?}", stage);

            stage_nodes.push(stage);
            square_one = false
        }

        for i in 0..stage_nodes.len() {
            let node = ni.nodes[i];

            let ok_index = node.ok_transition_action_index;
            let ok_count = node.ok_transition_options_count;
            if ok_index >= 0 && ok_count >= 1 {
                let id = Uuid::new_v4().to_string();
                let mut options = Vec::new();

                for index in ok_index..(ok_index + ok_count) {
                    let stage_node_index = li.list[index as usize];
                    options.push(stage_nodes[stage_node_index as usize].uuid.clone());
                }

                let action = ActionNode {
                    id: id.clone(),
                    options,
                };
                action_nodes.push(action);

                let stage_node = &mut stage_nodes[i];
                stage_node.ok_transition = Some(Transition {
                    action_node: id,
                    option_index: node.ok_transition_selected_option as usize,
                });

                println!("{:?}", stage_node);
            }
        }

        let story = Story {
            format,
            version,
            night_mode_available,
            stage_nodes,
            action_nodes,
        };

        Ok(Book {
            path: path.to_path_buf(),
            story,
            stages,
            actions,
            start_node_uuid: None, // FIXME
            current_stage_node: None,
            current_action_node: None,
            current_action_index: 0,
        })
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
    fn scenario() {
        let story = Path::new("test");
        let mut book = Book::from_file(story).expect("story.json not found");

        let start_node_uuid = String::from("2F0F3109BFAE4E0991D7CA0C2643948D");
        assert_eq!(book.start_node_uuid, Some(start_node_uuid));

        /* Init */
        let current_stage_node = String::from("2F0F3109BFAE4E0991D7CA0C2643948D");
        assert_eq!(book.current_action_index, 0);
        assert_eq!(book.current_action_node, None);
        assert_eq!(book.current_stage_node, Some(current_stage_node));

        /* OK */
        book.button_ok().expect("OK button fail");
        let current_action_node = String::from("ff38d914-9cca-4d50-86e9-4ae6bf3e65c1");
        let current_stage_node = String::from("ef895f69-6f4e-48a5-ad3b-3ed12c8c4608");
        assert_eq!(book.current_action_index, 0);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, true);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, false);

        /* WHEEL RIGHT */
        book.button_wheel_right().expect("Cannot move to option 1");
        let current_action_node = String::from("ff38d914-9cca-4d50-86e9-4ae6bf3e65c1");
        let current_stage_node = String::from("cd8566b9-b700-4694-9ea5-212ffe0e6e8e");
        assert_eq!(book.current_action_index, 1);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, true);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, false);

        /* OK */
        book.button_ok().expect("OK button fail");
        let current_action_node = String::from("e1204f8a-a39c-4de6-928b-491a6d4d0b2a");
        let current_stage_node = String::from("4e32f223-bc5d-4f3b-8cf2-34980664f356");
        assert_eq!(book.current_action_index, 0);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, true);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, false);

        /* WHEEL RIGHT 2× */
        book.button_wheel_right().expect("Cannot move to option 1");
        book.button_wheel_right().expect("Cannot move to option 2");
        let current_action_node = String::from("e1204f8a-a39c-4de6-928b-491a6d4d0b2a");
        let current_stage_node = String::from("0b296637-77cb-4b8b-83ee-8d5c9d9b805c");
        assert_eq!(book.current_action_index, 2);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, true);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, false);

        /* OK 2× */
        book.button_ok().expect("OK button fail");
        book.button_ok().expect("OK button fail");
        let current_action_node = String::from("fb7e0b44-a7f3-4967-9887-c6d7c8e9c1df");
        let current_stage_node = String::from("e643c767-d789-4bc2-b25f-71dc50d02020");
        assert_eq!(book.current_action_index, 0);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, false);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, true);

        /* HOME */
        book.button_home().expect("HOME button fail");
        let current_action_node = String::from("ff38d914-9cca-4d50-86e9-4ae6bf3e65c1");
        let current_stage_node = String::from("ef895f69-6f4e-48a5-ad3b-3ed12c8c4608");
        assert_eq!(book.current_action_index, 0);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, true);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, false);

        /* WHEEL LEFT */
        book.button_wheel_left().expect("Cannot move to option 1");
        let current_action_node = String::from("ff38d914-9cca-4d50-86e9-4ae6bf3e65c1");
        let current_stage_node = String::from("cd8566b9-b700-4694-9ea5-212ffe0e6e8e");
        assert_eq!(book.current_action_index, 1);
        assert_eq!(book.current_action_node, Some(current_action_node));
        assert_eq!(book.current_stage_node, Some(current_stage_node));
        let stage = book.stage_get().expect("stage get fail");
        assert_eq!(stage.control_settings.wheel, true);
        assert_eq!(stage.control_settings.ok, true);
        assert_eq!(stage.control_settings.home, true);
        assert_eq!(stage.control_settings.pause, false);
        assert_eq!(stage.control_settings.autoplay, false);

        /* HOME */
        book.button_home();
        let current_stage_node = String::from("2F0F3109BFAE4E0991D7CA0C2643948D");
        assert_eq!(book.current_action_index, 0);
        assert_eq!(book.current_action_node, None);
        assert_eq!(book.current_stage_node, Some(current_stage_node));
    }

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
    fn load_story_pack() {
        let pk = Path::new("/home/schroeterm/devel/lunii/nathan/.content/2643948D");
        let mut book = Book::from_directory(pk).expect("story pack not found");
    }
}
