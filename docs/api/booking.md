# Booking API — Passenger Identity Validation

## Overview

The `create_booking_with_identity` endpoint validates all required passenger identity fields **before** creating a payable booking. This ensures that only bookings with complete, well-formed passenger data enter the payment flow.

Validation occurs **synchronously** at booking creation time. If any field fails validation, the booking is **not created** and the corresponding error code is returned immediately.

---

## Endpoint

```
create_booking_with_identity(
  passenger: Address,
  airline: Address,
  flight_number: Symbol,
  from_airport: Symbol,
  to_airport: Symbol,
  departure_time: u64,        // Unix timestamp
  price: i128,                // Amount in token base units
  token: Address,             // Token contract address
  passenger_identity: PassengerIdentity
) -> Result<u64, ValidationError>
```

Returns the new `booking_id` on success, or a `ValidationError` variant on failure.

---

## Required Fields — `PassengerIdentity`

| Field | Type | Constraints | Description |
|-------|------|-------------|-------------|
| `full_name` | String | 1–100 characters, non-empty | Passenger's full legal name as it appears on the travel document. |
| `date_of_birth` | u64 | Unix timestamp; must be **≥ 2 years ago** and **< current ledger time** | Date of birth. Used to enforce minimum age of 2 years. |
| `nationality` | String | ISO 3166‑1 alpha‑3 code (3 uppercase letters), non-empty | Passenger's nationality / citizenship. Example: `USA`, `GBR`, `FRA`. |
| `document_type` | String | Enum: `passport` \| `national_id` (case‑insensitive) | Type of government‑issued identity document. |
| `document_number` | String | Format depends on `document_type` (see below), non-empty | Document identifier. |
| `email` | String | RFC 5322 compliant, non-empty | Contact email for booking confirmations and updates. |
| `phone` | String | E.164 format: `+` followed by 1–15 digits, non-empty | Contact phone number in international format. |

### Document Number Format Rules

| `document_type` | Allowed Characters | Length |
|-----------------|-------------------|--------|
| `passport` | Alphanumeric (A–Z, a–z, 0–9) | 6–9 characters |
| `national_id` | Digits only (0–9) | 8–12 digits |

> **Note:** The validation is intentionally permissive to accommodate global variation. Adjust length/character rules in the contract if stricter per‑country rules are required.

---

## Validation Error Codes

The `ValidationError` enum is returned directly from `create_booking_with_identity`. Each variant maps to a specific failure:

| Code | Variant | HTTP Equivalent | When Returned |
|------|---------|-----------------|---------------|
| 1 | `MissingField` | 400 Bad Request | Any required field is empty or missing. |
| 2 | `InvalidEmail` | 400 Bad Request | `email` does not match RFC 5322 basic structure (exactly one `@`, non‑empty local part, domain with at least one dot not at start/end). |
| 3 | `InvalidPhone` | 400 Bad Request | `phone` is not in E.164 format (`+` + 1–15 digits). |
| 4 | `InvalidDateOfBirth` | 400 Bad Request | `date_of_birth` is in the future or equals current ledger time. |
| 5 | `UnderAge` | 400 Bad Request | Passenger is younger than 2 years (age < 2 × 365 days). |
| 6 | `InvalidDocumentNumber` | 400 Bad Request | `document_number` does not match the format required by `document_type`, **or** `document_type` is not `passport` or `national_id`. |
| 7 | `InvalidDocumentType` | 400 Bad Request | *Reserved — currently not emitted separately; invalid document types fall through to `InvalidDocumentNumber`.* |

> **Implementation note:** The contract returns the first failing check in the order above. Only one error is returned per call.

---

## Example — Valid Request

```json
{
  "passenger": "G... (Address)