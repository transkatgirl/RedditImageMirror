use std::{io::Cursor, path::Path};

use image::{io::Reader as ImageReader, ImageOutputFormat};
use reqwest::blocking::Client;
use url::{Host, Url};

#[derive(Debug)]
pub struct HashedValidatedImage {
	pub hash: String,
	pub raw_image: Vec<u8>,
}

pub fn convert_to_media_url(url_str: &str) -> Option<String> {
	let mut url = Url::parse(url_str).ok()?;

	let path = Path::new(url.path());
	let ext = path.extension()?.to_ascii_lowercase();

	if ext == "png" || ext == "jpeg" || ext == "jpg" || ext == "webp" {
		return Some(url.to_string());
	} else if url.host() == Some(Host::Domain("imgur.com")) {
		url.set_host(Some("i.imgur.com")).ok()?;
		url.set_path(&(url.path().to_string() + ".jpg"));

		return Some(url.to_string());
	};

	None
}

pub fn url_to_hashed_image(
	client: &Client,
	url: &str,
) -> Result<HashedValidatedImage, Box<dyn std::error::Error>> {
	let mut response = client.get(url).send()?.error_for_status()?;

	let mut response_buffer: Vec<u8> = vec![];
	response.copy_to(&mut response_buffer)?;

	let cursor = Cursor::new(response_buffer);

	let image = ImageReader::new(cursor).with_guessed_format()?.decode()?;

	let hasher = img_hash::HasherConfig::new()
		.hash_size(8, 8)
		.preproc_dct()
		.hash_alg(img_hash::HashAlg::Mean)
		.to_hasher();

	let mut output_buffer: Vec<u8> = vec![];

	image.write_to(
		&mut Cursor::new(&mut output_buffer),
		ImageOutputFormat::Jpeg(95),
	)?;

	Ok(HashedValidatedImage {
		hash: hasher.hash_image(&image).to_base64(),
		raw_image: output_buffer,
	})
}
