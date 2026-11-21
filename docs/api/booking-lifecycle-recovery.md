# Booking Lifecycle Failure Recovery Guide (v1)

This document defines the recovery procedures and operational guidelines for booking lifecycle failures within v1 scope: supplier timeout, payment pending, contract pending, and partial completion states.

## States & Recovery Flows

### 1. Supplier Timeout
- **Symptom:** The airline or external partner service does not acknowledge seat reservation or booking confirmation within the expected network timeout window.
- **Recovery Procedure:** 
  - Client/Backend triggers reconciliation query or idempotent retry against the registry.
  - If unacknowledged, the transaction aborts safely or rolls back, releasing any temporary holds.
  - Automated tests: `test_reconcile_and_reserve_seat_timeout_recovery` in `airline_test.rs`.

### 2. Payment Pending
- **Symptom:** A booking is created in `pending` status, but the passenger has not invoked `pay_for_booking` or the token transfer escrow transaction is delayed/dropped.
- **Recovery Procedure:**
  - Passengers may re-submit the payment transaction using the existing booking ID while within the allowed expiry window.
  - Expired pending bookings without escrow can be cancelled or automatically cleaned up by operational workflows.

### 3. Contract Pending
- **Symptom:** Smart contract deployment or initialization state is pending confirmation on-chain.
- **Recovery Procedure:**
  - Re-verify contract instance state and storage version before proceeding with downstream booking creation or receipt minting.

### 4. Partial Completion
- **Symptom:** Batch operations (such as batch booking completions, batch flight creation, or batch status updates) experience partial failures where some items succeed and others fail.
- **Recovery Procedure:**
  - Soroban smart contracts enforce atomic or clearly bounded error handling where invalid items fail explicitly without corrupting successfully processed batch items (e.g., `test_batch_complete_bookings_partial_failure`).
  - Indexers and operators inspect event logs for failure reasons and retry failed subset items idempotently.
