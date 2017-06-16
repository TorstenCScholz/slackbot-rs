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

use slack::{Channel, Event, Message, RtmClient, User};

use dotenv::dotenv;

use std::env;
use std::collections::{HashSet};

use self::command::{Command, Context};
use self::models::*;

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

fn get_command_implementation<'a>(command_name: &str, command_implementations: &'a HashSet<Command<'a>>) -> Option<&'a Command<'a>> {
    for command_implementation in command_implementations {
        if command_implementation.matches(command_name) {
            return Some(command_implementation);
        }
    }

    None
}

struct BasicHandler<'a> {
    pub db_conn: SqliteConnection,
    pub commands: HashSet<Command<'a>>,
    pub users: Vec<User>
}

impl <'a> slack::EventHandler for BasicHandler<'a> {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        println!("on_event(event: {:?})", event);

        // TODO: 1 Fetch message of users other than me
        // TODO: 2 Check if it is a command_name
        // TODO: 3 If it is, extract command and map it to a program command call
        // TODO: 4 Execute command call with specific context

        // TODO: 1 (ugly)
        let (input, channel_id, user_id) = match event {
            Event::Message(message) => {
                match *message {
                    Message::Standard(standard_message) => {
                        (standard_message.text, standard_message.channel, standard_message.user)
                    },
                    _ => (None, None, None)
                }
            },
            _ => (None, None, None)
        };

        if let Some(input) = input {
            let command = get_command_from_input(&input);

            // TODO: Better splitting of command token, command and parameters
            // TODO: 2
            if let Some(command) = command {
                println!("Got command: {}", command);
                let command_line = get_command_line(&input);
                let command_parameters = get_command_parameters(&command_line.unwrap_or(String::from("")));

                let command_implementation_option = get_command_implementation(command.as_str(), &self.commands);

                let user = self.users.iter()
                    .find(|user| user.id.as_ref().unwrap() == user_id.as_ref().unwrap())
                    .unwrap()
                    .clone();

                if let Some(command_implementation) = command_implementation_option {
                    let user = Some(user);
                    let mut context = Context::new(&self.db_conn, cli, &channel_id, &user);
                    let enough_params = command_implementation.invoke(&mut context, command_parameters.iter().map(String::as_str).collect());

                    if !enough_params {
                        let _ = cli.sender().send_message(channel_id.as_ref().unwrap().as_str(), format!("Not enough parameters for command '{}'.", command).as_str());
                    }
                } else {
                    let _ = cli.sender().send_message(channel_id.as_ref().unwrap().as_str(), format!("Command '{}' not found.", command).as_str());
                }
            }
        }
    }

    fn on_close(&mut self, cli: &RtmClient) {
        println!("on_close");
    }

    fn on_connect(&mut self, cli: &RtmClient) {
        // Save users
        let users: Vec<_> = cli.start_response()
            .users
            .clone()
            .unwrap_or_else(Vec::new);

        self.users.extend(users.clone());
    }
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn create_poll(db_conn: &SqliteConnection, name: &str, status: PollStatus) -> Result<usize, diesel::result::Error> {
    use schema::polls;

    let new_poll = NewPoll {
        name: name,
        status: status.as_str(),
    };

    diesel::insert(&new_poll)
        .into(polls::table)
        .execute(db_conn)
}

pub fn can_start_poll(db_conn: &SqliteConnection, poll_name: &str) -> bool {
    use self::schema::polls::dsl::*;

    let results = polls
        .filter(name.eq(poll_name))
        .limit(1)
        .load::<Poll>(db_conn)
        .expect("Cannot load polls from DB.");

    ((results.len() == 1) && (results[0].status.as_str() == PollStatus::Stopped.as_str()))
}

pub fn start_poll(db_conn: &SqliteConnection, poll_name: &str) -> Result<usize, diesel::result::Error> {
    use self::schema::polls::dsl::*;

    diesel::update(polls.filter(name.eq(poll_name)))
        .set(status.eq(PollStatus::InProgress.as_str()))
        .execute(db_conn)
}

pub fn can_conclude_poll(db_conn: &SqliteConnection, poll_name: &str) -> bool {
    use self::schema::polls::dsl::*;

    let results = polls
        .filter(name.eq(poll_name))
        .limit(1)
        .load::<Poll>(db_conn)
        .expect("Cannot load polls from DB.");

    ((results.len() == 1) && (results[0].status.as_str() != PollStatus::Concluded.as_str()))
}

pub fn conclude_poll(db_conn: &SqliteConnection, poll_name: &str) -> Result<usize, diesel::result::Error> {
    use self::schema::polls::dsl::*;

    diesel::update(polls.filter(name.eq(poll_name)))
        .set(status.eq(PollStatus::Concluded.as_str()))
        .execute(db_conn)
}

