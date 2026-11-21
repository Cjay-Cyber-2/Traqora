# Booking Lifecycle Failure Recovery (v1)

This document defines the recovery procedures and protocol invariants for booking lifecycle failures within the v1 Traqora scope.

## States and Failure Modes

1. **Payment Pending / Supplier Timeout**
   - **Description:** A booking is created in `pending` state, but payment or airline seat reservation times out before escrow is funded or confirmed.
   - **Recovery / Resolution:** Unfunded or expired pending bookings can be cancelled or automatically refunded/timed-out via client or operator triggers using `cancel_booking` or `refund_passenger`. No funds are locked if escrow was never funded.

2. **Contract Pending**
   - **Description:** Smart contract interactions or batch operations (e.g., batch flight creation or batch completion) encounter partial failures.
   - **Recovery / Resolution:** Soroban atomicity ensures that transactions either fully succeed or roll back state changes. For batch operations, valid items succeed while failed items emit explicit errors or can be retried independently as defined in contract error handling tests.

3. **Partial Completion**
   - **Description:** A multi-leg journey or batch booking operation is partially fulfilled (e.g., one leg succeeds while another fails or encounters capacity constraints).
   - **Recovery / Resolution:** The protocol disallows partial multi-leg reservations that would leave inconsistent ledger state (see `test_reserve_journey_does_not_partially_decrement_when_a_leg_is_full`). Operators or passengers can reconcile remaining legs or trigger refunds for failed components.
