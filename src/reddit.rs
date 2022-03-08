#![allow(clippy::struct_excessive_bools)]

use serde_derive::Deserialize;

use crate::{config::Config, media::convert_to_media_url, posts::Post};

use reqwest::blocking::Client;

#[derive(Debug, Deserialize)]
struct RawSubredditResponse {
	data: RawSubredditData,
}

#[derive(Debug, Deserialize)]
struct RawSubredditData {
	children: Vec<RawRedditPost>,
}

#[derive(Debug, Deserialize)]
struct RawRedditPost {
	kind: String,
	data: RawRedditPostData,
}

#[derive(Debug, Deserialize)]
struct RawRedditPostData {
	title: String,

	name: String,
	id: String,

	url: String,

	upvote_ratio: f32,

	spoiler: bool,
	locked: bool,
	quarantine: bool,
	stickied: bool,
	is_robot_indexable: bool,

	is_video: bool,
}

pub fn get_reddit_posts(
	config: &Config,
	client: &Client,
) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
	let depth_string = if let Some(limit) = &config.reddit.max_depth {
		"?limit=".to_owned() + &limit.to_string()
	} else {
		"".to_string()
	};

	let response = client
		.get(
			"https://reddit.com/r/".to_owned()
				+ &config.reddit.subreddits.join("+")
				+ ".json" + &depth_string,
		)
		.send()?
		.error_for_status()?;

	let response_text = response.text()?;

	let json: RawSubredditResponse = serde_json::from_str(&response_text)?;

	let mut parsed_posts = Vec::new();

	for post in json.data.children {
		if post.kind != "t3"
			|| !post.data.is_robot_indexable
			|| post.data.stickied
			|| post.data.quarantine
			|| post.data.locked
			|| post.data.is_video
			|| post.data.upvote_ratio < config.reddit.min_ratio.unwrap_or(0.0)
		{
			continue;
		}

		let media = if let Some(m) = convert_to_media_url(&post.data.url) {
			m
		} else {
			continue;
		};

		parsed_posts.push(Post {
			title: post.data.title,
			static_id: post.data.name,
			link: "https://redd.it/".to_owned() + &post.data.id,
			media_url: media,
			sensitive: post.data.spoiler,
		});
	}

	Ok(parsed_posts)
}
