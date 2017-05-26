extern crate slack;
extern crate dotenv;

use slack::{Channel, Event, RtmClient};
use dotenv::dotenv;

use std::env;

struct BasicHandler;

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

#[allow(unused_variables)]
impl slack::EventHandler for BasicHandler {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        println!("on_event(event: {:?})", event);

        // TODO: Handle different events (i.e. channel messages NOT from me)
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
}
