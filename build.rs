fn main() {
    // Embed the application icon in the Windows executable. The build
    // script runs on the host, so check the target explicitly.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("app-icon.rc", embed_resource::NONE);
    }
}
