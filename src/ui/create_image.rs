// Import necessary modules and components from the application and Ratatui UI library.
use crate::app::App;
use ratatui::{
    layout::Alignment,     // Used for aligning text within widgets.
    style::{Color, Style}, // Used for styling text and widgets.
    widgets::*,            // Includes UI components like Paragraph and Block.
    Frame,                 // Represents the area where UI elements are drawn.
};

// Function to draw the image creation interface in the application.
pub fn draw_create_image(f: &mut Frame, app: &App) {
    let size = f.size();

    if size.width < 100 || size.height < 50 {
        let warning = Paragraph::new("Terminal too small. Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(warning, size);
        return;
    }
    let chunk = f.size(); // Get the current size of the terminal window or frame.

    // Define a UI element for the image creation feature using a Paragraph widget.
    let create_image_ui = Paragraph::new("Image creation functionality coming soon...")
        .style(Style::default().fg(Color::Magenta)) // Set the text color to magenta.
        .alignment(Alignment::Center) // Center-align the text within the widget.
        .block(Block::default().borders(Borders::ALL).title("Create Image")); // Enclose the paragraph in a block with a title and borders.

    // Render the defined UI element in the available space of the frame.
    f.render_widget(create_image_ui, chunk);
}
