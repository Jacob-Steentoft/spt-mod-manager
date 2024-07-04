use chrono::{DateTime, Utc};
use crate::shared_traits::TimeProvider;

#[derive(Debug, Default)]
pub struct Time {}

impl Time {
	pub fn new() -> Self {
		Self {}
	}
}

impl TimeProvider for Time {
	fn get_current_time(&self) -> DateTime<Utc> {
		Utc::now()
	}
}