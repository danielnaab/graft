"""Domain exceptions - business rule violations."""


class DomainError(Exception):
    """Base exception for all domain errors.

    Use this for business rule violations and domain-specific errors.
    """


class ValidationError(DomainError):
    """Validation error - data doesn't meet business rules.

    Raised when:
    - Value objects receive invalid data
    - Entities fail validation
    - Business constraints are violated
    """


class EntityNotFoundError(DomainError):
    """Entity not found error.

    Raised when attempting to retrieve entity that doesn't exist.
    """

    def __init__(self, entity_id: str, entity_type: str = "Entity") -> None:
        """Initialize error with entity details.

        Args:
            entity_id: ID of entity that wasn't found
            entity_type: Type of entity (default: "Entity")
        """
        super().__init__(f"{entity_type} not found: {entity_id}")
        self.entity_id = entity_id
        self.entity_type = entity_type
