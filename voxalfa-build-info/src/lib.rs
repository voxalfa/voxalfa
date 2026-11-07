pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn long_version() -> &'static str {
    if matches!(option_env!("NIGHTLY_BUILD"), Some("1")) {
        concat!(
            env!("CARGO_PKG_VERSION"),
            "-nightly",
            " (",
            env!("VERGEN_GIT_SHA"),
            " ",
            env!("VERGEN_BUILD_DATE"),
            ")"
        )
    } else {
        VERSION
    }
}
