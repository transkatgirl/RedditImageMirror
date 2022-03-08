#![allow(clippy::cast_possible_wrap)]

use sqlite::{Connection, OpenFlags, State};

use reqwest::blocking::Client;

use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct Post {
	pub title: String,
	pub static_id: String,
	pub link: String,
	pub media_url: String,
	pub sensitive: bool,
}

#[derive(Debug)]
pub struct UniquePost {
	pub title: String,
	pub link: String,
	static_id: String,
	pub raw_image: Vec<u8>,
	pub sensitive: bool,
	pub depth: usize,
}

fn database_contains(
	database: &Connection,
	table: &str,
	column: &str,
	search_str: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
	let mut statement =
		database.prepare(["SELECT * FROM ", table, " WHERE ", column, " == ?"].concat())?;
	statement.bind(1, search_str)?;

	Ok(statement.next()? == State::Row)
}

fn database_append_post(
	database: &Connection,
	static_id: &str,
	media_hash: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut statement = database.prepare(["INSERT INTO posts VALUES (?, ?)"].concat())?;
	statement.bind(1, static_id)?;
	statement.bind(2, media_hash)?;

	while let State::Row = statement.next()? {}

	Ok(())
}

pub fn database_append_message(
	database: &Connection,
	static_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut statement = database.prepare(["INSERT INTO messages VALUES (?, ?)"].concat())?;
	statement.bind(1, static_id)?;
	statement.bind(
		2,
		SystemTime::now()
			.duration_since(SystemTime::UNIX_EPOCH)?
			.as_secs() as i64,
	)?;

	while let State::Row = statement.next()? {}

	Ok(())
}

pub fn database_get_messages_older_than(
	database: &Connection,
	duration: Duration,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	let current_unix_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

	let search_time = (current_unix_time - duration).as_secs() as i64;

	let mut statement =
		database.prepare(["SELECT * FROM messages WHERE timestamp < ?"].concat())?;
	statement.bind(1, search_time)?;

	let mut messages = Vec::new();

	while let State::Row = statement.next()? {
		messages.push(statement.read::<String>(0)?);
	}

	Ok(messages)
}

pub fn database_remove_message(
	database: &Connection,
	static_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut statement = database.prepare(["DELETE FROM messages WHERE static_id == ?"].concat())?;
	statement.bind(1, static_id)?;

	while let State::Row = statement.next()? {}

	Ok(())
}

pub fn load_database(file: &str) -> Result<Connection, Box<dyn std::error::Error>> {
	let flags = OpenFlags::new().set_create().set_read_write();
	let connection = Connection::open_with_flags(file, flags)?;

	let _err1 = connection.execute("CREATE TABLE posts (static_id TEXT, media_hash TEXT);");
	let _err2 = connection.execute("CREATE TABLE messages (static_id TEXT, timestamp INTEGER);");

	//let _err2 = connection.execute("ALTER TABLE posts ADD media_hash TEXT;");

	Ok(connection)
}

pub fn database_remove_unique_post(
	database: &Connection,
	post: &UniquePost,
) -> Result<(), Box<dyn std::error::Error>> {
	let id: &str = &post.static_id;

	let mut statement = database.prepare(["DELETE FROM posts WHERE static_id == ?"].concat())?;
	statement.bind(1, id)?;

	while let State::Row = statement.next()? {}

	Ok(())
}

pub fn get_unique_post(
	database: &Connection,
	client: &Client,
	posts: Vec<Post>,
) -> Result<Option<UniquePost>, Box<dyn std::error::Error>> {
	for (depth, post) in posts.into_iter().enumerate() {
		if database_contains(database, "posts", "static_id", &post.static_id)? {
			continue;
		}

		let hashed_image = if let Ok(i) = crate::media::url_to_hashed_image(client, &post.media_url)
		{
			i
		} else {
			continue;
		};

		if database_contains(database, "posts", "media_hash", &hashed_image.hash)? {
			database_append_post(database, &post.static_id, &hashed_image.hash)?;
			continue;
		}

		database_append_post(database, &post.static_id, &hashed_image.hash)?;

		return Ok(Some(UniquePost {
			title: post.title,
			link: post.link,
			raw_image: hashed_image.raw_image,
			static_id: post.static_id,
			sensitive: post.sensitive,
			depth,
		}));
	}

	Ok(None)
}
