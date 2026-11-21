# Booking Lifecycle Failure Recovery — Operational Runbook

> **Scope:** v1 traveler booking and payment flow.
> **Audience:** On-call engineers, platform operators, support escalation.
> **Last reviewed:** {{DATE}}

---

## 1. Overview

This runbook defines recovery procedures for four transient failure states that can occur during the v1 booking lifecycle:

| State | Trigger | Typical Duration | Auto-recovery | Manual SLA |
|-------|---------|------------------|---------------|------------|
| **Supplier Timeout** | Airline/oracle does not respond to seat reservation or status update within SLA | < 5 min | Yes (retry + fallback) | 15 min |
| **Payment Pending** | Stripe payment intent created but not confirmed/failed within window | < 10 min | Yes (webhook reconciliation) | 30 min |
| **Contract Pending** | Soroban transaction submitted but not confirmed (included/rejected) | < 2 min | Yes (resubmit with bump) | 10 min |
| **Partial Completion** | Multi-step flow (e.g., batch complete) succeeds for subset of bookings | Immediate | Yes (idempotent retry) | 5 min per item |

All procedures assume:
- **Idempotency keys** are used on every mutating API call.
- **Event sourcing** via `contract_events` topics is the source of truth.
- **No manual DB edits** — recovery is driven by re-invoking contract entry points or backend jobs.

---

## 2. Supplier Timeout

### 2.1 State Definition

A supplier timeout occurs when the airline contract (or oracle acting on its behalf) fails to respond to:
- `reserve_seat` / `batch_reserve_seats`
- `update_flight_status` / `batch_update_flight_status`
- `reconcile_and_reserve_seat_timeout_recovery`

within the configured deadline (default: **120 seconds** for seat reservation, **60 seconds** for status updates).

**Booking status remains `pending`** — no escrow movement has occurred.

### 2.2 Detection

| Signal | Source | Query |
|--------|--------|-------|
| `reserve_seat` call returns `Timeout` error | Backend API logs | `grep "Timeout" api.log \| grep reserve_seat` |
| No `(airline, seat_reserved)` event within 120 s of `create_booking` | Indexer / contract events | `SELECT * FROM events WHERE topic0='airline' AND topic1='seat_reserved' AND booking_id=?` |
| `reconcile_and_reserve_seat_timeout_recovery` cron job finds stale `pending` bookings > 5 min | Cron job logs / metrics | `booking_stale_pending_total{state="supplier_timeout"}` |
| Alert: `supplier_timeout_rate > 0.05` for 5 min | Prometheus / Alertmanager | `rate(supplier_timeout_total[5m]) > 0.05` |

### 2.3 Automated Recovery

1. **Immediate retry (attempt 1/3):** Backend re-invokes `reserve_seat` with same idempotency key.
2. **Exponential backoff (attempt 2/3):** Wait 30 s, retry with `reconcile_and_reserve_seat_timeout_recovery` (airline contract).
3. **Fallback to alternate inventory (attempt 3/3):** If airline supports multiple fare classes, attempt adjacent class; else mark booking `failed_supplier` and trigger refund flow.
4. **Emit `(booking, supplier_timeout_recovery)` event** with fields: `booking_id`, `attempt`, `outcome`, `fallback_used`.

> **Idempotency:** All retries use the original `idempotency_key` from `create_booking`. The airline contract's `reserve_seat` is idempotent by `(flight_id, seat, idempotency_key)`.

### 2.4 Manual Intervention

| Condition | Action | Owner |
|-----------|--------|-------|
| All 3 retries exhausted, no fallback | 1. Verify airline API health via `/health` endpoint.
2. If airline degraded: escalate to airline ops contact.
3. If airline healthy: investigate contract gas/fee issues.
4. If unrecoverable: `cancel_booking` → `refund_passenger` (full). | On-call engineer |
| Fallback class used | Notify passenger via email/SMS with new seat/class details. | Support automation |
| Pattern: > 10 timeouts/hour for same airline | Open incident, engage airline technical contact. | Platform lead |

### 2.5 Escalation Path

```
L1 On-call (0-15 min) → L2 Platform Engineer (15-30 min) → Airline Integration Lead (30-60 min) → VP Engineering (>60 min)
```

