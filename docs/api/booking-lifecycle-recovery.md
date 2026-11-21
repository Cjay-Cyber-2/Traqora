# Booking Lifecycle Failure Recovery (V1)

This document defines the standard recovery flows for failure states within the v1 Traqora booking API and smart contract lifecycle: supplier timeouts, pending payments, pending contracts, and partial completions.

## 1. Supplier Timeout
- **Symptom:** Flight seat reservation or supplier confirmation hangs or fails to return a response within the expected window.
- **Recovery Flow:** 
  1. The client or backend triggers reconciliation (`reconcile_and_reserve_seat_timeout_recovery`).
  2. If the supplier never registered the hold, the escrowed funds remain untouched or are automatically eligible for passenger refund.
  3. If partial confirmation occurred, the system rolls back or cancels the affected leg deterministically.

## 2. Payment Pending
- **Symptom:** Booking created in `pending` state but payment transfer/escrow (`pay_for_booking`) is not completed before the expiration window or network timeout.
- **Recovery Flow:**
  1. The booking remains in `pending` status with `amount_escrowed = 0`.
  2. Passengers can invoke `cancel_booking` or request a refund window abort, transitioning the booking to `refunded` or canceled.
  3. Expired pending bookings without escrowed funds cannot be confirmed after timeout thresholds.

## 3. Contract Pending
- **Symptom:** Contract deployment or initialization steps (such as token linking or receipt contract setup) stall or fail midway.
- **Recovery Flow:**
  1. Re-run initialization or initialization check guards (`require_not_initialized`).
  2. Ensure atomic deployment patterns via integration test fixtures (`new_env`, `register_contracts`).

## 4. Partial Completion
- **Symptom:** Batch operations or multi-leg journeys fail partway through execution (e.g., `batch_complete_bookings_partial_failure`, `batch_create_flights_partial_failure`).
- **Recovery Flow:**
  1. Soroban transaction atomicity ensures that valid items succeed while failing items raise deterministic errors or log partial failure results without corrupting global ledger state.
  2. Operators inspect event logs and retry failed sub-batches independently with correct idempotency keys.
