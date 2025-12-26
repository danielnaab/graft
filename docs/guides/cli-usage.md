---
title: CLI Usage
status: stable
owners: [documentation-team]
---

# CLI Usage

This guide covers using the command-line interface provided by the Python Starter Template.

## Overview

The template includes a CLI built with Typer that demonstrates how to wire service functions to command-line commands.

## Installation

```bash
# Install package (creates starter-cli command)
uv pip install -e .

# Or run via uv
uv run starter-cli --help
```

## Basic Usage

### Getting Help

```bash
# Main help
uv run starter-cli --help

# Command-specific help
uv run starter-cli create --help
uv run starter-cli list --help
```

### Available Commands

```bash
# Create example entity
uv run starter-cli create <name> <value>

# List all entities
uv run starter-cli list
```

## Command Reference

### create

Create a new example entity.

**Usage**:
```bash
uv run starter-cli create NAME VALUE
```

**Arguments**:
- `NAME`: Entity name (string, required)
- `VALUE`: Entity value (integer, required)

**Examples**:
```bash
# Create entity with name "test" and value 100
uv run starter-cli create "test" 100

# Create with different values
uv run starter-cli create "example-1" 42
uv run starter-cli create "example-2" 999
```

**Output**:
```
Created entity: 550e8400-e29b-41d4-a716-446655440000
Name: test
Value: 100
```

**Errors**:
```bash
# Negative value
$ uv run starter-cli create "test" -1
Error: Value must be non-negative

# Missing arguments
$ uv run starter-cli create
Error: Missing argument 'NAME'
```

### list

List all entities.

**Usage**:
```bash
uv run starter-cli list
```

**Examples**:
```bash
# List all entities
uv run starter-cli list
```

**Output**:
```
Entities (2):
- 550e8400-e29b-41d4-a716-446655440000: test (value=100)
- 6ba7b810-9dad-11d1-80b4-00c04fd430c8: example (value=42)
```

**Empty list**:
```
No entities found
```

## Exit Codes

The CLI uses standard exit codes:

- `0`: Success
- `1`: Validation error or business rule violation
- `2`: Unexpected error

**Example**:
```bash
uv run starter-cli create "test" -1
echo $?  # Prints: 1 (validation error)
```

## Output Formatting

### Standard Output

Normal output goes to stdout:

```bash
uv run starter-cli list > entities.txt
```

### Error Output

Errors go to stderr:

```bash
uv run starter-cli create "test" -1 2> errors.txt
```

### Colored Output

The CLI uses colored output for better readability:

- **Green**: Success messages
- **Red**: Error messages
- **Yellow**: Warnings

Disable colors:
```bash
NO_COLOR=1 uv run starter-cli create "test" 100
```

## Using from Python

You can also use the CLI programmatically:

```python
from starter.cli.main import app

# Run CLI
if __name__ == "__main__":
    app()
```

## Extending the CLI

### Adding a New Command

1. **Create command function** in `src/starter/cli/commands/`:

```python
# src/starter/cli/commands/example.py

import typer
from starter.services.context import ServiceContext
from starter.services import example_service

app = typer.Typer()

@app.command()
def get(entity_id: str) -> None:
    """Get entity by ID."""
    ctx = get_context()
    entity = example_service.get_example(ctx, entity_id)

    if not entity:
        typer.secho(f"Entity not found: {entity_id}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)

    typer.echo(f"ID: {entity.id}")
    typer.echo(f"Name: {entity.name}")
    typer.echo(f"Value: {entity.value}")
```

2. **Register command** in `src/starter/cli/main.py`:

```python
from starter.cli.commands import example

app.add_typer(example.app, name="example")
```

3. **Use new command**:

```bash
uv run starter-cli example get <entity-id>
```

### Adding Command Groups

Group related commands:

```python
# src/starter/cli/commands/user.py

import typer

app = typer.Typer()

@app.command("create")
def create_user(email: str, name: str) -> None:
    """Create new user."""
    ...

@app.command("list")
def list_users() -> None:
    """List all users."""
    ...

# Register in main.py
from starter.cli.commands import user
app.add_typer(user.app, name="user")
```

**Usage**:
```bash
uv run starter-cli user create "test@example.com" "Test User"
uv run starter-cli user list
```

### Adding Options

Use Typer's option system:

```python
from typing import Optional

@app.command()
def create(
    name: str,
    value: int,
    description: Optional[str] = typer.Option(None, "--description", "-d", help="Entity description"),
    verbose: bool = typer.Option(False, "--verbose", "-v", help="Verbose output"),
) -> None:
    """Create entity with options."""
    if verbose:
        typer.echo(f"Creating entity: {name}")

    # ... create logic ...

    if description:
        typer.echo(f"Description: {description}")
```

**Usage**:
```bash
uv run starter-cli create "test" 100 --description "Test entity"
uv run starter-cli create "test" 100 -d "Test entity" -v
```

