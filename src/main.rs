#![warn(clippy::all)]
#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

mod config;
mod control;
mod media;
mod posts;
mod reddit;
mod twitter;

use clap::Parser;
use reqwest::blocking::Client;

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
	let (clock_sender, yield_receiver) = control::initalize_graceful_clock();

	loop {
		let mut sleep_seconds = 900;

		println!("Downloading posts from Reddit...");

		if let Ok(posts) = reddit::get_reddit_posts(&config, &http_client) {
			println!(
				"Downloaded {:?} usable posts, attempting to find a unique post...",
				posts.len()
			);

			if let Ok(Some(unique_post)) = posts::get_unique_post(&database, &http_client, posts) {
				println!(
					"Found unique post ({}, depth: {}) from Reddit, uploading to Twitter...",
					unique_post.link, unique_post.depth
				);

				if let Ok(uploaded_post) =
					twitter::upload_unique_post(&config, &database, &http_client, &unique_post)
				{
					println!(
						"Sucessfully uploaded post (id: {}) to Twitter",
						uploaded_post
					);
					sleep_seconds = 60 * unique_post.depth as u64;
				} else {
					posts::database_remove_unique_post(&database, &unique_post)
						.expect("Unable to talk to database!");
					eprintln!("Unable to upload post to Twitter!");
				}
			} else {
				eprintln!("Unable to find a unique post!");
			}
		} else {
			eprintln!("Unable to download posts from reddit!");
		}

		println!("Cleaning up old Twitter posts...");

		if let Ok(removed_posts) = twitter::delete_old_messages(&config, &database, &http_client) {
			println!("Removed {:?} old posts.\n", removed_posts.len());
		} else {
			eprintln!("Unable to delete old posts!");
		}

		clock_sender.send(sleep_seconds).expect("Thread communcation failed!");
		if yield_receiver.recv().expect("Thread communication failed!") {
			break
		}
	}
	println!("All active tasks have finished, exiting...");
}
