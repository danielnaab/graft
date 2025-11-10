# 2. Layered architecture with separation of concerns

Date: 2025-11-10

## Status

Accepted

## Context

As Graft grows from initial stubs to a production-ready system, we need a maintainable architecture that:

1. Separates business logic from infrastructure concerns
2. Makes testing easier by enabling dependency injection
3. Allows different implementations (e.g., in-memory for testing, filesystem for production)
4. Keeps the codebase understandable as it scales
5. Supports the file-first, auditable workflow that is core to Graft's mission

The initial implementation mixed CLI handlers, file I/O, YAML parsing, and business logic in a single layer. This approach works for prototypes but creates maintenance challenges:

- Testing requires filesystem fixtures
- Business logic is coupled to CLI framework (Typer)
- No clear contracts between components
- Difficult to reason about what the system does vs. how it does it

## Decision

We will organize the codebase into four distinct layers following clean architecture / hexagonal architecture principles:

### 1. Domain Layer (`src/graft/domain/`)

The **core business logic** layer containing:

- **Entities**: Pure domain objects (e.g., `Artifact`, `GraftConfig`, `Derivation`, `Policy`)
- **Value objects**: Immutable types representing domain concepts (e.g., `Material`, `Output`, `Template`)
- **Business rules**: Domain logic independent of infrastructure

**Characteristics:**
- No dependencies on other layers
- Pure Python with dataclasses
- Immutable where possible (`frozen=True`)
- Framework-agnostic

**Example:** `domain/entities.py` defines `Artifact`, `GraftConfig`, etc.

### 2. Adapter Layer (`src/graft/adapters/`)

The **infrastructure** layer implementing **ports** (interfaces) for external dependencies:

- **Filesystem adapter**: File I/O operations (`LocalFileSystem`)
- **Configuration adapter**: YAML parsing and entity construction (`ConfigAdapter`)
- **Future adapters**: JSON output, DVC integration, provenance storage

**Characteristics:**
- Implements port interfaces (protocols)
- Converts between external formats and domain entities
- Testable via interface substitution

**Example:** `adapters/filesystem.py` defines `FileSystemPort` protocol and `LocalFileSystem` implementation.

### 3. Service Layer (`src/graft/services/`)

The **application** layer orchestrating use cases:

- **Use case implementations**: `ExplainService`, `RunService`, `StatusService`, etc.
- **Workflow coordination**: Combines domain entities and adapters
- **Business rules enforcement**: Ensures policies are respected

**Characteristics:**
- Depends on domain layer and adapter ports
- Receives adapters via constructor (dependency injection)
- Returns domain-specific result objects
- No knowledge of CLI or presentation concerns

**Example:** `services/explain.py` defines `ExplainService` that loads an artifact and produces `ExplainResult`.

### 4. CLI Layer (`src/graft/cli.py`)

The **presentation** layer handling user interaction:

- **Command handlers**: Typer commands that parse arguments
- **Output formatting**: JSON vs. human-readable output
- **Error handling**: Converting exceptions to user-friendly messages
- **Dependency injection**: Wiring up adapters and services

**Characteristics:**
- Thin layer delegating to services
- Framework-specific (Typer)
- Handles presentation concerns only

**Example:** `cli.py` creates services and delegates commands to them.

### Dependency Flow

```
CLI Layer (presentation)
    ↓ depends on
Service Layer (use cases)
    ↓ depends on
Domain Layer (business logic) ← Adapter Layer (infrastructure)
```

**Key principle**: Dependencies point inward. Domain knows nothing about adapters, services, or CLI.

## Consequences

**Positive:**

- **Testability**: Each layer can be tested independently
  - Domain: Pure unit tests with no mocks
  - Services: Test with in-memory adapters
  - Adapters: Test contracts with real implementations
  - CLI: Integration tests via subprocess
- **Flexibility**: Swap implementations (e.g., cloud storage adapter)
- **Clarity**: Clear boundaries make it obvious where code belongs
- **Maintainability**: Changes to infrastructure don't affect business logic
- **Agent-friendly**: Well-structured code is easier for AI agents to understand and modify
- **Vertical slices**: Each feature can be implemented across all layers incrementally

**Negative:**

- **Initial overhead**: More files and structure for simple features
- **Learning curve**: Contributors need to understand the layering
- **Indirection**: More hops from CLI to implementation

**Neutral:**

- **File count increases**: One concept may span multiple files
- **Boilerplate**: Protocols, dataclasses, and result types add lines of code
- **Consistency required**: All features should follow this pattern

## Implementation Notes

- Start with Slice 0 (`explain` command) as the reference implementation
- Subsequent slices will follow this pattern
- Use Python protocols (`typing.Protocol`) for ports to enable static type checking
- Services receive adapters via constructor (explicit dependency injection)
- Result objects (e.g., `ExplainResult`) provide `.to_dict()` for JSON serialization
- Keep adapters focused: one adapter per external concern
