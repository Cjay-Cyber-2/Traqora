# Booking Lifecycle Failure Recovery (v1)

This document defines the recovery mechanisms for v1 booking lifecycle failure states: supplier timeout, payment pending, contract pending, and partial completion.

## 1. Supplier Timeout
- **Symptom**: An airline or provider contract does not respond within the expected network or submission window during seat reservation or flight scheduling.
- **Recovery**: Operations or automated reconciliation scripts invoke timeout recovery helpers (`reconcile_and_reserve_seat_timeout_recovery`) or abort/retry idempotently using unique tracking keys. Unconfirmed reservation attempts expire or roll back cleanly without locking funds.

## 2. Payment Pending
- **Symptom**: A booking is created with status `pending`, but payment (`pay_for_booking`) is not submitted before the expiration window or network drop occurs.
- **Recovery**: Unpaid bookings remain in `pending` state with zero escrowed funds. Passengers may retry payment if within valid issuance limits, or call `cancel_booking` / `refund_passenger` to release any stale references.

## 3. Contract Pending
- **Symptom**: On-chain initialization or contract execution halts midway due to missing prerequisites or uninitialized dependencies (e.g. uninitialized tokens or receipt contracts).
- **Recovery**: Guards ensure contracts revert atomically (`Not initialized` / `Already initialized`). Operators verify pre-requisites via initialization checks and re-submit transactions with correct state dependencies.

## 4. Partial Completion
- **Symptom**: Batch operations (such as `batch_create_flights`, `batch_update_flight_status`, or `batch_complete_bookings`) succeed for some items while failing or timing out on others.
- **Recovery**: Contracts enforce strict batch size limits and transactional validation. Partial failures return structured error vectors identifying invalid indices or items, allowing safe retries for the failed subset without corrupting successfully committed records.
