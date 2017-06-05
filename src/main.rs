#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate slack;
extern crate dotenv;

pub mod command;
pub mod schema;
pub mod models;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::{Error, DatabaseErrorKind};

use slack::{Channel, Event, Message, RtmClient};

use dotenv::dotenv;

use std::env;
use std::collections::{HashSet};

use self::command::{Command, Context};
use self::models::*;

// TODO: Should come from config
const COMMAND_TOKEN: &'static str = "*";

struct BasicHandler<'a> {
    pub db_conn: SqliteConnection,
    pub commands: HashSet<Command<'a>>
}

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

fn get_command_implementation<'a>(command_name: &str, command_implementations: &'a HashSet<Command<'a>>) -> Option<&'a Command<'a>> {
    for command_implementation in command_implementations {
        if command_implementation.matches(command_name) {
            return Some(command_implementation);
        }
    }

    None
}

impl <'a> slack::EventHandler for BasicHandler<'a> {
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

                let command_implementation_option = get_command_implementation(command.as_str(), &self.commands);

                if let Some(command_implementation) = command_implementation_option {
                    let mut context = Context::new(&self.db_conn, cli, &channel_id);
                    let enough_params = command_implementation.invoke(&mut context, command_parameters.iter().map(String::as_str).collect());

                    if !enough_params {
                        let _ = cli.sender().send_message(channel_id.as_ref().unwrap().as_str(), format!("Not enough parameters for command '{}'.", command).as_str());
                    }
                } else {
                    let _ = cli.sender().send_message(channel_id.as_ref().unwrap().as_str(), format!("Command '{}' not found.", command).as_str());
                }

                // for command_obj in &self.commands {
                //     if command_obj.matches(command.as_str()) {
                //         successful_invoked = true;
                //
                //         let mut context = Context::new(&self.db_conn, cli, &channel_id);
                //         let enough_params = command_obj.invoke(&mut context, command_parameters.iter().map(String::as_str).collect());
                //
                //         if !enough_params {
                //             let _ = cli.sender().send_message(channel_id.as_ref().unwrap().as_str(), format!("Not enough parameters for command '{}'.", command).as_str());
                //         }
                //     }
                // }

                // match command.as_ref() {
                    // "new_poll" => {
                    //     for command_obj in &self.commands {
                    //         if command_obj.matches(command.as_str()) {
                    //             let mut context = Context::new(&self.db_conn, cli, &channel_id);
                    //             let enough_params = command_obj.invoke(&mut context, command_parameters.iter().map(String::as_str).collect());
                    //
                    //             if !enough_params {
                    //                 let _ = context.cli.sender().send_message(channel_id.as_ref().unwrap().as_str(), "Not enough parameters.");
                    //             }
                    //         }
                    //     }
                    // },
                    // "start_poll" => {
                    //     let message_formatted = format!("Starting poll ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "new_user" => {
                    //     let message_formatted = format!("Creating new user ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "new_item" => {
                    //     let message_formatted = format!("Creating new item ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "new_proposal" => {
                    //     let message_formatted = format!("Creating new proposal ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "vote" => {
                    //     let message_formatted = format!("Registering vote ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "list_polls" => {
                    //     let message_formatted = format!("Listing all registered polls");
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "help" => {
                    //         let message_formatted = format!("Displaying help");
                    //         let message = message_formatted.as_str();
                    //         println!("{}", message);
                    //         if let Some(channel_id) = channel_id {
                    //             let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //         }
                    // },
                    // "conclude_poll" => {
                    //     let message_formatted = format!("Concluding poll ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                    // "show_poll_results" => {
                    //     let message_formatted = format!("Displaying current poll results ({:?})", command_parameters);
                    //     let message = message_formatted.as_str();
                    //     println!("{}", message);
                    //     if let Some(channel_id) = channel_id {
                    //         let _ = cli.sender().send_message(channel_id.as_str(), message);
                    //     }
                    // },
                //     _ => {
                //         // let message_formatted = format!("Unknown command '{}' ({:?})", command, command_parameters);
                //         // let message = message_formatted.as_str();
                //         // println!("{}", message);
                //         // if let Some(channel_id) = channel_id {
                //         //     let _ = cli.sender().send_message(channel_id.as_str(), message);
                //         // }
                //     }
                // }
            }
        }
    }

    fn on_close(&mut self, cli: &RtmClient) {
        println!("on_close");
    }

    fn on_connect(&mut self, cli: &RtmClient) {
    }
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn create_poll<'a>(db_conn: &SqliteConnection, name: &'a str, status: PollStatus) -> Result<usize, diesel::result::Error> {
    use schema::polls;

    let new_poll = NewPoll {
        name: name,
        status: status.as_str(),
    };

    diesel::insert(&new_poll)
        .into(polls::table)
        .execute(db_conn)
}



fn main() {
    dotenv().ok();

    let new_poll = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 1 {
            return false;
        }

        let poll_name = args[0];
        let mut message_formatted = format!("Creating a new poll '{}'", poll_name);

        if let Some(channel_id) = context.channel.as_ref() {
            let result = create_poll(&context.db_conn, poll_name, PollStatus::InProgress);

            match result {
                Err(Error::DatabaseError(error_type, error_message)) => {
                    match error_type {
                        DatabaseErrorKind::UniqueViolation => message_formatted = format!("Poll name already taken!"),
                        _ => ()
                    }
                },
                Err(_) => (),
                Ok(_) => ()
            }

            let message = message_formatted.as_str();
            println!("{}", message);

            let _ = context.cli.sender().send_message(channel_id.as_str(), message);
        }

        true
    };

    let list_polls = |context: &mut Context, args: Vec<&str>| -> bool {
        use self::schema::polls::dsl::*;

        if let Some(channel_id) = context.channel.as_ref() {
            let results = polls
                .order(id.desc())
                .limit(5)
                .load::<Poll>(context.db_conn)
                .expect("Error loading polls");

            println!("Displaying {} polls", results.len());

            let mut message = format!("Displaying latest polls:\n");

            for (num, poll) in results.iter().enumerate() {
                message = format!("{}{}. {}\n", message, (num + 1), poll.name);
            }

            let _ = context.cli.sender().send_message(channel_id.as_str(), message.as_str());
        }

        true
    };

    let mut commands: HashSet<Command> = HashSet::new();
    commands.insert(Command::new("new_poll", Box::new(new_poll)));
    commands.insert(Command::new("list_polls", Box::new(list_polls)));

    let db_conn = establish_connection();

    let api_key = env::var("SLACK_API_TOKEN").expect("SLACK_API_TOKEN not set.");
    let mut handler = BasicHandler {
        db_conn: db_conn,
        commands: commands
    };
    let r = RtmClient::login_and_run(&api_key, &mut handler);

    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }


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
