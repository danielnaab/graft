---
status: working
tags: [billing]
owner: payments-team
---

# Billing API

Subscription and payment handling service.

## Overview

The Billing API manages subscriptions, invoices, and payment methods. Currently in active development.

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /subscriptions | List subscriptions |
| POST | /subscriptions | Create subscription |
| GET | /invoices | List invoices |
| POST | /payments | Process payment |

## Status

This API is under active development. Breaking changes may occur.

### Planned additions

- Webhook events for payment status
- Subscription pause/resume
- Usage-based billing support

## Sources

- [Billing system design doc](https://docs.example.com/billing-design)
