# Passenger Identity Validation API Documentation

This document describes the required passenger identity fields and format validation rules for the Traqora v1 booking flow.

## Overview

Before creating a payable booking, the system validates all passenger identity fields. Validation occurs in the `FlightBookingContract.reserve_seat` entrypoint and the standalone `validate_passenger_identity` function in the booking crate.

## Required Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `full_name` | String | Yes | Passenger's full legal name |
| `email` | String | Yes | Passenger's email address |
| `phone` | String | Yes | Passenger's phone number in E.164 format |
| `document_type` | String | Yes | Type of identity document |
| `document_number` | String | Yes | Document number/identifier |
| `date_of_birth` | String | Yes | Date of birth in ISO 8601 format (YYYY-MM-DD) |
| `nationality` | String | Yes | ISO 3166-1 alpha-2 or alpha-3 country code |

## Format Specifications

### full_name
- **Constraints**: Non-empty, maximum 100 characters
- **Validation error**: `InvalidName`
- **Example valid**: `"John Doe"`, `"María José García López"`
- **Example invalid**: `""` (empty), `"a".repeat(101)` (too long)

### email
- **Constraints**: Non-empty, must contain `@` and `.`, maximum 254 characters
- **Validation error**: `InvalidEmail`
- **Example valid**: `"john.doe@example.com"`, `"user+tag@sub.domain.org"`
- **Example invalid**: `"invalid-email"` (missing @ and .), `"john@doeexamplecom"` (missing .), `""` (empty)

### phone
- **Constraints**: Non-empty, 10-15 characters, must start with `+` (E.164 format)
- **Validation error**: `InvalidPhone`
- **Example valid**: `"+15551234567"`, `"+442071234567"`, `"+819012345678"`
- **Example invalid**: `"15551234567"` (missing +), `"+123"` (too short), `"+1234567890123456"` (too long), `""` (empty)

### document_type
- **Constraints**: Must be one of the known types
- **Allowed values**: `"passport"`, `"national_id"`, `"driver_license"`
- **Validation error**: `InvalidDocumentNumber` (reused for invalid document type)
- **Example valid**: `"passport"`, `"national_id"`, `"driver_license"`
- **Example invalid**: `"invalid_type"`, `""` (empty)

### document_number
- **Constraints**: Non-empty, maximum 50 characters, alphanumeric only
- **Validation error**: `InvalidDocumentNumber`
- **Example valid**: `"A12345678"`, `"ID123456789"`, `"D1234567"`
- **Example invalid**: `""` (empty), `"a".repeat(51)` (too long), `"AB-123"` (contains non-alphanumeric)

### date_of_birth
- **Constraints**: Exactly 10 characters, ISO 8601 format `YYYY-MM-DD`, all parts numeric
- **Validation error**: `InvalidDateOfBirth`
- **Example valid**: `"1990-01-15"`, `"2005-12-31"`, `"1950-07-04"`
- **Example invalid**: `"15-01-1990"` (wrong format), `"1990-1-15"` (wrong length), `"1990-ab-cd"` (non-numeric), `"1990/01/15"` (wrong separator)

### nationality
- **Constraints**: 2 or 3 characters, uppercase ASCII letters only (ISO 3166-1 alpha-2 or alpha-3)
- **Validation error**: `InvalidNationality`
- **Example valid**: `"USA"`, `"GBR"`, `"DE"`, `"GB"`, `"FR"`, `"JPN"`
- **Example invalid**: `"usa"` (lowercase), `"U"` (too short), `"USAA"` (too long), `"US1"` (contains digit)

## Validation Error Codes

| Error Code | Description |
|------------|-------------|
| `MissingField` | A required field is empty (currently used for empty document_type) |
| `InvalidEmail` | Email format validation failed |
| `InvalidPhone` | Phone format validation failed |
| `InvalidDocumentNumber` | Document type unknown, document number empty/too long/non-alphanumeric |
| `InvalidDateOfBirth` | Date of birth format validation failed |
| `InvalidNationality` | Nationality format validation failed |
| `InvalidName` | Full name empty or too long |

## Integration with Checkout Flow

### Booking Creation with Identity Validation

```rust
// FlightBookingContract.reserve_seat
// Validates passenger_identity before creating booking and escrowing payment
let booking_id = client.reserve_seat(
    &passenger,
    &flight_id,
    &seat,
    &amount,
    &passenger_identity,
);
```

### Validation Behavior

1. **Validation occurs before any state changes** - No booking is created if validation fails
2. **Fail-fast** - First validation error encountered stops processing
3. **Panic with error code** - Contract panics with the specific `ValidationError` variant
4. **No partial state** - Seat reservation, payment escrow, and booking creation only proceed after all validations pass

### Example Valid Payload

```json
{
  "full_name": "John Doe",
  "email": "john.doe@example.com",
  "phone": "+15551234567",
  "document_type": "passport",
  "document_number": "A12345678",
  "date_of_birth": "1990-01-15",
  "nationality": "USA"
}
```

### Example Invalid Payloads and Expected Errors

| Payload | Expected Error |
|---------|----------------|
| `{"full_name": "", ...}` | `InvalidName` |
| `{"full_name": "a".repeat(101), ...}` | `InvalidName` |
| `{"email": "invalid-email", ...}` | `InvalidEmail` |
| `{"email": "john@doeexamplecom", ...}` | `InvalidEmail` |
| `{"phone": "15551234567", ...}` | `InvalidPhone` |
| `{"phone": "+123", ...}` | `InvalidPhone` |
| `{"document_type": "invalid_type", ...}` | `InvalidDocumentNumber` |
| `{"document_number": "", ...}` | `InvalidDocumentNumber` |
| `{"date_of_birth": "15-01-1990", ...}` | `InvalidDateOfBirth` |
| `{"date_of_birth": "1990-1-15", ...}` | `InvalidDateOfBirth` |
| `{"nationality": "usa", ...}` | `InvalidNationality` |
| `{"nationality": "USAA", ...}` | `InvalidNationality` |

## Testing

Unit tests for validation logic are located in:
- `contracts/packages/booking/src/identity_test.rs` - Standalone validation function tests
- `contracts/packages/integration-tests/tests/flight_booking_test.rs` - Integration tests with `reserve_seat`

Run tests with:
```bash
cd contracts && cargo test --package booking identity_test
cd contracts && cargo test --package integration-tests flight_booking_test
```

## Security and Privacy Considerations

- All validation occurs on-chain in the Soroban contract
- No PII is logged in events; only booking references and amounts are emitted
- Document numbers are stored in persistent storage but not exposed in events
- Nationality codes follow ISO standards to minimize free-text input risks
- Phone numbers validated to E.164 format for international compatibility

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025 | Initial v1 validation specification |
