use anyhow::Result;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    action_node: String,
    option_index: usize,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ControlSettings {
    wheel: bool,
    ok: bool,
    home: bool,
    pause: bool,
    autoplay: bool,
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
    control_settings: ControlSettings,
}

impl Stage {
    pub fn get_image_name() {}

    pub fn get_audio_name() {}
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
        if option_index > action_node.options.len() as isize {
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
        let file = File::open(path)?;
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
            story,
            stages,
            actions,
            start_node_uuid,
            current_stage_node,
            current_action_node,
            current_action_index,
        })
    }

    /* Reset the book to the start node */
    pub fn stage_reset(&mut self) {
        self.current_action_index = 0;
        self.current_stage_node = self.start_node_uuid.clone();
        self.current_action_node = None;
    }

    /* Get the current image, audio and inputs from the stage */
    pub fn stage_get(&self) -> Option<Stage> {
        let stage_node = self.stage_node_get()?;
        Some(Stage {
            control_settings: stage_node.control_settings.clone(),
        })
    }

    /* Handle OK button */
    pub fn button_ok(&mut self) -> Option<()> {
        self.button(ActionButtons::OK)
    }

    /* Handle the HOME button */
    pub fn button_home(&mut self) -> Option<()> {
        self.button(ActionButtons::HOME)
    }

    /* Handle the WHEEL button */
    pub fn button_wheel_right(&mut self) -> Option<()> {
        self.button_wheel(ActionWheel::RIGHT)
    }

    /* Handle the WHEEL button */
    pub fn button_wheel_left(&mut self) -> Option<()> {
        self.button_wheel(ActionWheel::LEFT)
    }
}
