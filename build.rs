//! Compiles the crest into the Windows executable.
//!
//! What Explorer draws for a `.exe`, and what a desktop shortcut inherits, is a
//! resource inside the binary rather than a file beside it — so it has to be put
//! there at build time. Everything else about the icon (the window, the task bar
//! while running, the macOS bundle) is handled elsewhere; see `wear_the_icon` and
//! `packaging/`.
//!
//! Failing here does not fail the build. An executable with the default icon is a
//! working executable, and a toolchain without the resource compiler is a thing
//! that happens on somebody else's machine.
fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=packaging/Copaimo.ico");
        if let Err(why) = winresource::WindowsResource::new()
            .set_icon("packaging/Copaimo.ico")
            .compile()
        {
            println!("cargo:warning=could not compile the icon in: {why}");
        }
    }
}
