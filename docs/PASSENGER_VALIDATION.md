# Passenger Identity and Booking Validation

Before creating a payable booking in V1, passenger identity fields must be validated:
- `fullName`: Required, non-empty string.
- `email`: Required, must contain '@'.
- `passportNumber`: Required, minimum length of 6 characters.
