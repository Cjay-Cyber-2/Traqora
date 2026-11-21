import React from 'react';
import { render, screen } from '@testing-library/react';
import BookingConfirmationScreen, { validatePassenger } from '../BookingConfirmationScreen';

describe('BookingConfirmationScreen', () => {
  const sampleItinerary = [
    {
      flightNumber: 'TRQ101',
      fromAirport: 'JFK',
      toAirport: 'LAX',
      departureTime: '2025-06-01 10:00',
    },
  ];

  it('renders booking confirmation details correctly', () => {
    render(
      <BookingConfirmationScreen
        bookingId="12345"
        itinerary={sampleItinerary}
        paymentStatus="confirmed"
        totalAmount="$450.00"
        transactionHash="abc123def456"
      />
    );

    expect(screen.getByText('Booking Confirmed!')).toBeInTheDocument();
    expect(screen.getByText('Reference: TRAQ-12345')).toBeInTheDocument();
    expect(screen.getByText('confirmed')).toBeInTheDocument();
    expect(screen.getByText('$450.00')).toBeInTheDocument();
    expect(screen.getByText('Flight TRQ101')).toBeInTheDocument();
    expect(screen.getByText('JFK → LAX')).toBeInTheDocument();
    expect(screen.getByText('abc123def456')).toBeInTheDocument();
  });
});

describe('validatePassenger', () => {
  it('returns true for valid passenger details', () => {
    const valid = {
      fullName: 'Jane Doe',
      email: 'jane@example.com',
      passportNumber: 'AB123456',
    };
    expect(validatePassenger(valid)).toBe(true);
  });

  it('returns false when fullName is missing or empty', () => {
    expect(validatePassenger({ fullName: '', email: 'jane@example.com', passportNumber: 'AB123456' })).toBe(false);
    expect(validatePassenger({ email: 'jane@example.com', passportNumber: 'AB123456' })).toBe(false);
  });

  it('returns false when email lacks @ symbol', () => {
    expect(validatePassenger({ fullName: 'Jane Doe', email: 'janegmail.com', passportNumber: 'AB123456' })).toBe(false);
  });

  it('returns false when passportNumber is too short', () => {
    expect(validatePassenger({ fullName: 'Jane Doe', email: 'jane@example.com', passportNumber: '12345' })).toBe(false);
  });
});
