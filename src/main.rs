#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

extern crate slack;
extern crate dotenv;

pub mod schema;
pub mod models;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use slack::{Channel, Event, Message, RtmClient};
use dotenv::dotenv;

use std::env;

use self::models::*;

struct BasicHandler;

// TODO: Should come from config
const COMMAND_TOKEN: &'static str = "*";

fn get_channel_id<'a>(cli: &'a RtmClient, channel_name: &str) -> Option<&'a Channel> {
    cli.start_response()
       .channels
       .as_ref()
       .and_then(|channels| {
            channels
                .iter()
                .find(|chan| match chan.name {
                    None => false,
                    Some(ref name) => name == channel_name,
                 })
        })
}

fn get_command_line(whole_input: &str) -> Option<String> {
    if whole_input.starts_with(COMMAND_TOKEN) {
        let token_len = COMMAND_TOKEN.len();
        return Some(whole_input[token_len..].to_owned())
    }

    None
}

fn get_command(whole_input_sans_command_token: &str) -> Option<String> {
    let parts: Vec<_> = whole_input_sans_command_token.split_whitespace().collect();

    if parts.len() > 0 {
        return Some(parts[0].to_owned())
    } else {
        None
    }
}

fn get_command_parameters(whole_input_sans_command_token: &str) -> Vec<String> {
    let parts: Vec<_> = whole_input_sans_command_token.split_whitespace().collect();

    if get_command(whole_input_sans_command_token).is_some() {
        return parts[1..].iter().cloned().map(String::from).collect()
    } else {
        Vec::new()
    }
}

fn get_command_from_input(whole_input: &str) -> Option<String> {
    if whole_input.starts_with(COMMAND_TOKEN) {
        let token_len = COMMAND_TOKEN.len();
        let command_part = &whole_input[token_len..];

        return get_command(command_part)
    }

    None
}

impl slack::EventHandler for BasicHandler {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        println!("on_event(event: {:?})", event);

        // TODO: 1 Fetch message of users other than me
        // TODO: 2 Check if it is a command_name
        // TODO: 3 If it is, extract command and map it to a program command call
        // TODO: 4 Execute command call with specific context

        // TODO: 1 (ugly)
        let (input, channel_id) = match event {
            Event::Message(message) => {
                match *message {
                    Message::Standard(standard_message) => {
                        (standard_message.text, standard_message.channel)
                    },
                    _ => (None, None)
                }
            },
            _ => (None, None)
        };

        if let Some(input) = input {
            let command = get_command_from_input(&input);

            // TODO: Better splitting of command token, command and parameters
            // TODO: 2
            if let Some(command) = command {
                // TODO: We have the command, lookup implemented function for it
                println!("Got command: {}", command);
                let command_line = get_command_line(&input);
                let command_parameters = get_command_parameters(&command_line.unwrap_or(String::from("")));

                match command.as_ref() {
                    "new_poll" => {
                        let message_formatted = format!("Creating a new poll ({:?})", command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    },
                    "start_poll" => {
                        let message_formatted = format!("Starting poll ({:?})", command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    },
                    "new_user" => {
                        let message_formatted = format!("Creating new user ({:?})", command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    },
                    "new_item" => {
                        let message_formatted = format!("Creating new item ({:?})", command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    },
                    "new_proposal" => {
                        let message_formatted = format!("Creating new proposal ({:?})", command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    },
                    "vote" => {
                        let message_formatted = format!("Registering vote ({:?})", command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    },
                    _ => {
                        let message_formatted = format!("Unknown command '{}' ({:?})", command, command_parameters);
                        let message = message_formatted.as_str();
                        println!("{}", message);
                        if let Some(channel_id) = channel_id {
                            let _ = cli.sender().send_message(channel_id.as_str(), message);
                        }
                    }
                }
            }
        }
    }

    fn on_close(&mut self, cli: &RtmClient) {
        println!("on_close");
    }

    fn on_connect(&mut self, cli: &RtmClient) {
        // TODO: Remove all of this, because it will not be needed
        println!("on_connect");

        let channel_name = "general";
        let channel = get_channel_id(cli, channel_name).expect(format!("channel '{}' not found", channel_name).as_str());
        let channel_id = channel.id.as_ref().expect("cannot extract channel id");

        let _ = cli.sender().send_message(channel_id, "Hello world!");
    }
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn create_poll<'a>(db_conn: &SqliteConnection, name: &'a str, status: PollStatus) -> usize {
    use schema::polls;

    let new_poll = NewPoll {
        name: name,
        status: status.as_str(),
    };

    diesel::insert(&new_poll)
        .into(polls::table)
        .execute(db_conn)
        .expect("Error saving new post")
}

fn main() {
    dotenv().ok();

    let api_key = env::var("SLACK_API_TOKEN").expect("SLACK_API_TOKEN not set.");
    let mut handler = BasicHandler;
    let r = RtmClient::login_and_run(&api_key, &mut handler);

    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }

    use self::schema::polls::dsl::*;
    use self::models::Poll;

    let connection = establish_connection();

    // let _ = create_poll(&connection, "2017-06-04.2", "IN_PROGRESS");

    // let results = polls.filter(status.eq(PollStatus::InProgress.as_str()))
    //     .limit(5)
    //     .load::<Poll>(&connection)
    //     .expect("Error loading polls");
    //
    // println!("Displaying {} polls", results.len());
    // for poll in results {
    //     println!("{}", poll.name);
    //     println!("{}", poll.status);
    //     println!("{}", poll.started_at.unwrap_or("None".to_owned()));
    // }
}
