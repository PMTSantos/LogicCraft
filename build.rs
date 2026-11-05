fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("packaging/assets/icon.ico");
        res.set("ProductName", "LogicCraft CAD");
        res.set("FileDescription", "LogicCraft CAD - Digital Logic Circuit Studio");
        res.set("LegalCopyright", "Craft Anything");
        res.set("FileVersion", "2.5.0.0");
        res.set("ProductVersion", "2.5.0.0");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {e}");
        }
    }
}
