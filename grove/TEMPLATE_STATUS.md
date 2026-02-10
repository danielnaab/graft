# Template Status - Grove

This project was generated from the **Rust Starter Template**.

## Generation Info

- **Template Version**: `None`
- **Generated On**: ``
- **Copier Version**: ``

## First Steps

After your first `cargo build`, commit the generated `Cargo.lock` — binary projects should track their lockfile.

## Template Configuration

Answers to template questions are stored in `.copier-answers.yml`.

To view your configuration:
```bash
cat .copier-answers.yml
```

## Updating from Template

The template may receive improvements over time. To update this project:

```bash
copier update --trust
```

**Before updating**:
1. Commit all local changes
2. Review the changelog (if available)
3. Test after updating

Copier will:
- Preserve files in `_skip_if_exists` (your customizations)
- Update template-managed files
- Show conflicts for manual resolution

## Template Documentation

All template patterns and guides are available in `.graft/rust-starter/docs/`:

- [Architecture Patterns](.graft/rust-starter/docs/architecture/)
- [Architectural Decisions](.graft/rust-starter/docs/decisions/)
- [Development Guides](.graft/rust-starter/docs/guides/)
- [Technical Reference](.graft/rust-starter/docs/reference/)

## Project Structure

This project uses the Rust Starter Template architecture:

- **Cargo workspace**: `crates/*` for libraries, root `src/` for binary
- **Library-first**: Business logic in library crates, thin CLI binary
- **Trait-based DI**: Traits define boundaries, fakes for testing
- **Error types**: `thiserror` in libraries, `anyhow` in binary

See [Architecture Overview](.graft/rust-starter/docs/architecture/architecture.md) for details.

## Customization

Files protected from updates (you can modify freely):
- `src/main.rs`
- `crates/grove-core/src/domain.rs`
- `crates/grove-core/src/error.rs`
- `crates/grove-core/src/traits.rs`
- `crates/grove-engine/src/service.rs`
- `tests/common/mod.rs`
- `tests/integration_test.rs`
- `README.md`
- `docs/agents.md`
- `knowledge-base.yaml`

All other files may be updated when you run `copier update`.

## Need Help?

- **Template documentation**: `.graft/rust-starter/docs/`
- **Agent guidance**: `docs/agents.md`
- **Project documentation**: `docs/README.md`

## Template Source

- Repository: `/home/coder/src/graft/.graft/rust-starter`
- Reference: `None`
