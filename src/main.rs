use iced::keyboard::{Key, Modifiers};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Length, Subscription};
use std::fs;
use std::path::{Path, PathBuf};

pub fn main() -> iced::Result {
    iced::application(FileLister::default, update, view)
        .subscription(subscription)
        .run()
}

#[derive(Debug, Clone)]
struct FileInfo {
    name: String,
    path: PathBuf,
    size: u64,
    is_dir: bool,
    extension: String,
}

impl FileInfo {
    fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let is_dir = path.is_dir();
        let size = if is_dir {
            0
        } else {
            fs::metadata(path).map(|m| m.len()).unwrap_or(0)
        };
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        FileInfo {
            name,
            path: path.to_path_buf(),
            size,
            is_dir,
            extension,
        }
    }

    fn size_formatted(&self) -> String {
        if self.is_dir {
            "Folder".to_string()
        } else {
            format_size(self.size)
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug, Clone)]
struct FileLister {
    current_dir: PathBuf,
    files: Vec<FileInfo>,
    selected_file: Option<FileInfo>,
    error_message: String,
    font_size: f32,
}

impl Default for FileLister {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut lister = FileLister {
            current_dir: current_dir.clone(),
            files: Vec::new(),
            selected_file: None,
            error_message: String::new(),
            font_size: 14.0,
        };
        lister.load_directory(&current_dir);
        lister
    }
}

impl FileLister {
    fn load_directory(&mut self, path: &Path) {
        self.files.clear();
        self.error_message.clear();

        // Add parent directory option if not at root
        if let Some(parent) = path.parent() {
            self.files.push(FileInfo {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                size: 0,
                is_dir: true,
                extension: String::new(),
            });
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let file_info = FileInfo::from_path(&entry.path());
                    self.files.push(file_info);
                }
            }
            Err(e) => {
                self.error_message = format!("Error reading directory: {}", e);
            }
        }

        // Sort: directories first, then files, alphabetically
        self.files.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
    }

    fn navigate_to(&mut self, path: &Path) {
        self.current_dir = path.to_path_buf();
        self.load_directory(path);
        self.selected_file = None;
    }
}

fn update(state: &mut FileLister, message: Message) {
    match message {
        Message::FileSelected(file_info) => {
            if file_info.name == ".." || file_info.is_dir {
                state.navigate_to(&file_info.path);
                return;
            }
            state.selected_file = Some(file_info);
        }
        Message::Refresh => {
            let dir = state.current_dir.clone();
            state.load_directory(&dir);
            state.selected_file = None;
        }
        Message::FontSizeChanged(delta) => {
            state.font_size = (state.font_size + delta).max(8.0).min(72.0);
        }
    }
}

fn view(state: &FileLister) -> Element<'_, Message> {
    // Title
    let title = text("File Lister").size(state.font_size + 10.0);

    // Current directory path
    let path_text = text(format!(
        "📁 {}",
        state.current_dir.display()
    ))
    .size(state.font_size);

    // Refresh button
    let refresh_btn = button(text("⟳ Refresh").size(state.font_size))
        .padding([5, 10])
        .on_press(Message::Refresh);

    // Header row
    let header = row![path_text, Space::new().width(Length::Fill), refresh_btn].spacing(10);

    // File list
    let file_list: Vec<Element<'_, Message>> = state
        .files
        .iter()
        .map(|file| {
            let icon = if file.is_dir { "📁" } else { "📄" };

            let file_row = row![
                text(icon).size(state.font_size + 2.0),
                text(file.name.clone()).size(state.font_size),
                Space::new().width(Length::Fill),
                text(file.size_formatted()).size(state.font_size - 2.0),
            ]
            .spacing(8);

            button(file_row)
                .padding(10)
                .width(Length::Fill)
                .on_press(Message::FileSelected(file.clone()))
                .into()
        })
        .collect();

    let file_scroll = scrollable(column(file_list).spacing(2))
        .height(Length::Fixed(300.0))
        .width(Length::Fill);

    // File info panel
    let info_panel: Element<'_, Message> = if let Some(selected) = &state.selected_file {
        let info_content: Element<'_, Message> = column![
            row![
                text("Name:").width(Length::Fixed(80.0)),
                text(&selected.name).size(state.font_size)
            ]
            .spacing(5),
            row![
                text("Type:").width(Length::Fixed(80.0)),
                text(if selected.is_dir {
                    "Folder".to_string()
                } else {
                    if selected.extension.is_empty() {
                        "".to_string()
                    } else {
                        selected.extension.to_uppercase()
                    }
                }).size(state.font_size)
            ]
            .spacing(5),
            row![
                text("Size:").width(Length::Fixed(80.0)),
                text(selected.size_formatted()).size(state.font_size)
            ]
            .spacing(5),
            row![
                text("Path:").width(Length::Fixed(80.0)),
                text(selected.path.display().to_string()).size(state.font_size)
            ]
            .spacing(5),
        ]
        .spacing(5)
        .into();

        container(info_content)
            .padding(15)
            .width(Length::Fill)
            .into()
    } else {
        container(text("Click on a file to see information").size(state.font_size - 2.0))
            .padding(15)
            .width(Length::Fill)
            .into()
    };

    // Error message
    let error_section: Element<'_, Message> = if !state.error_message.is_empty() {
        container(text(&state.error_message).size(state.font_size - 2.0))
            .padding(10)
            .width(Length::Fill)
            .into()
    } else {
        column![].into()
    };

    // Main layout
    let content = column![
        title,
        header,
        error_section,
        file_scroll,
        container(text("")).height(Length::Fixed(10.0)),
        info_panel
    ]
    .spacing(10)
    .padding(20);

    container(content)
        .width(Length::Fixed(600.0))
        .center_x(Length::Fill)
        .into()
}

fn subscription(_state: &FileLister) -> Subscription<Message> {
    use iced::keyboard;

    keyboard::listen().filter_map(move |event| {
        let keyboard::Event::KeyPressed {
            key,
            modifiers,
            repeat: false,
            ..
        } = event
        else {
            return None;
        };

        if modifiers == Modifiers::CTRL {
            match key {
                Key::Character(c) if c.as_str() == "+" || c.as_str() == "=" => {
                    return Some(Message::FontSizeChanged(2.0));
                }
                Key::Character(c) if c.as_str() == "-" => {
                    return Some(Message::FontSizeChanged(-2.0));
                }
                _ => {}
            }
        }
        None
    })
}

#[derive(Debug, Clone)]
enum Message {
    FileSelected(FileInfo),
    Refresh,
    FontSizeChanged(f32),
}
