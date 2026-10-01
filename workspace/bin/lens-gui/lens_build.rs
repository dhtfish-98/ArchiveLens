fn main() {
    println!("cargo:rerun-if-changed=archivelens.ico");
    #[cfg(windows)]
    {
        let mut lens_res = winresource::WindowsResource::new();
        lens_res.set_icon("archivelens.ico");
        let _ = lens_res.compile();
    }
}