### Adding Arguments Validation

```python
def validate_positive(value: int) -> int:
    """Validate that value is positive."""
    if value < 0:
        raise typer.BadParameter("Value must be positive")
    return value

@app.command()
def create(
    name: str,
    value: int = typer.Argument(..., callback=validate_positive),
) -> None:
    """Create with validated value."""
    ...
```

## Environment Configuration

### Environment Variables

The CLI respects environment variables:

```bash
# Set log level
export LOG_LEVEL=DEBUG

# Set environment
export ENVIRONMENT=development

# Run CLI
uv run starter-cli create "test" 100
```

### Configuration File

Load from `.env` file:

```bash
# .env
DATABASE_URL=postgresql://localhost/starter
LOG_LEVEL=INFO
```

```python
from dotenv import load_dotenv
load_dotenv()
```

## Scripting

### Batch Operations

```bash
#!/bin/bash
# create_test_data.sh

for i in {1..10}; do
    uv run starter-cli create "entity-$i" "$i"
done

uv run starter-cli list
```

### Error Handling

```bash
#!/bin/bash

if uv run starter-cli create "test" 100; then
    echo "Success!"
else
    echo "Failed with exit code: $?"
    exit 1
fi
```

### Parsing Output

```bash
# Get list of entities
entities=$(uv run starter-cli list)

# Parse output
echo "$entities" | grep "test"
```

## Debugging

### Verbose Mode

Add verbose flag to commands:

```python
@app.command()
def create(
    name: str,
    value: int,
    verbose: bool = typer.Option(False, "--verbose", "-v"),
) -> None:
    """Create with verbose output."""
    if verbose:
        typer.echo(f"Creating entity...")
        typer.echo(f"Name: {name}")
        typer.echo(f"Value: {value}")
```

### Stack Traces

Show full stack traces on errors:

```python
import sys
import traceback

@app.command()
def create(name: str, value: int) -> None:
    """Create with error handling."""
    try:
        # ... logic ...
    except Exception as e:
        if "--debug" in sys.argv:
            traceback.print_exc()
        typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)
```

## Best Practices

### 1. Keep CLI Thin

CLI should only:
- Parse input
- Build context
- Call service functions
- Format output

**Good**:
```python
@app.command()
def create(name: str, value: int) -> None:
    """Create entity."""
    ctx = get_context()  # Build context
    entity = create_example(ctx, name, value)  # Call service
    typer.echo(f"Created: {entity.id}")  # Format output
```

**Bad**:
```python
@app.command()
def create(name: str, value: int) -> None:
    """Create entity - too much logic in CLI."""
    # ❌ Validation logic in CLI
    if value < 0:
        raise ValueError("...")

    # ❌ Database logic in CLI
    db = connect_database()
    db.insert(...)

    # ❌ Business logic in CLI
    if special_condition:
        do_complex_stuff()
```

### 2. Consistent Error Handling

```python
@app.command()
def command() -> None:
    """Command with consistent error handling."""
    try:
        ctx = get_context()
        result = service_function(ctx, ...)
        typer.echo(f"Success: {result}")
    except ValidationError as e:
        typer.secho(f"Validation error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)
    except NotFoundError as e:
        typer.secho(f"Not found: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)
    except Exception as e:
        typer.secho(f"Unexpected error: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=2)
```

### 3. Good Help Text

```python
@app.command()
def create(
    name: str = typer.Argument(..., help="Name of the entity to create"),
    value: int = typer.Argument(..., help="Numeric value (must be positive)"),
    description: Optional[str] = typer.Option(None, "--description", "-d", help="Optional description"),
) -> None:
    """Create a new example entity.

    This command creates an entity with the specified name and value.
    The entity will be persisted to the configured repository.

    Examples:

        # Create entity with name and value
        $ starter-cli create "my-entity" 100

        # Create with description
        $ starter-cli create "my-entity" 100 --description "Test entity"
    """
    ...
```

### 4. Use Type Hints

Always use type hints for automatic validation:

```python
from pathlib import Path
from enum import Enum

class OutputFormat(str, Enum):
    JSON = "json"
    CSV = "csv"
    TABLE = "table"

@app.command()
def export(
    output: Path,  # Automatically validated as path
    format: OutputFormat,  # Automatically validated as enum
    count: int,  # Automatically validated as integer
) -> None:
    """Export with type validation."""
    ...
```

## Related Documentation

- [Getting Started](getting-started.md) - Initial setup
- [Development Workflow](development-workflow.md) - Daily practices
- [Adding Features](adding-features.md) - Extending the CLI
- [ADR 006: Typer CLI](../decisions/006-typer-cli.md) - CLI framework choice

## Sources

- [Typer Documentation](https://typer.tiangolo.com/) - Official Typer docs
- [Click Documentation](https://click.palletsprojects.com/) - Underlying framework
- [Rich Documentation](https://rich.readthedocs.io/) - Terminal output formatting
