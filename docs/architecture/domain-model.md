---
title: Domain Model
status: stable
owners: [architecture-team]
---

# Domain Model

This document explains domain modeling patterns used in this template, including entities, value objects, and validation strategies.

## What is Domain Modeling?

**Domain modeling** is the practice of creating a conceptual model of your business domain using code structures that reflect real-world concepts.

**Goals**:
- Capture business concepts clearly
- Encode business rules
- Create a ubiquitous language (shared terminology)
- Separate business logic from infrastructure

## Entities vs Value Objects

### Entities

**Entity**: An object with a unique identity that persists over time.

**Characteristics**:
- Has unique identifier (ID)
- Mutable (can change over time)
- Equality based on ID, not attributes
- Lifecycle (created, modified, deleted)

**Example**: User, Order, Product

```python
from dataclasses import dataclass, field
from uuid import uuid4

@dataclass
class User:
    """User entity with unique identity.

    Identity is based on `id`, not attribute values.
    Two users with same name/email are different if IDs differ.
    """
    email: str
    name: str
    id: str = field(default_factory=lambda: str(uuid4()))
    created_at: datetime = field(default_factory=datetime.utcnow)
    is_active: bool = True

    def __post_init__(self):
        """Validate entity invariants."""
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email address")
        if not self.name.strip():
            raise ValueError("Name cannot be empty")

    def deactivate(self) -> None:
        """Deactivate user account (business logic)."""
        self.is_active = False

    def change_email(self, new_email: str) -> None:
        """Change user email with validation."""
        if not new_email or "@" not in new_email:
            raise ValueError("Invalid email address")
        self.email = new_email

    def __eq__(self, other) -> bool:
        """Equality based on ID, not attributes."""
        if not isinstance(other, User):
            return False
        return self.id == other.id

    def __hash__(self) -> int:
        """Hash based on ID for use in sets/dicts."""
        return hash(self.id)
```

### Value Objects

**Value Object**: An object defined entirely by its attributes, with no unique identity.

**Characteristics**:
- No unique identifier
- Immutable (frozen dataclass)
- Equality based on all attributes
- No lifecycle (created, used, discarded)

**Example**: Address, Money, DateRange

```python
from dataclasses import dataclass
from decimal import Decimal

@dataclass(frozen=True)
class Money:
    """Money value object.

    Immutable representation of monetary value.
    Equality based on amount and currency.
    """
    amount: Decimal
    currency: str = "USD"

    def __post_init__(self):
        """Validate value object invariants."""
        if self.amount < 0:
            raise ValueError("Amount cannot be negative")
        if not self.currency or len(self.currency) != 3:
            raise ValueError("Currency must be 3-letter code (e.g., USD)")

    def add(self, other: "Money") -> "Money":
        """Add money (same currency only)."""
        if self.currency != other.currency:
            raise ValueError(f"Cannot add {self.currency} and {other.currency}")
        return Money(amount=self.amount + other.amount, currency=self.currency)

    def multiply(self, factor: Decimal) -> "Money":
        """Multiply money by factor."""
        return Money(amount=self.amount * factor, currency=self.currency)


@dataclass(frozen=True)
class Address:
    """Address value object.

    Immutable address representation.
    """
    street: str
    city: str
    state: str
    zip_code: str
    country: str = "US"

    def __post_init__(self):
        """Validate address."""
        if not self.street or not self.city or not self.state:
            raise ValueError("Street, city, and state are required")
        if not self.zip_code or len(self.zip_code) != 5:
            raise ValueError("ZIP code must be 5 digits")

    def format_one_line(self) -> str:
        """Format address as single line."""
        return f"{self.street}, {self.city}, {self.state} {self.zip_code}"
```

### When to Use Each

| Use Case | Entity | Value Object |
|----------|--------|--------------|
| Has unique identity | ✅ | ❌ |
| Can be mutated | ✅ | ❌ (immutable) |
| Tracked over time | ✅ | ❌ |
| Equality by ID | ✅ | ❌ (by attributes) |
| Examples | User, Order, Product | Money, Address, DateRange |

## Using Dataclasses

This template uses Python **dataclasses** for domain objects.

### Why Dataclasses?

**Advantages**:
- Less boilerplate than regular classes
- Automatic `__init__`, `__repr__`, `__eq__`
- Frozen option for immutability
- Type hints enforced
- Pythonic and modern (Python 3.7+)

### Dataclass Features

**Basic dataclass**:
```python
from dataclasses import dataclass

@dataclass
class Product:
    name: str
    price: Decimal
    sku: str
```

