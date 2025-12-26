"""Fake implementations for testing.

Fakes are in-memory implementations of protocols used in tests.
Prefer fakes over mocks for better test clarity and reliability.
"""

from tests.fakes.fake_repository import FakeRepository

__all__ = ["FakeRepository"]
