# Booking Lifecycle Failure Recovery Procedures

This document defines the automated and manual recovery procedures for failure states in the Traqora v1 booking lifecycle.

## Failure States and Recovery Actions

### 1. Supplier Timeout (Airline Non-Response)

**Trigger Condition**: Booking is in `Confirmed` status with funds escrowed, and the flight departure time has passed without the airline calling `release_payment_to_airline`.

**Automated Recovery**: `recover_supplier_timeout(booking_id)`
- **Preconditions**:
  - Booking status == `Confirmed`
  - `amount_escrowed` > 0
  - Current ledger timestamp >= `departure_time`
- **Action**: Transfers escrowed funds to airline, sets status to `Completed`, emits `(booking, released)` event
- **Idempotency**: Safe to retry; second call fails with `InvalidBookingStatus`

**Monitoring**: Off-chain indexers should flag `Confirmed` bookings past departure time for automated recovery.

### 2. Payment Pending Timeout

**Trigger Condition**: Booking stuck in `Pending` status beyond the 24-hour payment grace period.

**Automated Recovery**: `recover_payment_pending(booking_id)`
- **Preconditions**:
  - Booking status == `Pending`
  - Current ledger timestamp >= `created_at + 86400` (24 hours)
- **Action**: Sets status to `Cancelled`, emits `(booking, cancelled)` event with zero refunds
- **Idempotency**: Safe to retry; second call fails with `InvalidBookingStatus`

**Monitoring**: Scheduler should scan for `Pending` bookings older than 24 hours every hour.

### 3. Contract Pending (On-Chain Submission Stuck)

**Trigger Condition**: Booking in `Confirmed` status but `amount_escrowed == 0`, indicating `pay_for_booking` was called but token transfer failed or reverted.

**Automated Recovery**: `recover_contract_pending(booking_id)`
- **Preconditions**:
  - Booking status == `Confirmed`
  - `amount_escrowed == 0`
- **Action**: Attempts to collect payment from passenger:
  - **Success**: Transfers tokens, sets `amount_escrowed = amount`, emits `(booking, paid)`
  - **Failure (insufficient balance)**: Sets status to `Cancelled`, emits `(booking, cancelled)`
- **Idempotency**: Safe to retry; collects payment only once

**Monitoring**: Indexers should flag `Confirmed` bookings with zero escrow for recovery.

### 4. Partial Completion (Multi-Leg Journey)

**Trigger Condition**: A multi-leg booking where only some legs were fulfilled (e.g., connecting flight cancelled).

**Manual/Automated Recovery**: `recover_partial_completion(booking_id, completed_legs, total_legs)`
- **Preconditions**:
  - Booking status == `Confirmed`
  - `amount_escrowed` > 0
  - `0 < completed_legs <= total_legs`
- **Action**: Proportionally splits escrow:
  - Airline receives: `amount_escrowed * completed_legs / total_legs`
  - Passenger refunded: `amount_escrowed - airline_share`
  - Sets status to `Completed`, `amount_escrowed = 0`
  - Emits `(booking, released)` for airline share, `(booking, refunded)` for passenger share
- **Idempotency**: Safe to retry; second call fails with `InvalidBookingStatus`

**Note**: This requires off-chain determination of completed legs (oracle or manual review).

## Recovery Status Monitoring

### `get_recovery_status(booking_id)`

Off-chain monitoring tool to detect bookings needing recovery:

| Booking Status | Condition | RecoveryStatus |
|---|---|---|
| `Pending` | `now >= created_at + 24h` | `PaymentPendingTimeout` |
| `Confirmed` | `amount_escrowed == 0` | `ContractPending` |
| `Confirmed` | `now >= departure_time` | `SupplierTimeout` |
| Other | — | `Normal` |

**Recommended Polling Interval**: Every 15 minutes for active bookings.

## Event Schema for Recovery Actions

All recovery functions emit events for auditability:

| Recovery Action | Event Topics | Data Fields |
|---|---|---|
| `recover_supplier_timeout` | `(booking, released)` | `(airline, timestamp, booking_id)` |
| `recover_payment_pending` | `(booking, cancelled)` | `(passenger, timestamp, booking_id, 0, 0)` |
| `recover_contract_pending` (success) | `(booking, paid)` | `(passenger, timestamp, booking_id, amount)` |
| `recover_contract_pending` (failure) | `(booking, cancelled)` | `(passenger, timestamp, booking_id, 0, 0)` |
| `recover_partial_completion` | `(booking, released)` + `(booking, refunded)` | Split amounts per above |

## Security Considerations

1. **Authorization**: All recovery functions are permissionless (anyone can call) but guarded by strict state preconditions.
2. **Funds Safety**: Recovery only moves funds according to predefined rules; no arbitrary transfers.
3. **Replay Protection**: State transitions prevent double-recovery (status changes block re-entry).
4. **Audit Trail**: All recovery actions emit events with actor, timestamp, and booking ID.

## Operational Runbook

### Automated Recovery Cron Jobs

```yaml
# Example scheduler configuration
jobs:
  - name: supplier-timeout-recovery
    schedule: "*/15 * * * *"  # Every 15 minutes
    action: scan_confirmed_past_departure -> call recover_supplier_timeout
  
  - name: payment-pending-timeout
    schedule: "0 * * * *"  # Hourly
    action: scan_pending_older_than_24h -> call recover_payment_pending
  
  - name: contract-pending-recovery
    schedule: "*/30 * * * *"  # Every 30 minutes
    action: scan_confirmed_zero_escrow -> call recover_contract_pending
```

### Manual Intervention

For `recover_partial_completion`:
1. Operations team verifies completed vs total legs via airline systems
2. Calls `recover_partial_completion` with verified counts
3. Confirms events emitted and funds distributed correctly

### Escalation

If automated recovery fails (returns error):
1. Check event logs for `(booking, recovery)` events
2. Verify booking state on-chain
3. Contact airline/passenger for off-chain resolution
4. If funds stuck, use admin multisig for emergency extraction (see Admin Multisig docs)

## Testing

All recovery paths covered by integration tests in `contracts/packages/integration-tests/tests/booking_errors_test.rs`:
- `test_recover_supplier_timeout_success`
- `test_recover_supplier_timeout_wrong_status`
- `test_recover_supplier_timeout_before_departure`
- `test_recover_payment_pending_timeout`
- `test_recover_payment_pending_within_grace_period`
- `test_recover_contract_pending_payment_succeeds`
- `test_recover_contract_pending_payment_fails_cancels`
- `test_recover_partial_completion`
- `test_recover_partial_completion_invalid_legs`
- `test_get_recovery_status`

Run with: `cd contracts && cargo test --package integration-tests booking_errors_test::test_recover_`