**Automatic generation**:
```python
product = Product(name="Widget", price=Decimal("19.99"), sku="WID-001")

print(product)  # Product(name='Widget', price=Decimal('19.99'), sku='WID-001')

product == Product("Widget", Decimal("19.99"), "WID-001")  # True
```

**Frozen dataclass** (immutable):
```python
@dataclass(frozen=True)
class Money:
    amount: Decimal
    currency: str

money = Money(Decimal("100"), "USD")
money.amount = Decimal("200")  # ❌ FrozenInstanceError
```

**Field defaults and factories**:
```python
from dataclasses import dataclass, field
from uuid import uuid4

@dataclass
class Order:
    user_id: str
    items: list[Item]
    id: str = field(default_factory=lambda: str(uuid4()))
    status: str = "pending"
    tags: list[str] = field(default_factory=list)  # Mutable default
```

**Post-initialization** (validation):
```python
@dataclass
class User:
    email: str
    name: str

    def __post_init__(self):
        """Validate after initialization."""
        if "@" not in self.email:
            raise ValueError("Invalid email")
```

## Validation Strategies

### 1. Validation in `__post_init__`

**Pattern**: Validate in `__post_init__` after dataclass initialization.

```python
@dataclass
class User:
    email: str
    age: int

    def __post_init__(self):
        """Validate invariants."""
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email address")
        if self.age < 0 or self.age > 150:
            raise ValueError("Age must be between 0 and 150")
```

**Pros**:
- Simple and straightforward
- Validates on creation
- Catches errors early

**Cons**:
- No validation on later mutations (for mutable entities)
- Can't skip validation when needed

### 2. Properties with Setters

**Pattern**: Use properties to validate on assignment.

```python
@dataclass
class User:
    _email: str
    _age: int

    @property
    def email(self) -> str:
        return self._email

    @email.setter
    def email(self, value: str) -> None:
        if not value or "@" not in value:
            raise ValueError("Invalid email")
        self._email = value

    @property
    def age(self) -> int:
        return self._age

    @age.setter
    def age(self, value: int) -> None:
        if value < 0 or value > 150:
            raise ValueError("Age must be between 0 and 150")
        self._age = value
```

**Pros**:
- Validates on every assignment
- Good for mutable entities

**Cons**:
- More verbose
- Doesn't work with dataclasses' automatic `__init__`

### 3. Dedicated Validation Methods

**Pattern**: Explicit validation methods called from service layer.

```python
@dataclass
class Order:
    user_id: str
    items: list[Item]
    total: Decimal

def validate_order(order: Order) -> list[str]:
    """Validate order and return list of errors.

    Returns:
        List of validation errors (empty if valid)
    """
    errors = []

    if not order.user_id:
        errors.append("User ID required")

    if not order.items:
        errors.append("Order must have at least one item")

    if order.total < 0:
        errors.append("Total cannot be negative")

    expected_total = sum(item.price for item in order.items)
    if order.total != expected_total:
        errors.append(f"Total mismatch: {order.total} != {expected_total}")

    return errors

# Usage in service
def create_order(ctx: ServiceContext, order: Order) -> Order:
    """Create order with validation."""
    errors = validate_order(order)
    if errors:
        raise ValidationError("; ".join(errors))

    ctx.repository.save(order)
    return order
```

**Pros**:
- Clear separation of concerns
- Flexible (can return errors vs raising exceptions)
- Easy to test validation separately

**Cons**:
- Must remember to call validation
- Validation not automatic

### Recommended Approach

**Combine strategies**:
1. Use `__post_init__` for **invariants** (must always be true)
2. Use validation functions for **business rules** (context-dependent)

```python
@dataclass
class User:
    email: str
    age: int
    id: str = field(default_factory=lambda: str(uuid4()))

    def __post_init__(self):
        """Validate invariants (always enforced)."""
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email")
        if self.age < 0:
            raise ValueError("Age cannot be negative")

def can_purchase_alcohol(user: User, country: str) -> bool:
    """Business rule: age requirement varies by country."""
    min_age = {"US": 21, "UK": 18, "DE": 16}.get(country, 21)
    return user.age >= min_age
```

## Domain Logic in Entities

Entities can contain **business logic** as methods:

