---
title: "ADR 006: Typer for CLI Framework"
status: stable
owners: [architecture-team]
---

# ADR 006: Typer for CLI Framework

## Status

Accepted

## Context

The service layer needs a command-line interface for user interaction. We need a CLI framework that is:

- Modern and Pythonic
- Type-safe
- Easy to use and extend
- Generates good help documentation automatically
- Integrates well with our service layer architecture

## Decision

Use **Typer** for building the command-line interface.

## Rationale

### Type-Hint Based Interface

Typer uses Python type hints to define CLI:

```python
import typer

app = typer.Typer()

@app.command()
def create(
    name: str,
    value: int,
    active: bool = True,
) -> None:
    """Create example entity.

    Args:
        name: Entity name
        value: Entity value
        active: Whether entity is active (default: True)
    """
    typer.echo(f"Creating: {name} with value {value}")
```

**No decorators with complex syntax** - just normal Python type hints!

### Automatic Help Generation

Typer automatically generates help from:
- Function docstrings
- Parameter type hints
- Default values

```bash
$ starter-cli create --help

Usage: starter-cli create [OPTIONS] NAME VALUE

  Create example entity.

Arguments:
  NAME   Entity name          [required]
  VALUE  Entity value         [required]

Options:
  --active / --no-active  Whether entity is active  [default: active]
  --help                  Show this message and exit
```

**Professional help** without manual configuration.

### Great Developer Experience

**Type safety**:
- Type hints validate at runtime
- IDE autocomplete for Typer API
- Type checkers verify correctness

**Intuitive API**:
```python
@app.command()
def greet(name: str, enthusiastic: bool = False) -> None:
    """Greet someone."""
    greeting = f"Hello {name}!"
    if enthusiastic:
        greeting += "!!"
    typer.echo(greeting)
```

**Clear error messages**:
```bash
$ starter-cli create invalid-name not-a-number
Error: Invalid value for 'VALUE': 'not-a-number' is not a valid integer.
```

### Built on Click

Typer is built on **Click**, the industry-standard CLI framework:

- Inherits Click's reliability and maturity
- Can drop down to Click for advanced features
- Compatible with Click plugins and extensions
- Battle-tested foundation

**Best of both worlds**: Modern API + proven reliability.

### Easy Integration with Services

Typer commands easily wire to service functions:

```python
import typer
from starter.services.context import ServiceContext
from starter.services import create_example
from starter.adapters import InMemoryRepository

app = typer.Typer()

def get_context() -> ServiceContext:
    """Build context with real adapters."""
    return ServiceContext(
        repository=InMemoryRepository(),
        config=Config.from_env(),
        logger=setup_logger(),
    )

@app.command()
def create(name: str, value: int) -> None:
    """Create example entity."""
    ctx = get_context()  # Build context
    entity = create_example(ctx, name=name, value=value)  # Call service
    typer.echo(f"Created: {entity.id}")  # Display result
```

**Thin adapter** - CLI just wires context and calls services.

### Rich Output Support

Typer integrates with **Rich** for beautiful terminal output:

```python
from rich.console import Console
from rich.table import Table

console = Console()

@app.command()
def list_items() -> None:
    """List all items."""
    ctx = get_context()
    items = list_examples(ctx)

    # Beautiful table output
    table = Table(title="Examples")
    table.add_column("ID")
    table.add_column("Name")
    table.add_column("Value")

    for item in items:
        table.add_row(str(item.id), item.name, str(item.value))

    console.print(table)
```

### Command Groups and Subcommands

Easy to organize commands:

```python
app = typer.Typer()

# Subcommand group
user_app = typer.Typer()
app.add_typer(user_app, name="user")

@user_app.command("create")
def create_user(email: str, name: str) -> None:
    """Create new user."""
    ...

@user_app.command("list")
def list_users() -> None:
    """List all users."""
    ...

# Usage:
# $ starter-cli user create test@example.com "Test User"
# $ starter-cli user list
```

### Modern Python

Typer embraces modern Python:
- Type hints (Python 3.6+)
- Enum support
- Pathlib for file paths
- Dataclasses for options
- Async support (optional)

```python
from enum import Enum
from pathlib import Path

class OutputFormat(str, Enum):
    JSON = "json"
    CSV = "csv"
    TABLE = "table"

@app.command()
def export(
    output: Path,
    format: OutputFormat = OutputFormat.JSON,
) -> None:
    """Export data to file."""
    typer.echo(f"Exporting to {output} in {format.value} format")
```

## Alternatives Considered

### Click

**Traditional approach**:
```python
import click

@click.command()
@click.argument('name')
@click.argument('value', type=int)
@click.option('--active/--no-active', default=True)
def create(name, value, active):
    """Create example entity."""
    click.echo(f"Creating: {name}")
```

**Pros**:
- Industry standard (very mature)
- Widely known
- Extensive ecosystem
- Very stable

**Cons**:
- Decorator-heavy syntax
- No type hints (uses decorators)
- Manual type specification
- More verbose

**Why not chosen**: Typer provides better developer experience with type hints while building on Click's reliability.

### argparse

**Standard library approach**:
```python
import argparse

parser = argparse.ArgumentParser(description='Create example entity')
parser.add_argument('name', help='Entity name')
parser.add_argument('value', type=int, help='Entity value')
parser.add_argument('--active', action='store_true', help='Active flag')

args = parser.parse_args()
```

**Pros**:
- Standard library (no dependencies)
- Well-documented
- Works everywhere

**Cons**:
- Verbose and imperative
- No type hints
- Manual help formatting
- Less intuitive API
- No automatic type conversion

