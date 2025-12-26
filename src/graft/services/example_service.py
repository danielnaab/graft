"""Example service functions demonstrating functional service pattern.

Service functions are pure functions taking ServiceContext as first parameter.
Replace or customize these for your domain.
"""

from graft.domain.entities import Entity
from graft.domain.exceptions import EntityNotFoundError
from graft.domain.value_objects import EntityName, EntityValue
from graft.services.context import ServiceContext


def create_example(
    ctx: ServiceContext,
    name: str,
    value: int,
) -> Entity:
    """Create and persist a new entity.

    Args:
        ctx: Service context with dependencies
        name: Entity name (will be wrapped in EntityName value object)
        value: Entity value (will be wrapped in EntityValue value object)

    Returns:
        Created entity

    Raises:
        ValidationError: If name or value are invalid

    Example:
        >>> ctx = get_context()
        >>> entity = create_example(ctx, "Example", 100)
        >>> entity.name.text
        'Example'
    """
    # Create domain objects with validation
    entity_name = EntityName(text=name)
    entity_value = EntityValue(amount=value)

    # Create entity
    entity = Entity(name=entity_name, value=entity_value)

    # Persist via repository
    ctx.repository.save(entity)

    return entity


def get_example(
    ctx: ServiceContext,
    entity_id: str,
) -> Entity:
    """Retrieve entity by ID.

    Args:
        ctx: Service context with dependencies
        entity_id: Unique entity identifier

    Returns:
        Entity if found

    Raises:
        EntityNotFoundError: If entity doesn't exist

    Example:
        >>> ctx = get_context()
        >>> entity = get_example(ctx, "some-uuid")
    """
    entity = ctx.repository.get(entity_id)

    if entity is None:
        raise EntityNotFoundError(entity_id)

    return entity


def list_examples(
    ctx: ServiceContext,
) -> list[Entity]:
    """List all entities.

    Args:
        ctx: Service context with dependencies

    Returns:
        List of all entities (empty if none exist)

    Example:
        >>> ctx = get_context()
        >>> entities = list_examples(ctx)
        >>> len(entities)
        5
    """
    return ctx.repository.list_all()
