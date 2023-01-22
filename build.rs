#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.compile().unwrap();
    built::write_built_file().expect("Failed to acquire build-time information");
}

#[cfg(unix)]
fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");
}