**Why not chosen**: Typer provides much better DX with minimal dependencies.

### Python Fire

**Google's automatic CLI**:
```python
import fire

def create(name: str, value: int, active: bool = True):
    """Create example entity."""
    print(f"Creating: {name}")

if __name__ == '__main__':
    fire.Fire(create)
```

**Pros**:
- Automatic CLI from functions
- Very minimal code
- Interesting approach

**Cons**:
- Too magical (hard to control)
- Limited customization
- Unusual command syntax
- Less predictable behavior

**Why not chosen**: Too magical. Typer provides better control while still being simple.

### Cement

**Enterprise CLI framework**:
```python
from cement import App

class MyApp(App):
    class Meta:
        label = 'myapp'
        handlers = [MyController]
```

**Pros**:
- Full-featured
- Plugin system
- Configuration management

**Cons**:
- Very heavyweight
- Complex architecture
- Steep learning curve
- Overkill for most projects

**Why not chosen**: Too complex for our needs. Typer is simpler and sufficient.

## Consequences

### Positive

1. **Type-safe**: Type hints provide safety and IDE support
2. **Clean code**: Minimal boilerplate, readable functions
3. **Automatic help**: Professional documentation generated automatically
4. **Modern**: Embraces Python 3 features
5. **Good DX**: Intuitive API, clear errors
6. **Reliable**: Built on Click (battle-tested)
7. **Extensible**: Easy to add commands and groups

### Negative

1. **External dependency**: Requires `typer` package (and `click`)
2. **Learning curve**: New framework to learn (though simple)
3. **Less mature**: Newer than Click (released 2019)
4. **Breaking changes possible**: Still evolving (though stable)

### Mitigations

1. **Minimal dependency**: Typer is small and well-maintained
2. **Documentation**: Clear examples in guides
3. **Click fallback**: Can use Click features if needed
4. **Active development**: Maintained by FastAPI creator (reliable)

## Implementation

### Basic Setup

```python
# src/starter/cli/main.py

import typer
from starter.services.context import ServiceContext
from starter.services import create_example, list_examples

app = typer.Typer()

def get_context() -> ServiceContext:
    """Build production context."""
    return ServiceContext(...)

@app.command()
def create(name: str, value: int) -> None:
    """Create example entity."""
    ctx = get_context()
    entity = create_example(ctx, name=name, value=value)
    typer.echo(f"Created: {entity.id}")

@app.command("list")
def list_all() -> None:
    """List all entities."""
    ctx = get_context()
    entities = list_examples(ctx)
    for entity in entities:
        typer.echo(f"{entity.id}: {entity.name} = {entity.value}")

if __name__ == "__main__":
    app()
```

### Entry Point Configuration

In `pyproject.toml`:

```toml
[project.scripts]
starter-cli = "starter.cli.main:app"
```

### Usage

```bash
# Install package
uv pip install -e .

# Run CLI
starter-cli create "example" 100
starter-cli list

# Get help
starter-cli --help
starter-cli create --help
```

### Command Organization

```
cli/
├── __init__.py
├── main.py              # Main Typer app
└── commands/
    ├── __init__.py
    ├── user.py          # User commands
    └── order.py         # Order commands
```

```python
# cli/main.py
from starter.cli.commands import user, order

app = typer.Typer()
app.add_typer(user.app, name="user")
app.add_typer(order.app, name="order")
```

### Error Handling

```python
@app.command()
def create(name: str, value: int) -> None:
    """Create example entity."""
    try:
        ctx = get_context()
        entity = create_example(ctx, name=name, value=value)
        typer.echo(f"Created: {entity.id}")
    except ValidationError as e:
        typer.secho(f"Validation error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)
    except Exception as e:
        typer.secho(f"Unexpected error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=2)
```

## Best Practices

### 1. Keep CLI Layer Thin

CLI should just wire context and call services:

```python
@app.command()
def command(param: str) -> None:
    """Command description."""
    ctx = get_context()           # Build context
    result = service_function(ctx, param)  # Call service
    display_result(result)        # Display result
```

**Don't put business logic in CLI** - that belongs in services.

### 2. Use Type Hints

Always use type hints for parameters:

```python
# ✅ Good
def create(name: str, value: int, active: bool = True) -> None: ...

# ❌ Bad
def create(name, value, active=True): ...
```

### 3. Write Good Docstrings

Docstrings become help text:

```python
@app.command()
def create(name: str, value: int) -> None:
    """Create new example entity.

    This command creates an entity with the specified name and value.
    The entity will be persisted to the configured repository.

    Args:
        name: Unique name for the entity
        value: Numeric value (must be non-negative)
    """
```

### 4. Use Enums for Choices

```python
from enum import Enum

class LogLevel(str, Enum):
    DEBUG = "debug"
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"

@app.command()
def run(log_level: LogLevel = LogLevel.INFO) -> None:
    """Run with specified log level."""
    setup_logger(level=log_level.value)
```

## Related Decisions

- [ADR 004: Functional Services](004-functional-services.md) - Services called by CLI
- [ADR 005: Context Objects](005-context-objects.md) - Context built in CLI
- [CLI Usage Guide](../guides/cli-usage.md) - Usage documentation

## Sources

- [Typer Documentation](https://typer.tiangolo.com/) - Official documentation
- [Typer GitHub](https://github.com/tiangolo/typer) - Source code
- [Click Documentation](https://click.palletsprojects.com/) - Underlying framework
- [Rich Documentation](https://rich.readthedocs.io/) - Terminal output library
