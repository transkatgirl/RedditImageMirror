use std::{fs::OpenOptions, io::Read, io::Write, process};

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
	pub reddit: Reddit,
	pub twitter: Twitter,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reddit {
	pub subreddits: Vec<String>,
	pub min_ratio: Option<f32>,
	pub max_depth: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Twitter {
	pub api_key: String,
	pub api_secret_key: String,
	pub access_token: String,
	pub access_token_secret: String,
	pub max_tweet_age: Option<u64>,
}

pub fn load_config_file(file: &str) -> Result<Config, Box<dyn std::error::Error>> {
	let mut config_file = OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.open(file)?;

	let mut config_string = String::new();

	if config_file.read_to_string(&mut config_string)? == 0 {
		let example_config = Config {
			reddit: Reddit {
				subreddits: Vec::from(["all".to_string(), "popular".to_string()]),
				min_ratio: Some(0.85),
				max_depth: Some(40),
			},
			twitter: Twitter {
				api_key: "API Key".to_string(),
				api_secret_key: "API Secret Key".to_string(),
				access_token: "Access Token".to_string(),
				access_token_secret: "Access Token Secret".to_string(),
				max_tweet_age: Some(86400),
			},
		};

		eprintln!("Config file is empty or does not exist, creating example config...");

		config_file.write_all(toml::to_string(&example_config)?.as_ref())?;

		process::exit(1);
	}

	let config: Config = toml::from_str(&config_string)?;

	Ok(config)
}
