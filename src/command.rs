use diesel::sqlite::{SqliteConnection};

use slack::{RtmClient, User};

use std::cmp::{PartialEq, Eq};
use std::hash::{Hash, Hasher};

pub struct Command<'a> {
    name: &'a str,
    callback: Box<Fn(&mut Context, Vec<&str>) -> bool>,
}

impl <'a> Command<'a> {
    pub fn new<'r>(name: &'r str, callback: Box<Fn(&mut Context, Vec<&str>) -> bool>) -> Command<'r> {
        Command {
            name: name,
            callback: callback
        }
    }

    pub fn invoke(&self, context: &mut Context, parameters: Vec<&str>) -> bool {
        // TODO: Better with chained map etc. magic?
        let mut user_name = "<UNKNOWN USER>";

        if let Some(user) = context.user.as_ref() {
            if let Some(name) = user.name.as_ref() {
                user_name = name;
            }
        }

        println!("[Info] Invoking command {} by {} with parameters {:?}.", self.name, user_name, parameters);
        (self.callback)(context, parameters)
    }

    pub fn matches(&self, name: &str) -> bool {
        self.name == name
    }
}

impl <'a> PartialEq for Command<'a> {
    fn eq(&self, other: &Command) -> bool {
        self.name == other.name
    }
}

impl <'a> Eq for Command<'a> {}

impl <'a> Hash for Command<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

pub struct Context<'a> {
    pub db_conn: &'a SqliteConnection,
    pub cli: &'a RtmClient,
    pub channel: &'a Option<String>,
    pub user: &'a Option<User>
}

impl <'a> Context<'a> {
    pub fn new(db_conn: &'a SqliteConnection, cli: &'a RtmClient, channel: &'a Option<String>, user: &'a Option<User>) -> Context<'a> {
        Context {
            db_conn: db_conn,
            cli: cli,
            channel: channel,
            user: user
        }
    }
}
