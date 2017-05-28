CREATE TABLE polls (
	id integer PRIMARY KEY AUTOINCREMENT,
	name varchar
);

CREATE TABLE items (
	id integer PRIMARY KEY AUTOINCREMENT,
	name varchar
);

CREATE TABLE proposals (
	id integer PRIMARY KEY AUTOINCREMENT,
  poll_id integer,
  item_id integer,
	FOREIGN KEY(poll_id) REFERENCES poll(id),
	FOREIGN KEY(item_id) REFERENCES item(id)
);

CREATE TABLE votes (
	id integer PRIMARY KEY AUTOINCREMENT,
  voter_id integer,
  proposal_id integer,
	FOREIGN KEY(voter_id) REFERENCES voter(id),
	FOREIGN KEY(proposal_id) REFERENCES proposal(id)
);

CREATE TABLE voters (
	id integer PRIMARY KEY AUTOINCREMENT,
	name varchar,
  slack_id varchar
);
