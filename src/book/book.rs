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
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use crate::decrypt::{DecryptedFile, FileReader};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub(super) action_node: String,
    pub(super) option_index: usize,
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
pub(super) struct StageNode {
    pub(super) uuid: String,
    pub(super) square_one: Option<bool>,
    pub(super) image: Option<String>,
    pub(super) audio: Option<String>,
    pub(super) ok_transition: Option<Transition>,
    pub(super) home_transition: Option<Transition>,
    pub(super) control_settings: ControlSettings,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActionNode {
    pub(super) id: String,
    pub(super) options: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct Story {
    pub(super) format: String,
    pub(super) version: usize,
    pub(super) night_mode_available: bool,
    pub(super) stage_nodes: Vec<StageNode>,
    pub(super) action_nodes: Vec<ActionNode>,
}

#[derive(Debug)]
pub struct Book {
    pub(super) encrypted: bool,

    pub(super) images_path: PathBuf,
    pub(super) audio_path: PathBuf,

    pub(super) story: Story,
    pub(super) stages: HashMap<String, usize>,
    pub(super) actions: HashMap<String, usize>,
    pub(super) start_node_uuid: Option<String>,

    pub(super) current_stage_node: Option<String>,
    pub(super) current_action_node: Option<String>,
    pub(super) current_action_index: usize,
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

pub enum Source<'a> {
    StoryJson(&'a Path),
    StoryPack(&'a Path),
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

    pub fn images_file_get(&self, image: &String) -> Result<(FileReader, image::ImageFormat)> {
        let path = &self.images_path.join(image);
        let file = if self.encrypted {
            FileReader::Encrypted(DecryptedFile::open(path)?)
        } else {
            FileReader::Plain(File::open(path)?)
        };

        let format = match path.extension() {
            Some(ext) => {
                if ext == "png" {
                    image::ImageFormat::Png
                } else if ext == "jpg" || ext == "jpeg" {
                    image::ImageFormat::Jpeg
                } else {
                    image::ImageFormat::Bmp
                }
            }
            None => image::ImageFormat::Bmp,
        };

        Ok((file, format))
    }

    pub fn audio_file_get(&self, audio: &String) -> Result<FileReader> {
        let path = &self.audio_path.join(audio);
        let file = if self.encrypted {
            FileReader::Encrypted(DecryptedFile::open(path)?)
        } else {
            FileReader::Plain(File::open(path)?)
        };

        Ok(file)
    }

    pub fn from_source(source: Source) -> Result<Self> {
        match source {
            Source::StoryJson(path) => Self::from_json_file(path),
            Source::StoryPack(path) => Self::from_pack_directory(path),
        }
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
