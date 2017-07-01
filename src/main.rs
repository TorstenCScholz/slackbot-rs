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

fn create_item(db_conn: &SqliteConnection, item_name: &str) -> Result<usize, diesel::result::Error> {
    use schema::items;

    let new_item = NewItem {
        name: item_name.to_owned().to_lowercase(),
    };

    diesel::insert(&new_item)
        .into(items::table)
        .execute(db_conn)
}

pub fn find_poll_by_name<'a, 'b>(db_conn: &'a SqliteConnection, poll_name: &'a str) -> Result<Poll, diesel::result::Error> {
    use self::schema::polls::dsl::*;

    let results = polls
        .filter(name.eq(poll_name))
        .limit(1)
        .load::<Poll>(db_conn)
        .expect("Cannot load polls from DB.");

    // TODO: What if not found?
    Ok(results[0].clone())
}

pub fn find_proposals_by_poll<'a, 'b>(db_conn: &'a SqliteConnection, poll: &'a Poll) -> Result<Vec<Proposal>, diesel::result::Error> {
    use self::schema::proposals::dsl::*;

    let results = proposals
        .filter(poll_id.eq(poll.id))
        .load::<Proposal>(db_conn)
        .expect("Cannot load proposals from DB.");

    Ok(results.clone())
}

pub fn find_votes_by_proposal<'a, 'b>(db_conn: &'a SqliteConnection, proposal: &'a Proposal) -> Result<Vec<Vote>, diesel::result::Error> {
    use self::schema::votes::dsl::*;

    let results = votes
        .filter(proposal_id.eq(proposal.id))
        .load::<Vote>(db_conn)
        .expect("Cannot load votes from DB.");

    Ok(results.clone())
}

pub fn find_item_by_proposal<'a, 'b>(db_conn: &'a SqliteConnection, proposal: &'a Proposal) -> Result<Item, diesel::result::Error> {
    use self::schema::items::dsl::*;

    let results = items
        .filter(id.eq(proposal.item_id))
        .load::<Item>(db_conn)
        .expect("Cannot load items from DB.");

    Ok(results[0].clone())
}

pub fn find_voter_by_vote<'a, 'b>(db_conn: &'a SqliteConnection, vote: &'a Vote) -> Result<Voter, diesel::result::Error> {
    use self::schema::voters::dsl::*;

    let results = voters
        .filter(id.eq(vote.voter_id))
        .load::<Voter>(db_conn)
        .expect("Cannot load votes from DB.");

    Ok(results[0].clone())
}

pub fn find_item_by_name<'a>(db_conn: &'a SqliteConnection, item_name: &'a str) -> Result<Item, diesel::result::Error> {
    use self::schema::items::dsl::*;

    let results = items
        .filter(name.eq(item_name))
        .limit(1)
        .load::<Item>(db_conn)
        .expect("Cannot load items from DB.");

    // TODO: What if not found?
    Ok(results[0].clone())
}

fn create_proposal(db_conn: &SqliteConnection, poll_id: i32, item_id: i32) -> Result<usize, diesel::result::Error> {
    use schema::proposals;

    let new_proposal = NewProposal {
        poll_id: poll_id,
        item_id: item_id
    };

    diesel::insert(&new_proposal)
        .into(proposals::table)
        .execute(db_conn)
}

fn find_proposal_by_poll_name_and_item_name(db_conn: &SqliteConnection, poll_name: &str, item_name: &str) -> Result<Proposal, diesel::result::Error> {
    use self::schema::proposals::dsl::*;

    // TODO: Error handling
    let poll = find_poll_by_name(db_conn, poll_name).unwrap();
    let item = find_item_by_name(db_conn, item_name).unwrap();

    let results = proposals
        .filter(poll_id.eq(poll.id))
        .filter(item_id.eq(item.id))
        .limit(1)
        .load::<Proposal>(db_conn)
        .expect("Cannot load proposals from DB.");

    // TODO: What if not found?
    Ok(results[0].clone())
}

fn find_voter_by_slack_id(db_conn: &SqliteConnection, slack_id: &str) -> Result<Voter, diesel::result::Error> {
    use self::schema::voters::dsl::*;

    let results = voters
        .filter(slack_id.eq(slack_id))
        .limit(1)
        .load::<Voter>(db_conn)
        .expect("Cannot load voters from DB.");

    // TODO: What if not found?
    Ok(results[0].clone())
}

fn find_vote_by_proposal_id_and_voter_id(db_conn: &SqliteConnection, proposal_id_param: i32, voter_id_param: i32) -> Result<Option<Vote>, diesel::result::Error> {
    use self::schema::votes::*;

    let results = dsl::votes
        .filter(dsl::proposal_id.eq(proposal_id_param))
        .filter(dsl::voter_id.eq(voter_id_param))
        .limit(1)
        .load::<Vote>(db_conn)
        .expect("Cannot load votes from DB.");

    if results.len() == 0 {
        Ok(None)
    } else {
        Ok(Some(results[0].clone()))
    }
}

fn exists_vote(db_conn: &SqliteConnection, proposal_id: i32, voter_id: i32) -> bool {
    let vote = find_vote_by_proposal_id_and_voter_id(db_conn, proposal_id, voter_id);

    if !vote.is_ok() {
        return false;
    }

    vote.unwrap().is_some()
}

fn create_vote(db_conn: &SqliteConnection, voter_id: i32, proposal_id: i32, weight: i32) -> Result<usize, diesel::result::Error> {
    use schema::votes;

    let new_vote = NewVote {
        voter_id: voter_id,
        proposal_id: proposal_id,
        weight: weight
    };

    diesel::insert(&new_vote)
        .into(votes::table)
        .execute(db_conn)
}

