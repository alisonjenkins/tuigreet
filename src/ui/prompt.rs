use std::{error::Error, io};

use termion::raw::RawTerminal;
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Text},
    Frame,
};

use crate::{info::get_hostname, Greeter, Mode};

const WIDTH: u16 = 80;
const PADDING: u16 = 2;

const GREETING_INDEX: usize = 0;
const USERNAME_INDEX: usize = 1;
const ANSWER_INDEX: usize = 2;
const MESSAGE_INDEX: usize = 3;

const TITLE: &'static str = "Authenticate into";
const USERNAME: &'static str = "Username:";
const WORKING: &'static str = "Please wait...";

pub fn draw(
    greeter: &mut Greeter,
    f: &mut Frame<TermionBackend<RawTerminal<io::Stdout>>>,
) -> Result<(u16, u16), Box<dyn Error>> {
    let size = f.size();

    let height = get_height(&greeter, &greeter.message);
    let x = (size.width - WIDTH) / 2;
    let y = (size.height - height) / 2;

    let container = Rect::new(x, y, WIDTH, height);
    let frame = Rect::new(x + PADDING, y + PADDING, WIDTH - PADDING, height - PADDING);

    let hostname = format!(" {} {} ", TITLE, get_hostname());

    let block = Block::default()
        .title(&hostname)
        .borders(Borders::ALL)
        .border_type(BorderType::Plain);

    f.render_widget(block, container);

    let (message, message_height) = get_message_height(&greeter.message, 1, 1);
    let (greeting, greeting_height) = get_greeting_height(&greeter.greeting, 1, 0);

    let constraints = [
        Constraint::Length(greeting_height), // Greeting
        Constraint::Length(2),               // Username
        Constraint::Length(if let Mode::Username = greeter.mode {
            message_height
        } else {
            2
        }), // Message or answer
        Constraint::Length(if let Mode::Password = greeter.mode {
            message_height
        } else {
            1
        }), // Message
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.as_ref())
        .split(frame);

    let pos = chunks[USERNAME_INDEX];

    if let Some(greeting) = &greeting {
        let greeting_text = [Text::raw(greeting.trim_end())];
        let greeting_label = Paragraph::new(greeting_text.iter()).alignment(Alignment::Center);

        f.render_widget(greeting_label, chunks[GREETING_INDEX]);
    }

    let username_text = [Text::styled(
        USERNAME,
        Style::default().modifier(Modifier::BOLD),
    )];
    let username_label = Paragraph::new(username_text.iter());

    let username_value_text = [Text::raw(&greeter.username)];
    let username_value = Paragraph::new(username_value_text.iter());

    let answer_text = if greeter.working {
        [Text::raw(WORKING)]
    } else {
        [Text::styled(
            &greeter.prompt,
            Style::default().modifier(Modifier::BOLD),
        )]
    };
    let answer_label = Paragraph::new(answer_text.iter());

    f.render_widget(username_label, chunks[USERNAME_INDEX]);
    f.render_widget(
        username_value,
        Rect::new(
            1 + chunks[USERNAME_INDEX].x + USERNAME.len() as u16,
            chunks[USERNAME_INDEX].y,
            30,
            1,
        ),
    );

    if let Mode::Password = greeter.mode {
        f.render_widget(answer_label, chunks[ANSWER_INDEX]);

        if !greeter.secret {
            let answer_value_text = [Text::raw(&greeter.answer)];
            let answer_value = Paragraph::new(answer_value_text.iter());

            f.render_widget(
                answer_value,
                Rect::new(
                    chunks[ANSWER_INDEX].x + greeter.prompt.len() as u16,
                    chunks[ANSWER_INDEX].y,
                    30,
                    1,
                ),
            );
        }
    }

    if let Some(ref message) = message {
        let message_text = [Text::raw(message)];
        let message = Paragraph::new(message_text.iter());

        match greeter.mode {
            Mode::Username => f.render_widget(message, chunks[ANSWER_INDEX]),
            Mode::Password => f.render_widget(message, chunks[MESSAGE_INDEX]),
        }
    }

    match greeter.mode {
        Mode::Username => {
            let username = greeter.username.clone();
            let offset = get_cursor_offset(greeter, username);

            Ok((2 + pos.x + USERNAME.len() as u16 + offset as u16, 1 + pos.y))
        }

        Mode::Password => {
            let answer = greeter.answer.clone();
            let offset = get_cursor_offset(greeter, answer);

            if greeter.secret {
                Ok((1 + pos.x + greeter.prompt.len() as u16, 3 + pos.y))
            } else {
                Ok((
                    1 + pos.x + greeter.prompt.len() as u16 + offset as u16,
                    3 + pos.y,
                ))
            }
        }
    }
}

fn get_height(greeter: &Greeter, message: &Option<String>) -> u16 {
    let (_, message_height) = get_message_height(message, 2, 0);
    let (_, greeting_height) = get_greeting_height(&greeter.greeting, 1, 0);
    let initial = match greeter.mode {
        Mode::Username => 5,
        Mode::Password => 7,
    };

    initial + greeting_height + message_height
}

fn get_greeting_height(
    greeting: &Option<String>,
    padding: u16,
    fallback: u16,
) -> (Option<String>, u16) {
    if let Some(greeting) = greeting {
        let wrapped = textwrap::fill(greeting, WIDTH as usize - 4);
        let height = wrapped.trim_end().matches('\n').count();

        (Some(wrapped), height as u16 + 1 + padding)
    } else {
        (None, fallback)
    }
}

fn get_message_height(
    message: &Option<String>,
    padding: u16,
    fallback: u16,
) -> (Option<String>, u16) {
    if let Some(message) = message {
        let wrapped = textwrap::fill(message.trim_end(), WIDTH as usize - 4);
        let height = wrapped.trim_end().matches('\n').count();

        (Some(wrapped), height as u16 + padding)
    } else {
        (None, fallback)
    }
}

fn get_cursor_offset(greeter: &mut Greeter, text: String) -> i16 {
    let mut offset = text.len() as i16 + greeter.cursor_offset;
    if offset < 0 {
        offset = 0;
        greeter.cursor_offset = -(text.len() as i16);
    }
    if offset > text.len() as i16 {
        offset = text.len() as i16;
        greeter.cursor_offset = 0;
    }

    offset
}