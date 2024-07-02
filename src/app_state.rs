// app_state.rs

#[derive(PartialEq)]
pub enum AppState {
    MainMenu,
    InGame,
    LoadGame,
    CreateImage,
    Settings,
    InputApiKey,
    InputSaveName,
}
