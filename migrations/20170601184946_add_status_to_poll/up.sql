ALTER TABLE polls ADD COLUMN status varchar;
ALTER TABLE polls ADD COLUMN started_at varchar;
ALTER TABLE polls ADD column concluded_at varchar;

ALTER TABLE votes ADD COLUMN weight integer;
