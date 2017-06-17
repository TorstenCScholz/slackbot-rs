use schema::{polls, items, proposals, votes, voters};

#[derive(Clone)]
pub enum PollStatus {
    Aborted, // Not in use
    Concluded,
    InProgress,
    Stopped
}

impl PollStatus {
    pub fn from_str(s: &str) -> Option<PollStatus> {
        match s {
            "ABORTED" => Some(PollStatus::Aborted),
            "CONCLUDED" => Some(PollStatus::Concluded),
            "IN_PROGRESS" => Some(PollStatus::InProgress),
            "STOPPED" => Some(PollStatus::Stopped),
            _ => None
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            &PollStatus::Aborted => "ABORTED",
            &PollStatus::Concluded => "CONCLUDED",
            &PollStatus::InProgress => "IN_PROGRESS",
            &PollStatus::Stopped => "STOPPED",
        }
    }
}

#[derive(Identifiable, Queryable, Associations, Clone)]
#[has_many(proposals)]
pub struct Poll {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub concluded_at: Option<String>
}

#[derive(Insertable, Clone)]
#[table_name="polls"]
pub struct NewPoll<'a> {
    pub name: &'a str,
    pub status: &'a str,
}

#[derive(Identifiable, Queryable, Associations, Clone)]
#[has_many(proposals)]
pub struct Item {
    pub id: i32,
    pub name: String
}

#[derive(Insertable, Clone)]
#[table_name="items"]
pub struct NewItem {
    pub name: String
}

#[derive(Identifiable, Queryable, Associations, Clone)]
#[has_many(votes)]
#[belongs_to(Poll)]
#[belongs_to(Item)]
pub struct Proposal {
    pub id: i32,
    pub poll_id: i32,
    pub item_id: i32
}

#[derive(Insertable, Clone)]
#[table_name="proposals"]
pub struct NewProposal {
    pub poll_id: i32,
    pub item_id: i32
}

#[derive(Identifiable, Queryable, Associations, Clone)]
#[belongs_to(Voter)]
#[belongs_to(Proposal)]
pub struct Vote {
    pub id: i32,
    pub voter_id: i32,
    pub proposal_id: i32,
    pub weight: i32
}

#[derive(Identifiable, Queryable, Associations, Clone)]
#[has_many(votes)]
pub struct Voter {
    pub id: i32,
    pub name: String,
    pub slack_id: Option<String>
}

#[derive(Insertable, Clone)]
#[table_name="voters"]
pub struct NewVoter {
    pub name: String,
    pub slack_id: Option<String> // TODO: Should not be optional and it should be unique as name can be changed!
}
