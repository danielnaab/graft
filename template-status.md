# Template Status - Graft

This project was generated from the **Python Starter Template**.

## Generation Info

- **Template Version**: `145552b914a2b4270019fc4cc8d51db01cb88a96`
- **Generated On**: ``
- **Copier Version**: ``

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

All template patterns and guides are available in `../python-starter/docs/`:

- [Architecture Patterns](../python-starter/docs/architecture/)
- [Architectural Decisions](../python-starter/docs/decisions/)
- [Development Guides](../python-starter/docs/guides/)
- [Technical Reference](../python-starter/docs/reference/)

## Project Structure

This project uses the Python Starter Template architecture:

- **Functional service layer**: Services as pure functions
- **Protocol-based DI**: Structural typing for flexibility
- **Context objects**: Immutable dependency containers
- **Testing with fakes**: Prefer fakes over mocks

See [Template Architecture Overview](../python-starter/docs/architecture/overview.md) for details.

## Customization

Files protected from updates (you can modify freely):
- `src/graft/domain/entities.py`
- `src/graft/domain/value_objects.py`
- `src/graft/services/example_service.py`
- `src/graft/adapters/repository.py`
- `src/graft/cli/commands/example.py`
- `tests/unit/*.py`
- `tests/integration/*.py`
- `README.md`
- `docs/agents.md`
- `knowledge-base.yaml`

All other files may be updated when you run `copier update`.

## Need Help?

- **Template documentation**: `../python-starter/docs/`
- **Agent guidance**: `docs/agents.md`
- **Project documentation**: `docs/README.md`

## Template Source

- Repository: `/tmp/copier._vcs.clone._kw2lm3u`
- Reference: `145552b914a2b4270019fc4cc8d51db01cb88a96`
