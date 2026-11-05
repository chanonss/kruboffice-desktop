//! Windows: KrubOffice's icon and version information in `kruboffice.exe` (Explorer, the taskbar and
//! the Start Menu show it). Nothing to do elsewhere.
fn main() {
    println!("cargo:rerun-if-changed=packaging/windows/kruboffice.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/windows/kruboffice.ico").set("ProductName", "KrubOffice").set("FileDescription", "KrubOffice");
        if let Err(e) = res.compile() {
            println!("cargo:warning=KrubOffice icon not embedded: {e}");
        }
    }
}
