---
status: stable
tags: [core]
owner: platform-team
---

# Users API

Core user management service.

## Overview

The Users API provides CRUD operations for user accounts, authentication, and profile management.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /users | List users |
| GET | /users/:id | Get user by ID |
| POST | /users | Create user |
| PUT | /users/:id | Update user |
| DELETE | /users/:id | Delete user |

## Authentication

All endpoints require a valid JWT token in the Authorization header.

## Rate limits

- 1000 requests per minute per API key
- Burst limit: 100 requests

## Sources

- [OpenAPI spec](https://api.example.com/docs/users)