```python
@dataclass
class Order:
    """Order entity with business logic."""
    user_id: str
    items: list[Item]
    status: str = "pending"
    id: str = field(default_factory=lambda: str(uuid4()))

    def __post_init__(self):
        if not self.items:
            raise ValueError("Order must have items")

    def total(self) -> Decimal:
        """Calculate order total."""
        return sum(item.price * item.quantity for item in self.items)

    def add_item(self, item: Item) -> None:
        """Add item to order (business logic)."""
        if self.status != "pending":
            raise ValueError("Cannot modify completed order")
        self.items.append(item)

    def complete(self) -> None:
        """Mark order as complete."""
        if self.status != "pending":
            raise ValueError(f"Cannot complete order in {self.status} status")
        if not self.items:
            raise ValueError("Cannot complete empty order")
        self.status = "completed"

    def cancel(self, reason: str) -> None:
        """Cancel order with reason."""
        if self.status == "completed":
            raise ValueError("Cannot cancel completed order")
        self.status = "cancelled"
```

**Guidelines**:
- Put business logic that operates on entity's own data in the entity
- Keep services for orchestration across multiple entities
- Avoid external dependencies in entities (repositories, APIs, etc.)

## Immutability Patterns

### Frozen Dataclasses

For value objects, use `frozen=True`:

```python
@dataclass(frozen=True)
class DateRange:
    """Immutable date range value object."""
    start: date
    end: date

    def __post_init__(self):
        if self.start > self.end:
            raise ValueError("Start date must be before end date")

    def duration(self) -> int:
        """Calculate duration in days."""
        return (self.end - self.start).days

    def contains(self, check_date: date) -> bool:
        """Check if date is within range."""
        return self.start <= check_date <= self.end
```

### Copy-on-Write

For entities that need "immutable" updates:

```python
from dataclasses import replace

@dataclass
class User:
    email: str
    name: str
    id: str = field(default_factory=lambda: str(uuid4()))

def change_user_email(user: User, new_email: str) -> User:
    """Return new User with updated email (copy-on-write)."""
    if "@" not in new_email:
        raise ValueError("Invalid email")
    return replace(user, email=new_email)

# Usage
user = User(email="old@example.com", name="John")
updated = change_user_email(user, "new@example.com")

assert user.email == "old@example.com"      # Original unchanged
assert updated.email == "new@example.com"   # New instance
assert user.id == updated.id                # Same identity
```

## Example Domain Model Structure

```
domain/
├── __init__.py
├── entities.py          # Domain entities
├── value_objects.py     # Value objects
└── exceptions.py        # Domain exceptions
```

### entities.py

```python
"""Domain entities with identity and lifecycle."""

from dataclasses import dataclass, field
from datetime import datetime
from uuid import uuid4

@dataclass
class User:
    """User entity."""
    email: str
    name: str
    id: str = field(default_factory=lambda: str(uuid4()))
    created_at: datetime = field(default_factory=datetime.utcnow)
    is_active: bool = True

    def __post_init__(self):
        if not self.email or "@" not in self.email:
            raise ValueError("Invalid email")

    def deactivate(self) -> None:
        """Deactivate user."""
        self.is_active = False
```

### value_objects.py

```python
"""Immutable value objects."""

from dataclasses import dataclass
from decimal import Decimal

@dataclass(frozen=True)
class Money:
    """Monetary value."""
    amount: Decimal
    currency: str = "USD"

    def __post_init__(self):
        if self.amount < 0:
            raise ValueError("Amount cannot be negative")

    def add(self, other: "Money") -> "Money":
        if self.currency != other.currency:
            raise ValueError("Currency mismatch")
        return Money(self.amount + other.amount, self.currency)
```

### exceptions.py

```python
"""Domain-specific exceptions."""

class DomainException(Exception):
    """Base exception for domain errors."""
    pass

class ValidationError(DomainException):
    """Validation rule violated."""
    pass

class NotFoundError(DomainException):
    """Entity not found."""
    pass

class BusinessRuleViolation(DomainException):
    """Business rule violated."""
    pass
```

## Related Documentation

- [Functional Services](functional-services.md) - How services use domain objects
- [Testing Strategy](testing-strategy.md) - Testing domain logic
- [Architecture Overview](overview.md) - Domain layer in overall architecture

## Sources

- [Domain-Driven Design (DDD) - Eric Evans](https://www.domainlanguage.com/ddd/) - Domain modeling concepts
- [Implementing Domain-Driven Design - Vaughn Vernon](https://vaughnvernon.com/iddd/) - Practical DDD
- [Python Dataclasses](https://docs.python.org/3/library/dataclasses.html) - Official documentation
- [Architecture Patterns with Python](https://www.cosmicpython.com/book/chapter_01_domain_model.html) - Domain modeling in Python
- [Value Objects - Martin Fowler](https://martinfowler.com/bliki/ValueObject.html) - Value object pattern
