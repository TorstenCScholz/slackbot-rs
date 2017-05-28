use schema::{polls, items, proposals, votes, voters};

#[derive(Identifiable, Queryable, Associations)]
#[has_many(proposals)]
pub struct Poll {
    pub id: i32,
    pub name: String
}

#[derive(Identifiable, Queryable, Associations)]
#[has_many(proposals)]
pub struct Item {
    pub id: i32,
    pub name: String
}

#[derive(Identifiable, Queryable, Associations)]
#[has_many(votes)]
#[belongs_to(Poll)]
#[belongs_to(Item)]
pub struct Proposal {
    pub id: i32,
    pub poll_id: i32,
    pub item_id: i32
}

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(Voter)]
#[belongs_to(Proposal)]
pub struct Vote {
    pub id: i32,
    pub voter_id: i32,
    pub proposal_id: i32
}

#[derive(Identifiable, Queryable, Associations)]
#[has_many(votes)]
pub struct Voter {
    pub id: i32,
    pub name: String,
    pub slack_id: String
}
