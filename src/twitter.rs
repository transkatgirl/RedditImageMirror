use oauth1_header::Credentials;
use reqwest::blocking::Client;
use serde_derive::Deserialize;
use sqlite::Connection;

use std::{collections::HashMap, thread, time::Duration, vec};

#[derive(Debug, Deserialize)]
struct RawTweetResponse {
	id: u64,
}

#[derive(Debug, Deserialize)]
struct RawMediaResponse {
	media_id: u64,
}

fn build_auth_header(
	config: &crate::config::Config,
	http_method: &str,
	base_url: &str,
	query_string: &[(&str, &str)],
) -> String {
	let mut params = HashMap::new();

	for param in query_string {
		params.insert(param.0, param.1);
	}

	Credentials::new(
		&config.twitter.api_key,
		&config.twitter.api_secret_key,
		&config.twitter.access_token,
		&config.twitter.access_token_secret,
	)
	.auth(http_method, base_url, &params)
}

fn upload_post_media(
	config: &crate::config::Config,
	client: &Client,
	post: &crate::posts::UniquePost,
) -> Result<String, Box<dyn std::error::Error>> {
	let query_string = &[
		("media_category", "tweet_image"),
		(
			"media_data",
			&base64::encode_config(&post.raw_image, base64::URL_SAFE),
		),
	];

	let response = client
		.post("https://upload.twitter.com/1.1/media/upload.json")
		.header(
			"Authorization",
			build_auth_header(
				config,
				"POST",
				"https://upload.twitter.com/1.1/media/upload.json",
				query_string,
			),
		)
		.form(query_string)
		.send()?
		.error_for_status()?;

	let json: RawMediaResponse = serde_json::from_str(&response.text()?)?;

	Ok(json.media_id.to_string())
}

fn upload_post_tweet(
	config: &crate::config::Config,
	client: &Client,
	post: &crate::posts::UniquePost,
	media_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
	let query_string: &[(&str, &str)] = &[
		("media_ids", media_id),
		("possibly_sensitive", &post.sensitive.to_string()),
		("status", &format!("{} {}", post.title, post.link)),
	];

	let response = client
		.post("https://api.twitter.com/1.1/statuses/update.json")
		.query(query_string)
		.header(
			"Authorization",
			build_auth_header(config, "POST", "https://api.twitter.com/1.1/statuses/update.json", query_string),
		)
		/*.header("Content-Type", "application/json")
		.body("{\"status\": \"test\"}") */
		.send()?
		.error_for_status()?;

	let json: RawTweetResponse = serde_json::from_str(&response.text()?)?;

	Ok(json.id.to_string())
}

fn delete_tweet(
	config: &crate::config::Config,
	client: &Client,
	tweet_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	let query_string = &[];

	let url = [
		"https://api.twitter.com/1.1/statuses/destroy/",
		tweet_id,
		".json",
	]
	.concat();

	let _err = client
		.post(&url)
		.header(
			"Authorization",
			build_auth_header(config, "POST", &url, query_string),
		)
		.send()?
		.error_for_status()?;

	Ok(())
}

pub fn upload_unique_post(
	config: &crate::config::Config,
	database: &Connection,
	client: &Client,
	post: &crate::posts::UniquePost,
) -> Result<String, Box<dyn std::error::Error>> {
	let media_id = upload_post_media(config, client, post)?;
	let post_id = upload_post_tweet(config, client, post, &media_id)?;

	crate::posts::database_append_message(database, &post_id)?;

	Ok(post_id)
}

pub fn delete_old_messages(
	config: &crate::config::Config,
	database: &Connection,
	client: &Client,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	let max_age = if let Some(secs) = config.twitter.max_tweet_age {
		Duration::from_secs(secs)
	} else {
		return Ok(vec![]);
	};

	let message_ids = crate::posts::database_get_messages_older_than(database, max_age)?;

	let mut removed_messages = vec![];

	for message in &message_ids {
		if delete_tweet(config, client, message).is_ok() {
			crate::posts::database_remove_message(database, message)?;
			removed_messages.push(message.clone());
		}
		thread::sleep(Duration::from_secs(18));
	}

	let broken_message_ids = crate::posts::database_get_messages_older_than(database, max_age*10)?;

	for message in &broken_message_ids {
		crate::posts::database_remove_message(database, message)?;
	}

	Ok([removed_messages, broken_message_ids].concat())
}
