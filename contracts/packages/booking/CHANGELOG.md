# Changelog

All notable changes to the Booking contract will be documented in this file.

## [Unreleased]

### Added
- **Supplier Booking References**: Airlines can now set supplier confirmation identifiers (e.g., PNR codes) on confirmed or completed bookings via `set_supplier_reference()`.
- **Access-Controlled Retrieval**: Passengers and airlines can retrieve the supplier reference via `get_supplier_reference()`. Unauthorized callers are rejected.
- **Redacted Operational Logs**: Events emitted when setting a supplier reference (`supplier_ref_set` action) intentionally omit the reference value for privacy in operational logs.

### Security
- Only the airline that owns the booking can set a supplier reference.
- Only the booking's passenger or airline can retrieve the supplier reference.
- Supplier references can only be set on bookings in `confirmed` or `completed` status (not `pending`, `cancelled`, or `refunded`).

### Testing
- Added comprehensive test coverage in `supplier_reference_test.rs` covering:
  - Successful setting and retrieval by airline/passenger
  - Authorization enforcement (unauthorized set/get rejected)
  - Status validation (only confirmed/completed bookings)
  - Event redaction verification (reference not logged)
  - Unset reference returns `None`