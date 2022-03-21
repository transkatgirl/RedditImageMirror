use std::{
	sync::{
		mpsc::{self, SyncSender, Receiver},
	},
	thread,
	time::{Duration,Instant},
};

pub fn initalize_graceful_clock() -> (SyncSender<u64>, Receiver<bool>) {
	let (yield_sender, yield_receiver) = mpsc::channel();
	let (clock_sender, clock_receiver) = mpsc::sync_channel(0);
	let clock_yield_sender = yield_sender.clone();

	ctrlc::set_handler(move || {
		println!("Waiting for all active tasks to finish...");
		let _ = yield_sender.send(true);
	})
	.expect("Unable to set signal handler!");

	thread::spawn(move || {
		loop {
			let init_time = Instant::now();
			let sleep_duration = Duration::from_secs(clock_receiver.recv().expect("Thread communication failed!"));
			let calculated_sleep_duration = sleep_duration.saturating_sub(init_time.elapsed());

			let displayed_sleep_duration = calculated_sleep_duration.as_secs() / 60;

			match displayed_sleep_duration {
				0 => println!("Next posting will be in less than a minute.\n"),
				1 => println!("Next posting will be in a minute\n"),
				_ => println!("Next posting will be in {} minutes.\n", displayed_sleep_duration),
			};

			thread::sleep(calculated_sleep_duration);
			clock_yield_sender.send(false).expect("Thread communcation failed!");
		}
	});

	(clock_sender, yield_receiver)
}