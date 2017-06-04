-- Restore polls table

-- ALTER TABLE polls RENAME TO polls_old;
--
-- CREATE TABLE polls (
-- 	id integer PRIMARY KEY AUTOINCREMENT,
-- 	name varchar
-- );
--
-- INSERT INTO polls (id, name) SELECT id, name FROM polls_old;
--
-- DROP TABLE polls_old;

-- /Restore

-- Restore votes table

-- ALTER TABLE votes RENAME TO votes_old;
-- 
-- CREATE TABLE votes (
-- 	id integer PRIMARY KEY AUTOINCREMENT,
--   voter_id integer,
--   proposal_id integer,
-- 	FOREIGN KEY(voter_id) REFERENCES voter(id),
-- 	FOREIGN KEY(proposal_id) REFERENCES proposal(id)
-- );
--
-- INSERT INTO votes (id, voter_id, proposal_id) SELECT id, voter_id, proposal_id FROM votes_old;
--
-- DROP TABLE votes_old;

-- /Restore