### 2.6 Monitoring & Alerting

| Metric | Warning | Critical | Dashboard |
|--------|---------|----------|-----------|
| `supplier_timeout_total` | > 5/min | > 20/min | `traqora/booking/supplier-timeouts` |
| `supplier_timeout_recovery_success_rate` | < 90% | < 70% | `traqora/booking/recovery` |
| `booking_stale_pending_total{state="supplier_timeout"}` | > 50 | > 200 | `traqora/booking/stale` |
| Airline API latency (p99) | > 5 s | > 15 s | `traqora/airline/latency` |

---

## 3. Payment Pending

### 3.1 State Definition

Payment is **pending** when:
- Stripe `PaymentIntent` created (`status=requires_payment_method` or `processing`)
- Backend has stored `payment_intent_id` on booking record
- **No** `payment_intent.succeeded` or `payment_intent.payment_failed` webhook received
- Booking status = `pending_payment` (escrow not yet funded)

Default pending window: **10 minutes** (configurable per payment method).

### 3.2 Detection

| Signal | Source | Query |
|--------|--------|-------|
| Booking in `pending_payment` > 10 min | Backend cron / DB | `SELECT * FROM bookings WHERE status='pending_payment' AND created_at < NOW() - INTERVAL '10 min'` |
| Stripe webhook `payment_intent.processing` received but no terminal event after 8 min | Webhook logs / metrics | `payment_webhook_lag_seconds{intent_status="processing"} > 480` |
| Passenger reports "payment stuck" | Support ticket tag | `tag:payment_pending` |
| Alert: `payment_pending_stale_total > 20` | Prometheus | `payment_pending_stale_total > 20` |

### 3.3 Automated Recovery

1. **Webhook reconciliation job (runs every 60 s):**
   - Fetch `PaymentIntent` from Stripe API for all `pending_payment` bookings > 2 min.
   - If `status=succeeded`: call `pay_for_booking` (idempotent via `payment_intent_id`).
   - If `status=requires_payment_method` / `canceled`: mark booking `payment_failed`, release seat reservation via `cancel_seat_reservation`, emit `(booking, payment_failed)`.
   - If `status=processing`: wait (do not retry yet).
2. **Stripe auto-cancel:** PaymentIntents auto-cancel after 24 h; job handles this as above.
3. **Emit `(booking, payment_reconciled)`** with `booking_id`, `stripe_status`, `action_taken`.

> **Idempotency:** `pay_for_booking` uses `payment_intent_id` as idempotency key. Safe to call multiple times.

### 3.4 Manual Intervention

| Condition | Action | Owner |
|-----------|--------|-------|
| Stripe reports `succeeded` but `pay_for_booking` reverts (contract error) | 1. Check contract error code.
2. If `InsufficientAllowance`: mint/approve tokens for passenger.
3. If `InvalidBookingStatus`: verify booking not already paid/cancelled.
4. Re-invoke `pay_for_booking` manually via admin CLI. | On-call engineer |
| Stripe reports `requires_action` (3DS) | Passenger must complete 3DS challenge. Send reminder notification (email/push). If no action after 15 min → treat as failed. | Support automation |
| Discrepancy: Stripe `succeeded` but no webhook received after 30 min | 1. Verify Stripe dashboard.
2. Manually trigger reconciliation job for that `payment_intent_id`.
3. If webhook endpoint down: check logs, restart webhook receiver. | On-call engineer |
| Passenger requests cancel while pending | Call `cancel_booking` → releases seat, marks `cancelled`. Stripe PaymentIntent auto-cancels. | Support agent |

### 3.5 Escalation Path

```
L1 On-call (0-10 min) → L2 Payments Engineer (10-30 min) → Stripe Support (via dashboard) → VP Engineering (>30 min)
```

### 3.6 Monitoring & Alerting

| Metric | Warning | Critical | Dashboard |
|--------|---------|----------|-----------|
| `payment_pending_stale_total` | > 10 | > 50 | `traqora/payment/pending` |
| `payment_reconciliation_job_duration_seconds` | > 30 s | > 120 s | `traqora/payment/reconciliation` |
| `stripe_webhook_received_total{status="succeeded"}` vs `pay_for_booking_called_total` | drift > 5 | drift > 20 | `traqora/payment/webhook-drift` |
| `payment_intent_requires_action_total` | > 5/min | > 20/min | `traqora/payment/3ds` |

