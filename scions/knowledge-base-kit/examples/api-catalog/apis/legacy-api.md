---
status: deprecated
tags: [legacy]
owner: platform-team
---

# Legacy API (Deprecated)

Old v1 API endpoints. Scheduled for removal.

## Deprecation notice

This API is deprecated and will be removed on 2025-06-01. Please migrate to the [Users API](users-api.md).

## Migration guide

| Legacy endpoint | New endpoint |
|-----------------|--------------|
| GET /v1/users | GET /users |
| POST /v1/user/create | POST /users |
| GET /v1/user/:id | GET /users/:id |

## Timeline

- 2024-12-01: Deprecation announced
- 2025-03-01: Read-only mode begins
- 2025-06-01: API shutdown

## Sources

- [Migration playbook](https://docs.example.com/v1-migration)
