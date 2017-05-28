#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

extern crate slack;
extern crate dotenv;

pub mod schema;
pub mod models;

use diesel::prelude::*;

use slack::{Channel, Event, Message, RtmClient};
use dotenv::dotenv;

use std::env;

struct BasicHandler;

const COMMAND_TOKEN: &'static str = "-&gt; ";

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

fn get_command_from_input(input: &str) -> Option<String> {
    if input.starts_with(COMMAND_TOKEN) {
        let token_len = COMMAND_TOKEN.len();
        let command_part = &input[token_len..];

        Some(command_part.to_owned())
    } else {
        None
    }
}

impl slack::EventHandler for BasicHandler {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        println!("on_event(event: {:?})", event);

        // TODO: 1 Fetch message of users other than me
        // TODO: 2 Check if it is a command_name
        // TODO: 3 If it is, extract command and map it to a program command call
        // TODO: 4 Execute command call with specific context

        // TODO: 1 (ugly)
        let input = match event {
            Event::Message(message) => {
                match *message {
                    Message::Standard(standard_message) => standard_message.text,
                    _ => None
                }
            },
            _ => None
        };

        if let Some(input) = input {
            let command = get_command_from_input(&input);

            // TODO: 2
            if let Some(command) = command {
                // TODO: We have the command, lookup implemented function for it
                println!("Got command: {}", command);
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

fn main() {
    dotenv().ok();

    let api_key = env::var("SLACK_API_TOKEN").expect("SLACK_API_TOKEN not set.");
    let mut handler = BasicHandler;
    let r = RtmClient::login_and_run(&api_key, &mut handler);

    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }

    // TODO: Test if database with diesel works as planned
}
