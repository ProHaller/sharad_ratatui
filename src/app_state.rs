// app_state.rs

#[derive(PartialEq)]
pub enum AppState {
    MainMenu,
    InGame,
    LoadMenu,
    CreateImage,
    SettingsMenu,
    InputApiKey,
    InputSaveName,
}
