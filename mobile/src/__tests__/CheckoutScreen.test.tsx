import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import CheckoutScreen from '../screens/CheckoutScreen';

describe('CheckoutScreen Passenger Identity Validation', () => {
  it('renders passenger input fields correctly', () => {
    render(
      <CheckoutScreen
        flightId="FL100"
        price="$250.00"
        onCompleteBooking={() => {}}
      />
    );

    expect(screen.getByLabelText(/Full Name/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Passport Number/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Nationality/i)).toBeInTheDocument();
  });

  it('displays validation errors when submitting empty fields', () => {
    render(
      <CheckoutScreen
        flightId="FL100"
        price="$250.00"
        onCompleteBooking={() => {}}
      />
    );

    const submitButton = screen.getByRole('button', { name: /Proceed to Payment/i });
    fireEvent.click(submitButton);

    expect(screen.getByText(/Full name is required/i)).toBeInTheDocument();
    expect(screen.getByText(/Passport number is required/i)).toBeInTheDocument();
    expect(screen.getByText(/Nationality is required/i)).toBeInTheDocument();
  });

  it('succeeds and calls onCompleteBooking when valid identity is provided', () => {
    const handleComplete = jest.fn();
    render(
      <CheckoutScreen
        flightId="FL100"
        price="$250.00"
        onCompleteBooking={handleComplete}
      />
    );

    fireEvent.change(screen.getByLabelText(/Full Name/i), { target: { value: 'Alice Smith' } });
    fireEvent.change(screen.getByLabelText(/Passport Number/i), { target: { value: 'AB123456' } });
    fireEvent.change(screen.getByLabelText(/Nationality/i), { target: { value: 'USA' } });

    const submitButton = screen.getByRole('button', { name: /Proceed to Payment/i });
    fireEvent.click(submitButton);

    expect(handleComplete).toHaveBeenCalledTimes(1);
    expect(handleComplete).toHaveBeenCalledWith({
      fullName: 'Alice Smith',
      passportNumber: 'AB123456',
      nationality: 'USA',
    });
  });
});
