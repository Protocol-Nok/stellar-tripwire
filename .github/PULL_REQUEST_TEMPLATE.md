## What

<!-- What does this change, in one or two sentences? -->

## Related issue

<!-- Closes #... -->

## How this was tested

<!-- cargo test / node adapter run / manual steps -->

## Checklist

- [ ] `cargo fmt --all --check` and `cargo clippy --all-targets -- -D warnings` pass
- [ ] `cargo test` passes
- [ ] A new corpus entry names its CAP or a public sighting (`docs/checks.md`)
- [ ] A new check kind is documented in `docs/checks.md`
- [ ] A new adapter follows `docs/adapter-protocol.md` and does not special-case entry IDs
