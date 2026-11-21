# Booking API — Idempotent Supplier Booking

This document describes the idempotent supplier booking endpoints added to prevent duplicate bookings when retrying ambiguous requests.

---

## Overview

The Flight Booking contract now supports **idempotent supplier booking submission** using a composite key of `supplier_id` + `idempotency_key`. This allows suppliers to safely retry booking requests without creating duplicate reservations.

### Key Concepts

| Term | Description |
|------|-------------|
| `supplier_id` | Unique identifier for the airline/supplier (e.g., "airline_001") |
| `idempotency_key` | Client-generated unique key per booking attempt (e.g., UUID, reference number) |
| **Composite Key** | `(supplier_id, idempotency_key)` — uniquely identifies a booking request |

---

## Endpoints

### 1. Submit Booking (Idempotent)

Creates a new booking or returns an existing booking if the same `(supplier_id, idempotency_key)` was already used.

```rust
pub fn submit_booking(
    env: Env,
    supplier_id: String,
    idempotency_key: String,
    passenger: Address,
    flight_id: Symbol,
    seat: Symbol,
    amount: i128,
) -> u64
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `supplier_id` | String | Yes | Supplier's unique identifier |
| `idempotency_key` | String | Yes | Unique key for this booking attempt |
| `passenger` | Address | Yes | Passenger's Stellar address |
| `flight_id` | Symbol | Yes | Flight identifier (e.g., "FL100") |
| `seat` | Symbol | Yes | Seat identifier (e.g., "12A") |
| `amount` | i128 | Yes | Booking amount in base units (must be > 0) |

#### Returns

| Return | Type | Description |
|--------|------|-------------|
| `booking_id` | u64 | The booking ID (new or existing) |

#### Behavior

1. **First submission** with a given `(supplier_id, idempotency_key)`:
   - Validates seat availability
   - Creates new booking with `BookingState::Confirmed`
   - Stores mapping `(supplier_id, idempotency_key) → booking_id`
   - Emits `SupplierBookingSubmittedEvent` with `is_duplicate: false`
   - Returns new `booking_id`

2. **Duplicate submission** with same `(supplier_id, idempotency_key)`:
   - Returns existing `booking_id` immediately
   - Does **not** validate seat again
   - Does **not** create new booking
   - Emits `SupplierBookingSubmittedEvent` with `is_duplicate: true`

#### Events Emitted

```rust
SupplierBookingSubmittedEvent {
    supplier_id: String,
    idempotency_key: String,
    booking_id: u64,
    is_duplicate: bool,  // true = duplicate request, false = new booking
}
```

---

### 2. Reconcile Supplier Booking

Retrieves the booking ID for a given `(supplier_id, idempotency_key)` without creating a new booking. Useful for checking status after a timeout or network error.

```rust
pub fn reconcile_supplier_booking(
    env: Env,
    supplier_id: String,
    idempotency_key: String,
) -> Option<u64>
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `supplier_id` | String | Yes | Supplier's unique identifier |
| `idempotency_key` | String | Yes | Idempotency key used in original submission |

#### Returns

| Return | Type | Description |
|--------|------|-------------|
| `booking_id` | Option<u64> | `Some(booking_id)` if found, `None` if unknown |

#### Behavior

- Pure read operation — no state changes
- Returns the booking ID if the composite key exists in storage
- Returns `None` if the key was never used or was used by a different supplier

---

## Example: Idempotent Retry Flow

### Scenario
Supplier submits a booking request. Network times out before response. Supplier retries with same idempotency key.

### Request 1 (Initial Submission)

```json
{
  "supplier_id": "airline_001",
  "idempotency_key": "req_abc123def456",
  "passenger": "GAAAA...",
  "flight_id": "FL100",
  "seat": "12A",
  "amount": 500000000
}
```

### Response 1 (Success — New Booking)

```json
{
  "booking_id": 42,
  "event": {
    "type": "SupplierBookingSubmitted",
    "supplier_id": "airline_001",
    "idempotency_key": "req_abc123def456",
    "booking_id": 42,
    "is_duplicate": false
  }
}
```

### Request 2 (Retry After Timeout — Same Key)

```json
{
  "supplier_id": "airline_001",
  "idempotency_key": "req_abc123def456",
  "passenger": "GAAAA...",
  "flight_id": "FL100",
  "seat": "12A",
  "amount": 500000000
}
```

### Response 2 (Success — Duplicate Detected)

```json
{
  "booking_id": 42,
  "event": {
    "type": "SupplierBookingSubmitted",
    "supplier_id": "airline_001",
    "idempotency_key": "req_abc123def456",
    "booking_id": 42,
    "is_duplicate": true
  }
}
```

### Key Observations

| Aspect | Behavior |
|--------|----------|
| Same `booking_id` returned | ✅ Yes (42) |
| New booking created | ❌ No |
| Seat re-validated | ❌ No (skipped for duplicate) |
| Event `is_duplicate` | `false` → `true` |
| Safe to retry indefinitely | ✅ Yes |

---

## Example: Reconciliation After Ambiguous Failure

### Scenario
Supplier submitted a booking but never received a response (network partition). Supplier wants to know if booking was created.

### Reconcile Request

```json
{
  "supplier_id": "airline_001",
  "idempotency_key": "req_abc123def456"
}
```

### Reconcile Response (Booking Exists)

```json
{
  "booking_id": 42
}
```

### Reconcile Response (Booking Not Found)

```json
{
  "booking_id": null
}
```

---

## Error Handling

| Error Condition | Response |
|-----------------|----------|
| `amount <= 0` | Panic: `"Invalid amount"` |
| Seat already reserved (first submission) | Panic: `"Seat already reserved"` |
| Unknown composite key (reconcile) | Returns `None` |

---

## Best Practices for Suppliers

1. **Generate unique idempotency keys** per booking attempt (UUID v4 recommended)
2. **Use the same key for retries** — do not generate a new key on retry
3. **Call `reconcile_supplier_booking`** after a timeout before retrying `submit_booking`
4. **Log both `booking_id` and `is_duplicate`** from events for audit trail
5. **Use distinct `supplier_id` per integration** — keys are scoped per supplier

---

## Idempotency Key Guidelines

| Guideline | Recommendation |
|-----------|----------------|
| Format | UUID v4, or `<supplier_prefix>_<timestamp>_<random>` |
| Uniqueness | Must be unique per booking attempt per supplier |
| Length | Max 256 characters (String) |
| Storage | Keys persisted indefinitely with booking record |

---

## Related Endpoints

| Function | Purpose |
|----------|---------|
| `get_booking(booking_id)` | Retrieve full booking details by ID |
| `reserve_seat(passenger, flight_id, seat, amount)` | Direct seat reservation (non-supplier, requires auth) |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v1.1.0 | 2025 | Added `submit_booking` with idempotency, `reconcile_supplier_booking` |
| v1.0.0 | 2025 | Initial `reserve_seat` endpoint |