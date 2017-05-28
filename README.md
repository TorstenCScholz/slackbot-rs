# BUILD
Create an `.env` file in the project's root directory that contains the following environment variables:
* `SLACK_API_TOKEN` - Your Slack Bot API token
* `DATABASE_URL` - The URL of your SQLite database file

## DATABASE
To create an empty SQLite database you need to have `sqlite3` installed in your
system and then run
`sqlite3 <your-database-name> "CREATE TABLE a(f INT); DROP TABLE a;"`. Now you
should see a new file in the current folder named `<your-database-name`.

### MIGRATION
In order to migrate the database, you have to install Diesel CLI first:
`cargo install diesel_cli --no-default-features --features "sqlite"`. Now you can
run the database migration scripts via `diesel migration run`. Your database
should be up to date.

# RUN
Type `cargo run` to simply run the program.
