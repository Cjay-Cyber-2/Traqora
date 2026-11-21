# Booking Lifecycle Failure Recovery - V1

This document defines the recovery procedures for booking lifecycle failure states in Traqora's v1 booking system.

## State Machine Overview

The booking contract (`contracts/packages/booking`) implements the following states:

```
Pending → Confirmed → Completed
    ↓          ↓
  Refunded   Refunded (before departure)
    ↓          ↓
 Cancelled  (split via settle_cancellation)
```

## Failure States and Recovery Procedures

### 1. Supplier Timeout

**State**: Booking stuck in `Pending` or `Confirmed` due to airline/supplier not responding.

**Detection**:
- Booking remains in `Pending` beyond configurable timeout (e.g., 24 hours)
- Booking in `Confirmed` but flight departure passed without `release_payment_to_airline`

**Recovery**:
- **Passenger-initiated**: Call `refund_passenger(booking_id)` if before departure time
- **Oracle-initiated**: Call `oracle_refund_airline_cancel(booking_id)` if airline confirms cancellation
- **Automated**: Refund automation contract monitors and processes eligible refunds

**Invariants**:
- Only passenger or authorized oracle can trigger refund
- Refund only allowed before departure timestamp
- Escrowed amount returned to passenger

### 2. Payment Pending

**State**: Booking created (`Pending`) but payment not yet received.

**Detection**:
- Booking status = `Pending`
- `amount_escrowed` = 0
- Time since creation exceeds payment window (configurable, default 1 hour)

**Recovery**:
- **Auto-expiry**: Booking can be cancelled by passenger via `cancel_booking` (wrapper for `refund_passenger` on pending)
- **Payment completion**: Passenger calls `pay_for_booking(booking_id)` with valid token transfer
- **Manual intervention**: Admin can force-cancel stale pending bookings

**Invariants**:
- No funds escrowed → no financial loss on expiry
- Idempotent payment: duplicate `pay_for_booking` calls panic with "Already paid or cancelled"
- Payment window enforced by frontend/backend, not on-chain (flexible)

### 3. Contract Pending

**State**: On-chain transaction submitted but not yet confirmed (Soroban transaction in mempool or pending inclusion).

**Detection**:
- Backend submitted transaction but no confirmation event received
- Booking state in backend ≠ on-chain state

**Recovery**:
- **Polling**: Backend polls for transaction confirmation via Soroban RPC
- **Idempotent submission**: Backend uses idempotency keys; duplicate submissions safe
- **State reconciliation**: Periodic job compares backend state with on-chain state
- **Manual override**: Admin can force state sync via admin panel

**Invariants**:
- Contract state is authoritative source of truth
- Events emitted on every state transition (see `BOOKING_EVENT_SCHEMA.md`)
- No double-spend: token transfers only succeed once

### 4. Partial Completion

**State**: Multi-booking batch operations where some succeed and others fail.

**Context**: `batch_complete_bookings` processes multiple bookings atomically per-item.

**Detection**:
- Batch transaction succeeds but individual booking results vary
- Events show mixed `released` and failed operations

**Recovery**:
- **Per-item retry**: Failed bookings can be retried individually via `release_payment_to_airline`
- **Compensation**: If airline payout failed, retry release; if passenger refund needed, call `refund_passenger`
- **Monitoring**: Event indexer alerts on partial batch failures

**Invariants**:
- Each booking in batch processed independently
- Failed items don't block successful ones
- Events emitted per booking for auditability

## Automated Recovery Systems

### Refund Automation Contract

`contracts/packages/refund_automation` monitors bookings and executes:
- Time-based refunds for expired pending bookings
- Departure-time-based refunds for unfulfilled confirmed bookings
- Oracle-triggered refunds for airline cancellations

### Oracle Integration

`contracts/packages/oracle` provides trusted off-chain signals:
- `oracle_release_payment`: Airline confirms flight completion
- `oracle_refund_airline_cancel`: Airline confirms cancellation

## Operational Procedures

### For Operators

1. **Monitoring Dashboard**: Track bookings by state, age, and failure rate
2. **Alerting**: 
   - Pending > 1 hour without payment
   - Confirmed > 24h past departure without release
   - Batch partial failure rate > 5%
3. **Manual Recovery**:
   - Use admin panel to view booking state
   - Trigger oracle actions if airline coordination needed
   - Force state reconciliation for stuck bookings

### For Developers

1. **Event-based reconciliation**: Subscribe to `(booking, *)` events, rebuild state
2. **Idempotency keys**: Use on all booking creation/payment submissions
3. **Testing**: Run integration tests covering all failure paths (see test files)

## Security Considerations

- **Access control**: Only passenger, airline, oracle, or admin can trigger state changes
- **Replay protection**: Idempotency keys on creation; nonces on token transfers
- **Front-running**: State checks before transitions prevent invalid moves
- **Audit trail**: All state changes emit events with actor, timestamp, booking_id

## Testing Coverage

Automated tests cover:
- Normal flow: `test_payment_escrow_flow`, `test_refund_flow`
- Payment failures: `test_pay_for_booking_nonexistent_should_panic`, `test_pay_for_booking_again_should_panic`
- Release failures: `test_release_payment_invalid_status_should_panic`
- Refund windows: `test_refund_passenger_window_and_status_checks`, `test_refund_passenger_window_closed_should_panic`
- Batch partial failure: `test_batch_complete_bookings_partial_failure` snapshot
- Wrapper functions: `test_cancel_and_complete_wrappers`

## Version Compatibility

- This recovery model is v1 scope only
- New failure modes require new contract actions (additive) or new topic pairs (breaking)
- Event schema versioning follows `BOOKING_EVENT_SCHEMA.md` rules

---

*Last updated: 2025 | Part of Traqora v1 Booking API documentation*