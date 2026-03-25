fn main() {
    // Embed icon if it exists; skip gracefully otherwise
    let icon_path = "src/icons/icon.ico";
    if std::path::Path::new(icon_path).exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon(icon_path);
        res.compile().expect("Failed to compile Windows resources");
    }
}
