use rfd::FileDialog;


pub fn select_folder() {
    let _files = FileDialog::new()
    .add_filter("text", &["txt", "rs"])
    .add_filter("rust", &["rs", "toml"])
    .set_directory("/")
    .pick_file();
}