fn create_voter(db_conn: &SqliteConnection, user_id: &str, user_name: &str) -> Result<usize, diesel::result::Error> {
    use schema::voters;

    let new_voter = NewVoter {
        name: user_name.to_owned(),
        slack_id: Some(user_id.to_owned())
    };

    diesel::insert(&new_voter)
        .into(voters::table)
        .execute(db_conn)
}

fn main() {
    dotenv().ok();

    let new_poll = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 1 {
            return false;
        }

        let poll_name = args[0];
        let mut message_formatted = format!("Creating a new poll '{}'.", poll_name);

        if let Some(channel_id) = context.channel.as_ref() {
            let result = create_poll(&context.db_conn, poll_name, PollStatus::Stopped);

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

    let start_poll = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 1 {
            return false;
        }

        let poll_name = args[0];
        let mut message_formatted = format!("Started poll '{}'.", poll_name);

        if let Some(channel_id) = context.channel.as_ref() {
            // TODO: Check if poll exists

            if can_start_poll(&context.db_conn, poll_name) {
                let result = start_poll(&context.db_conn, poll_name);

                match result {
                    Err(error) => message_formatted = format!("Cannot start poll: {:?}", error),
                    Ok(_) => ()
                }

                let message = message_formatted.as_str();
                println!("{}", message);

                let _ = context.cli.sender().send_message(channel_id.as_str(), message);
            } else {
                let _ = context.cli.sender().send_message(channel_id.as_str(), "Cannot start a poll that has already been started once.");
            }
        }

        true
    };

    let conclude_poll = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 1 {
            return false;
        }

        let poll_name = args[0];
        let mut message_formatted = format!("Concluded poll '{}'.", poll_name);

        if let Some(channel_id) = context.channel.as_ref() {
            // TODO: Check if poll exists

            if can_conclude_poll(&context.db_conn, poll_name) {
                let result = conclude_poll(&context.db_conn, poll_name);

                match result {
                    Err(error) => message_formatted = format!("Cannot conclude poll: {:?}", error),
                    Ok(_) => ()
                }

                let message = message_formatted.as_str();
                println!("{}", message);

                let _ = context.cli.sender().send_message(channel_id.as_str(), message);
            } else {
                let _ = context.cli.sender().send_message(channel_id.as_str(), "Cannot conclude a poll that has already been concluded.");
            }
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
                message = format!("{}{}. {} ({})\n", message, (num + 1), poll.name, poll.status);
            }

            let _ = context.cli.sender().send_message(channel_id.as_str(), message.as_str());
        }

        true
    };

    let new_voter = |context: &mut Context, args: Vec<&str>| -> bool {
        if !context.user.is_some() {
            println!("Error: Cannot create voter.");
            return true;
        }

        let user = context.user.as_ref().unwrap();
        let user_name = user.name.as_ref().unwrap();
        let user_id = user.id.as_ref().unwrap();

        let mut message_formatted = format!("Creating a new voter '{}' ({}).", user_name, user_id);

        if let Some(channel_id) = context.channel.as_ref() {
            let result = create_voter(&context.db_conn, user_id, user_name);

            match result {
                Err(Error::DatabaseError(error_type, error_message)) => {
                    match error_type {
                        DatabaseErrorKind::UniqueViolation => message_formatted = format!("User name already exists!"),
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

    let help = |context: &mut Context, args: Vec<&str>| -> bool {
        if let Some(channel_id) = context.channel.as_ref() {
            let _ = context.cli.sender().send_message(channel_id.as_str(), "I cannot help you right now :confused:. Maybe try a real person?");
        }

        true
    };

    // TODO: Implemented the following commands:
    // * new_voter (/)
    // * new_item
    // * new_proposal
    // * vote
    // * show_poll_results

    let mut commands: HashSet<Command> = HashSet::new();
    commands.insert(Command::new("new_poll", Box::new(new_poll)));
    commands.insert(Command::new("start_poll", Box::new(start_poll)));
    commands.insert(Command::new("conclude_poll", Box::new(conclude_poll)));
    commands.insert(Command::new("list_polls", Box::new(list_polls)));
    commands.insert(Command::new("new_voter", Box::new(new_voter)));
    commands.insert(Command::new("help", Box::new(help)));

    let db_conn = establish_connection();

    let api_key = env::var("SLACK_API_TOKEN").expect("SLACK_API_TOKEN not set.");
    let mut handler = BasicHandler {
        db_conn: db_conn,
        commands: commands,
        users: Vec::new()
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
