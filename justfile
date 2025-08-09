dev:
    cargo run

publish:
    cargo publish --registry crates-io

release-patch:
    cargo release patch --no-publish --execute

release-minor:
    cargo release minor --no-publish --execute

release-major:
    cargo release major --no-publish --execute