---

## 4. Contract Pending

### 4.1 State Definition

A Soroban transaction has been **submitted** but not yet **confirmed** (included in a ledger with success or explicit failure).

Applies to:
- `pay_for_booking` (escrow deposit)
- `release_payment_to_airline` / `batch_complete_bookings`
- `refund_passenger` / `oracle_refund_airline_cancel`
- `settle_cancellation`

**Booking status** reflects *intent* (e.g., `confirmed` after `pay_for_booking` submitted), but **escrow balances** only update on-chain confirmation.

Default confirmation timeout: **90 seconds** (≈ 3-4 ledgers at 5-6 s/ledger).

### 4.2 Detection

| Signal | Source | Query |
|--------|--------|-------|
| Transaction hash submitted but no `getTransaction` result after 90 s | Backend submission logs / Horizon / Soroban RPC | `SELECT * FROM tx_submissions WHERE submitted_at < NOW() - INTERVAL '90 s' AND confirmed_at IS NULL` |
| `tx_status=UNKNOWN` in indexer after 3 ledgers | Indexer logs | `tx_status_total{status="UNKNOWN"} > 0` |
| Booking shows `confirmed` but `amount_escrowed=0` on-chain | Contract read / indexer | `booking_escrow_mismatch_total > 0` |
| Alert: `contract_pending_total > 15` | Prometheus | `contract_pending_total > 15` |

### 4.3 Automated Recovery

1. **Confirm poller (runs every 10 s):**
   - For each unconfirmed submission, call `sorobanRpc.getTransaction(hash)`.
   - If `SUCCESS`: update booking status, emit corresponding event (e.g., `(booking, paid)`).
   - If `FAILED` with **retriable error** (tx fee too low, nonce mismatch, temporary network): **resubmit with fee bump** (max 2 retries).
   - If `FAILED` with **non-retriable error** (logic error, auth failure): mark booking `contract_failed`, trigger compensating action (e.g., refund if payment failed), alert on-call.
2. **Nonce gap detector:** If nonce gap detected, re-sync account sequence and resubmit.
3. **Emit `(booking, contract_confirmed)`** or `(booking, contract_failed)` with `tx_hash`, `outcome`, `retries`.

> **Idempotency:** Resubmission uses same signed XDR (or re-signs with bumped fee). Contract entry points are idempotent by booking_id + action.

### 4.4 Manual Intervention

| Condition | Action | Owner |
|--------|--------|-------|
| Transaction stuck in `UNKNOWN` > 5 min (network congestion) | 1. Check Stellar network status (status.stellar.org).
2. If congestion: bump fee to 10x base, resubmit.
3. If persistent: use `soroban-cli` to simulate and diagnose.
4. If contract logic error: deploy hotfix via proxy upgrade (see UPGRADE_PROCEDURE.md). | On-call engineer |
| `FAILED` with `HostError: BudgetExceeded` | Optimize contract / increase fee. Requires contract upgrade. | Platform engineer |
| `FAILED` with `AuthFailure` (unexpected) | 1. Verify signer keys.
2. Check if proxy upgrade changed contract ID.
3. Re-sign with current contract ID. | Security engineer |
| Multiple bookings stuck on same tx batch | Use `batch_complete_bookings` recovery path (see Partial Completion). | On-call engineer |

### 4.5 Escalation Path

```
L1 On-call (0-5 min) → L2 Platform Engineer (5-15 min) → Soroban/Stellar Core Team (via Discord/Slack) → VP Engineering (>15 min)
```

### 4.6 Monitoring & Alerting

| Metric | Warning | Critical | Dashboard |
|--------|---------|----------|-----------|
| `contract_pending_total` | > 10 | > 50 | `traqora/contract/pending` |
| `contract_confirmation_latency_seconds_p99` | > 60 s | > 180 s | `traqora/contract/latency` |
| `contract_resubmit_total` | > 5/min | > 20/min | `traqora/contract/resubmits` |
| `contract_failed_nonretriable_total` | > 0 | > 5 | `traqora/contract/failures` |
| Soroban RPC health (`/health`) | degraded | down | `traqora/infra/rpc` |

