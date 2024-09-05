// app_state.rs

#[derive(PartialEq, Clone)]
pub enum AppState {
    MainMenu,
    InGame,
    LoadMenu,
    CreateImage,
    SettingsMenu,
    InputApiKey,
    InputSaveName,
}
