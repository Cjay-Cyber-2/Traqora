# Booking Idempotency

To prevent duplicate supplier booking submissions during retries or network timeouts, the booking service supports idempotent requests via a `supplierReference`. When provided, subsequent requests with the same `supplierReference` return the existing booking record.
