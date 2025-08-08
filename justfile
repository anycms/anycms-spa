dev:
    cargo run

publish:
    cargo publish

release-patch:
    cargo release patch --execute

release-minor:
    cargo release minor --execute

release-major:
    cargo release major --execute