fn update_vote(db_conn: &SqliteConnection, voter_id_param: i32, proposal_id_param: i32, weight_param: i32) -> Result<usize, diesel::result::Error> {
    use self::schema::votes::dsl::*;

    diesel::update(votes
                    .filter(voter_id.eq(voter_id_param))
                    .filter(proposal_id.eq(proposal_id_param)))
        .set(weight.eq(weight_param))
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

    let list_items = |context: &mut Context, args: Vec<&str>| -> bool {
        use self::schema::items::dsl::*;

        if let Some(channel_id) = context.channel.as_ref() {
            let results = items
                .order(id.desc())
                .limit(5)
                .load::<Item>(context.db_conn)
                .expect("Error loading items");

            println!("Displaying {} items", results.len());

            let mut message = format!("Displaying latest items:\n");

            for (num, item) in results.iter().enumerate() {
                message = format!("{}{}. {}\n", message, (num + 1), item.name);
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

    let new_item = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 1 {
            return false;
        }

        let item_name = args[0];
        let mut message_formatted = format!("Creating a new item '{}'.", item_name);

        if let Some(channel_id) = context.channel.as_ref() {
            let result = create_item(&context.db_conn, item_name);

            match result {
                Err(Error::DatabaseError(error_type, error_message)) => {
                    match error_type {
                        DatabaseErrorKind::UniqueViolation => message_formatted = format!("Item name already taken!"),
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

    let new_proposal = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 2 {
            return false;
        }

        let poll_name = args[0];
        let item_name = args[1];
        let mut message_formatted = format!("Creating a new proposal for '{}' at '{}'.", poll_name, item_name);

        // TODO: Error handling
        let poll = find_poll_by_name(context.db_conn, poll_name).unwrap();
        let item = find_item_by_name(context.db_conn, item_name).unwrap();

        // TODO: Exit if proposal already exists

        if let Some(channel_id) = context.channel.as_ref() {
            let result = create_proposal(&context.db_conn, poll.id, item.id);

            match result {
                Err(Error::DatabaseError(error_type, error_message)) => {
                    match error_type {
                        DatabaseErrorKind::UniqueViolation => message_formatted = format!("Item name already taken!"),
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

    let vote = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 3 {
            return false;
        }

        let poll_name = args[0];
        let item_name = args[1];
        let weight_char = args[2];

        let weight = match weight_char {
            "-" => -1,
            _ => 1
        };
        let mut message_formatted = format!("Accepting vote for '{}' at '{}' with weight {}.", poll_name, item_name, weight);

        // TODO: Error handling
        // let poll = find_poll_by_name(context.db_conn, poll_name).unwrap();
        // let item = find_item_by_name(context.db_conn, item_name).unwrap();

        let proposal = find_proposal_by_poll_name_and_item_name(context.db_conn, poll_name, item_name).unwrap();
        let voter = find_voter_by_slack_id(context.db_conn, context.user.as_ref().unwrap().id.as_ref().unwrap()).unwrap();

        if let Some(channel_id) = context.channel.as_ref() {
            let vote_exists = exists_vote(context.db_conn, proposal.id, voter.id);

            let result = if !vote_exists {
                println!("Creating vote: prop({})", proposal.id);
                create_vote(context.db_conn, voter.id, proposal.id, weight)
            } else {
                println!("Updating vote: prop({})", proposal.id);
                update_vote(context.db_conn, voter.id, proposal.id, weight)
            };

            match result {
                Err(Error::DatabaseError(error_type, error_message)) => {
                    match error_type {
                        DatabaseErrorKind::UniqueViolation => message_formatted = format!("Vote already cast!"),
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

    let show_poll_results = |context: &mut Context, args: Vec<&str>| -> bool {
        if args.len() < 1 {
            return false;
        }

        let poll_name = args[0];

        let poll = find_poll_by_name(context.db_conn, poll_name).unwrap();

        if let Some(channel_id) = context.channel.as_ref() {
            let proposals = find_proposals_by_poll(context.db_conn, &poll).unwrap();

            let mut message = format!("Displaying poll results for {}:\n", poll_name);

            for proposal in proposals.iter() {
                let item = find_item_by_proposal(context.db_conn, &proposal).unwrap();
                let votes = find_votes_by_proposal(context.db_conn, &proposal).unwrap();

                message = format!("{}{}:", message, item.name);

                for vote in votes.iter() {
                    let voter = find_voter_by_vote(context.db_conn, &vote).unwrap();

                    message = format!("{} {}({})", message, voter.name, vote.weight);
                }

                message = format!("{}\n", message);
            }

            let _ = context.cli.sender().send_message(channel_id.as_str(), message.as_str());
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
    // * new_item (/)
    // * new_proposal (/)
    // * vote (/)
    // * show_poll_results (/)

    let mut commands: HashSet<Command> = HashSet::new();
    commands.insert(Command::new("new_poll", Box::new(new_poll)));
    commands.insert(Command::new("start_poll", Box::new(start_poll)));
    commands.insert(Command::new("conclude_poll", Box::new(conclude_poll)));
    commands.insert(Command::new("list_polls", Box::new(list_polls)));
    commands.insert(Command::new("list_items", Box::new(list_items)));
    commands.insert(Command::new("new_voter", Box::new(new_voter)));
    commands.insert(Command::new("new_item", Box::new(new_item)));
    commands.insert(Command::new("new_proposal", Box::new(new_proposal)));
    commands.insert(Command::new("vote", Box::new(vote)));
    commands.insert(Command::new("show_poll_results", Box::new(show_poll_results)));
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
}
