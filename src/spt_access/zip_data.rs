use crate::spt_access::{file_parser, FileType, InstallTarget};

pub struct ZipData<'a> {
	data: &'a [u8],
	hash: String,
	zip_path: &'a str,
	file_type: FileType,
}

impl<'a> ZipData<'a> {
	pub fn new(data: &'a [u8], zip_path: &'a str) -> Self{
		let hash = sha256::digest(data);
		let file_type = file_parser(&mut zip_path.as_ref());
		Self {
			hash,
			data,
			zip_path,
			file_type,
		}
	}
	pub fn get_hash(&self) -> &str {
		&self.hash
	}
	pub fn get_data(&self) -> &[u8] {
		self.data
	}
	pub fn get_path(&self) -> &str {
		self.zip_path
	}
	pub fn should_install(&self, target: &InstallTarget) -> bool {
		matches!(
			(&self.file_type, target),
			(FileType::Client, InstallTarget::Client) | (FileType::Server, _)
		)
	}
}
