# Booking API & Lifecycle

## Overview
The Booking API manages flight bookings, escrow payments, cancellations, and supplier confirmation references with strict access control and redacted operational logging.

## Supplier Booking References
Suppliers and authorized participants can attach and query external confirmation identifiers (PNR/supplier booking references) securely:

- `set_supplier_booking_reference(env, booking_id, caller, supplier_ref)`: Records or updates the supplier booking reference for a given booking. Requires authorization from either the passenger or the airline.
- `get_redacted_supplier_reference(env, booking_id, viewer)`: Returns a redacted operational representation (`****`) of the supplier booking reference to authorized viewers (passenger or airline), preserving privacy in operational logs.