fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("packaging/assets/icon.ico");
        res.set("ProductName", "LogicCraft CAD");
        res.set("FileDescription", "LogicCraft CAD - Digital Logic Circuit Studio");
        res.set("LegalCopyright", "Craft Anything");
        res.set("FileVersion", "1.0.2.0");
        res.set("ProductVersion", "1.0.2.0");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {e}");
        }
    }
}