---

## 5. Partial Completion

### 5.1 State Definition

A **multi-booking operation** (batch) completes for a subset of bookings. Occurs in:
- `batch_complete_bookings` (release payment to airline for multiple bookings)
- `batch_create_flights` / `batch_update_flight_status` (airline ops)
- `batch_reserve_seats` (group booking)

**Contract behavior:** Returns `Vec<Result>` — each item succeeds or fails independently. **No atomic rollback.**

Booking statuses diverge: some `completed`, some `confirmed` (payment escrowed but not released), some `failed_batch`.

### 5.2 Detection

| Signal | Source | Query |
|--------|--------|-------|
| `batch_complete_bookings` event contains mixed `Ok`/`Err` results | Contract event / indexer | `SELECT * FROM batch_results WHERE array_contains(status, 'Err')` |
| Airline reports "partial payment received" | Airline reconciliation webhook | `airline_reconciliation_mismatch_total > 0` |
| Support ticket: "my booking not completed but others in group did" | Support tag | `tag:partial_completion` |
| Alert: `batch_partial_failure_total > 0` for 5 min | Prometheus | `increase(batch_partial_failure_total[5m]) > 0` |

### 5.3 Automated Recovery

1. **Batch result processor (runs on every batch event):**
   - Parse `Vec<Result<BookingId, Error>>`.
   - For each `Err`: determine error class:
     - `InsufficientEscrow` → booking never paid; mark `failed_batch`, notify passenger.
     - `InvalidStatus` → booking already completed/refunded; no-op.
     - `ContractPaused` / `EmergencyStop` → re-queue for retry after resume.
     - `BudgetExceeded` / transient → schedule retry with backoff (max 3).
   - For each `Ok`: verify escrow released, emit `(booking, released)` if not already emitted.
2. **Retry queue (runs every 5 min):**
   - Re-invoke `batch_complete_bookings` with **only the failed booking IDs** (idempotent).
   - Max 3 retries over 30 min.
3. **Compensation for permanent failures:**
   - If `InsufficientEscrow`: trigger `refund_passenger` (if paid) or `cancel_booking` (if not).
   - Emit `(booking, batch_recovery_complete)` with `booking_id`, `original_batch_id`, `final_status`.

> **Idempotency:** `batch_complete_bookings` can be called repeatedly with same or subset of IDs. Successful items are no-ops.

### 5.4 Manual Intervention

| Condition | Action | Owner |
|-----------|--------|-------|
| Retries exhausted, `ContractPaused` | 1. Check admin multisig for `emergency_stop`.
2. If paused: coordinate with platform lead to `emergency_resume`.
3. Re-run batch. | On-call + Platform lead |
| Airline received partial payment, demands reconciliation | 1. Query `token.balance_of(airline)` vs expected.
2. If underpaid: re-run batch for missing IDs.
3. If overpaid (duplicate release): open dispute via `dispute` contract.
4. Provide airline with transaction hashes for audit. | Payments engineer |
| Group booking: some seats confirmed, some failed | 1. Offer passenger: wait for retry, or cancel entire group (full refund).
2. If cancel: `refund_passenger` for all, `cancel_seat_reservation` for all. | Support agent |
| Pattern: same booking ID fails repeatedly in batches | Investigate booking-specific issue (e.g., corrupted storage, odd status). | Platform engineer |

### 5.5 Escalation Path

```
L1 On-call (0-5 min) → L2 Platform Engineer (5-15 min) → Airline Ops (if reconciliation) → VP Engineering (>15 min)
```

### 5.6 Monitoring & Alerting

| Metric | Warning | Critical | Dashboard |
|--------|---------|----------|-----------|
| `batch_partial_failure_total` | > 0 | > 10 | `traqora/batching/partial-failures` |
| `batch_retry_queue_depth` | > 20 | > 100 | `traqora/batching/retry-queue` |
| `batch_recovery_success_rate` | < 95% | < 80% | `traqora/batching/recovery` |
| `airline_reconciliation_mismatch_total` | > 0 | > 5 | `traqora/airline/reconciliation` |

