fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winres::WindowsResource::new()
            .set_icon("assets/com.tbx.translator.ico")
            .set("ProductVersion", env!("CARGO_PKG_VERSION"))
            .set("FileDescription", "TBX Translator")
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}
