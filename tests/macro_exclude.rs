//! The `spa!` macro's `exclude:` arm must (a) keep excluded globs out of the
//! embedded asset set while (b) leaving every other invocation shape intact.
//! Each framework's macro expands separately, so each gets its own invocation.

use anycms_spa::spa;
use rust_embed::RustEmbed;

// The new shape: precompressed companions stay on disk (for the CDN/nginx
// lane) but are not embedded into the binary.
#[cfg(feature = "axum")]
spa!(ExcludedSpa, "tests/fixtures/site", exclude: ["*.gz", "*.br"]);
#[cfg(feature = "actix")]
spa!(ExcludedSpa, "tests/fixtures/site", exclude: ["*.gz", "*.br"]);
#[cfg(feature = "salvo")]
spa!(ExcludedSpa, "tests/fixtures/site", exclude: ["*.gz", "*.br"]);

// The old shapes must keep compiling — they forward to the terminal arm with
// an empty exclude list.
#[cfg(feature = "axum")]
spa!(PlainSpa, "tests/fixtures/site");
#[cfg(feature = "actix")]
spa!(PlainSpa, "tests/fixtures/site", { });
#[cfg(feature = "salvo")]
spa!(PlainSpa, "tests/fixtures/site", "/", ["index.html"], { });

fn embedded_paths<E: RustEmbed>() -> Vec<String> {
    E::iter().map(|p| p.to_string()).collect()
}

#[test]
fn exclude_globs_are_not_embedded() {
    let paths = embedded_paths::<ExcludedSpa>();
    assert!(paths.contains(&"index.html".to_string()));
    assert!(paths.contains(&"app.js".to_string()));
    assert!(paths.contains(&"data.json".to_string()));
    assert!(
        !paths.iter().any(|p| p.ends_with(".gz") || p.ends_with(".br")),
        "compressed companions leaked into the embed: {paths:?}"
    );
}

#[test]
fn legacy_shapes_still_embed_everything() {
    // No exclude → the companions ARE embedded (pre-0.7.2 behavior, e.g. for
    // static file servers that serve the .gz/.br directly).
    let paths = embedded_paths::<PlainSpa>();
    assert!(paths.contains(&"app.js.gz".to_string()));
    assert!(paths.contains(&"app.js.br".to_string()));
}
