use crossterm::event::KeyEvent;
use enum_dispatch::enum_dispatch;
use ratatui::{buffer::Buffer, layout::Rect};
use std::{fmt::Debug, path::PathBuf};

use crate::{
    app::Action,
    context::Context,
    ui::{
        ImageMenu, LoadMenu, MainMenu, SaveName, SettingsMenu, api_key_input::ApiKeyInput,
        game::InGame,
    },
};

#[enum_dispatch]
pub trait Component: Debug {
    fn on_key(&mut self, key: KeyEvent, context: Context) -> Option<Action>;
    // TODO: Implement KeyHints
    // fn key_hints(&mut self, key: KeyEvent, ) -> KeyHints
    // HACK: Could return a cursor postition?
    // TODO: Switch to Ratatui Textarea
    fn render(&mut self, area: Rect, buffer: &mut Buffer, context: &Context);
}

#[enum_dispatch(Component)]
#[derive(Debug)]
pub enum ComponentEnum {
    MainMenu,
    LoadMenu,
    SaveName,
    SettingsMenu,
    ImageMenu,
    InGame,
    ApiKeyInput,
}

impl ComponentEnum {
    pub fn get_ingame_save_path(&self) -> Option<&PathBuf> {
        if let ComponentEnum::InGame(game) = self {
            if let Some(path) = &game.state.save_path {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    }
}
