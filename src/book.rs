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
    io::BufReader,
    path::{Path, PathBuf},
};

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
    OK,
    HOME,
}

enum ActionWheel {
    RIGHT,
    LEFT,
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
            ActionButtons::OK => stage_node.ok_transition.as_ref()?,
            ActionButtons::HOME => stage_node.home_transition.as_ref()?,
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
            ActionButtons::OK => stage_node.ok_transition.as_ref()?,
            ActionButtons::HOME => stage_node.home_transition.as_ref()?,
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
            ActionWheel::LEFT => self.current_action_index as isize - 1,
            ActionWheel::RIGHT => self.current_action_index as isize + 1,
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

        /* Get action ba id */
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
        self.button(ActionButtons::OK)
    }

    /// Handle the HOME button
    pub fn button_home(&mut self) -> Option<()> {
        self.button(ActionButtons::HOME)
    }

    /// Handle the WHEEL button
    pub fn button_wheel_right(&mut self) -> Option<()> {
        self.button_wheel(ActionWheel::RIGHT)
    }

    /// Handle the WHEEL button
    pub fn button_wheel_left(&mut self) -> Option<()> {
        self.button_wheel(ActionWheel::LEFT)
    }
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
}
