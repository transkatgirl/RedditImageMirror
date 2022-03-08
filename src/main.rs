#![warn(clippy::all)]
#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

mod config;
mod media;
mod posts;
mod reddit;
mod twitter;

use clap::Parser;
use reqwest::blocking::Client;

use std::{thread, time::Duration};

/// Mirrors image posts from Reddit to Twitter.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
	/// Filename for the program's configuration.
	#[clap(short, long, default_value_t = String::from("config.toml"))]
	config: String,

	/// Filename for the program's database.
	#[clap(short, long, default_value_t = String::from("posts.db"))]
	database: String,
}

fn main() {
	let args = Args::parse();

	let config = config::load_config_file(&args.config).expect("Unable to load config file!");
	let database = posts::load_database(&args.database).expect("Unable to load post database!");
	let http_client = Client::builder()
		.build()
		.expect("Unable to initalize HTTP client!");

	/*loop {}*/

	println!("\nDownloading posts from Reddit...");

	let posts = reddit::get_reddit_posts(&config, &http_client)
		.expect("Unable to download posts from Reddit!");

	println!(
		"Downloaded {:?} usable posts, attempting to find a unique post...",
		posts.len()
	);

	let unique_post = posts::get_unique_post(&database, &http_client, posts)
		.expect("Unable to find a unique post!")
		.expect("No unique posts found!");

	println!(
		"Found unique post ({}) from Reddit, uploading to Twitter...",
		unique_post.link
	);

	let uploaded_post = twitter::upload_unique_post(&config, &database, &http_client, &unique_post)
		.expect("Unable to upload post to Twitter!");

	println!(
		"Sucessfully uploaded post (id: {}) to Twitter",
		uploaded_post
	);

	println!("Cleaning up old Twitter posts...");

	let removed_posts = twitter::delete_old_messages(&config, &database, &http_client)
		.expect("Unable to delete old posts!");

	println!("Removed {:?} old posts.", removed_posts.len());
}