---

## 6. Cross-Cutting Procedures

### 6.1 Incident Response Checklist (All States)

1. **Acknowledge alert** in PagerDuty / Opsgenie.
2. **Identify state** using detection queries above.
3. **Check automated recovery status** — has the job run? Check logs.
4. **If auto-recovery failed**, execute manual intervention for that state.
5. **Communicate**: Update statuspage.io, notify affected passengers via template.
6. **Document**: Add timeline to incident doc (Notion/Confluence).
7. **Post-incident**: Root cause analysis within 48 h, action items tracked.

### 6.2 Passenger Communication Templates

| State | Channel | Template Key |
|-------|---------|--------------|
| Supplier Timeout | Email + Push | `booking_supplier_delay` |
| Payment Pending | Email + SMS | `booking_payment_pending` |
| Contract Pending | Push (silent) | `booking_processing` |
| Partial Completion | Email + Call | `booking_partial_group` |

Templates live in `mobile/src/notifications/templates/`.

### 6.3 Admin CLI Commands

```bash
# Check booking status & on-chain state
soroban-cli booking get --id <booking_id> --network mainnet

# Manually trigger reconciliation
soroban-cli booking reconcile-payment --payment-intent-id <pi_xxx> --network mainnet

# Resubmit stuck transaction with fee bump
soroban-cli tx resubmit --hash <tx_hash> --fee-bump 10 --network mainnet

# Retry failed batch items
soroban-cli booking retry-batch --original-batch-id <id> --network mainnet

# Emergency stop/resume (multisig)
soroban-cli admin propose-emergency-stop --network mainnet
soroban-cli admin approve --proposal-id <id> --network mainnet
```

---

## 7. Runbook Maintenance

| Trigger | Action |
|---------|--------|
| New contract entry point added | Update relevant state detection/recovery sections. |
| New failure mode observed in production | Add detection signal, auto-recovery step, and alert. |
| SLA breach post-incident | Review and tighten thresholds / add capacity. |
| Quarterly | Full runbook review with on-call rotation. |

**Owner:** Platform Engineering Lead
**Review cadence:** Quarterly + post-incident
**Location:** `docs/operations/booking-lifecycle-recovery.md`

---

## Appendix A: State Machine Reference

```
booking lifecycle (simplified):

  create_booking
       │
       ▼
  ┌─────────┐     pay_for_booking      ┌───────────┐
  │ pending │ ───────────────────────▶ │ confirmed │
  └─────────┘                          └─────┬─────┘
       │                                      │
       │ supplier_timeout                     │ release_payment_to_airline
       ▼                                      ▼
  ┌──────────────┐                    ┌─────────────┐
  │ failed_supplier │                   │  completed  │
  └──────────────┘                    └─────────────┘
       │                                      │
       │ refund_passenger                     │
       ▼                                      ▼
  ┌─────────┐                          ┌──────────┐
  │ refunded│                          │  settled │ (cancellation split)
  └─────────┘                          └──────────┘

Transient states (this runbook):
  pending ──────▶ pending_payment ──────▶ confirmed
       ▲              │                      │
       │              ▼                      ▼
       │         contract_pending      contract_pending
       │              │                      │
       └──────────────┴──────────────────────┘
                    (recovery loops)
```

---

## Appendix B: Key Contract Entry Points

| Contract | Function | Failure State |
|----------|----------|---------------|
| `airline` | `reserve_seat`, `batch_reserve_seats`, `reconcile_and_reserve_seat_timeout_recovery` | Supplier Timeout |
| `booking` | `pay_for_booking`, `release_payment_to_airline`, `batch_complete_bookings`, `refund_passenger`, `cancel_booking`, `settle_cancellation` | Payment Pending, Contract Pending, Partial Completion |
| `token` (TRQ) | `mint`, `transfer`, `approve`, `transfer_from` | Contract Pending (allowance/balance issues) |
| `admin` (multisig) | `propose_emergency_stop`, `approve`, `execute` | All (circuit breaker) |

---

*End of runbook. For questions or updates, contact #platform-oncall in Slack.*