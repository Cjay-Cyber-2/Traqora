# Booking API Documentation

## Overview
The Booking contract handles travel booking lifecycles including creation, payment escrow, release of funds to airlines, passenger refunds, cancellation settlement, batch processing, and recording secure supplier confirmation references (PNR).

## Methods

### `create_booking`
Initializes a new booking in `pending` status.
- **Parameters:** `passenger`, `airline`, `flight_number`, `from_airport`, `to_airport`, `departure_time`, `price`, `token`
- **Returns:** `u64` (booking ID)

### `pay_for_booking`
Transfers payment from passenger to contract escrow and marks the booking as `confirmed`.
- **Parameters:** `booking_id`

### `release_payment_to_airline`
Releases escrowed funds to the airline post-flight.
- **Parameters:** `booking_id`

### `refund_passenger`
Refunds escrowed funds to the passenger prior to the cancellation window.
- **Parameters:** `booking_id`

### `set_supplier_booking_reference`
Records or updates the supplier booking reference (PNR) with strict access control (airline or passenger only) and redacted operational logging for privacy compliance.
- **Parameters:** `caller`, `booking_id`, `supplier_ref` (`String`)

### `get_booking`
Retrieves booking details including status, escrow amount, and supplier booking reference.
- **Parameters:** `booking_id`
- **Returns:** `Option<Booking>`
