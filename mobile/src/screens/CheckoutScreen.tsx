import React, { useState } from 'react';
import { validatePassengerIdentity, PassengerIdentityInput, ValidationError } from '../../booking';

interface CheckoutScreenProps {
  flightId: string;
  price: string;
  onCompleteBooking: (identity: PassengerIdentityInput) => void;
}

export default function CheckoutScreen({ flightId, price, onCompleteBooking }: CheckoutScreenProps) {
  const [fullName, setFullName] = useState('');
  const [passportNumber, setPassportNumber] = useState('');
  const [nationality, setNationality] = useState('');
  const [errors, setErrors] = useState<ValidationError[]>([]);
  const [touched, setTouched] = useState(false);

  const handleValidateAndSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setTouched(true);
    const identityInput: PassengerIdentityInput = {
      fullName,
      passportNumber,
      nationality,
    };
    const validationErrors = validatePassengerIdentity(identityInput);
    setErrors(validationErrors);

    if (validationErrors.length === 0) {
      onCompleteBooking(identityInput);
    }
  };

  const getFieldError = (field: keyof PassengerIdentityInput) => {
    const err = errors.find((e) => e.field === field);
    return err ? err.message : null;
  };

  return (
    <div className="checkout-screen">
      <h1>Passenger Details & Checkout</h1>
      <div className="flight-summary">
        <p>Flight: {flightId}</p>
        <p>Total: {price}</p>
      </div>
      <form onSubmit={handleValidateAndSubmit} className="passenger-form">
        <div className="form-group">
          <label htmlFor="fullName">Full Name (as in passport)</label>
          <input
            id="fullName"
            type="text"
            value={fullName}
            onChange={(e) => setFullName(e.target.value)}
            placeholder="Jane Doe"
          />
          {touched && getFieldError('fullName') && (
            <span className="error-message" role="alert">{getFieldError('fullName')}</span>
          )}
        </div>

        <div className="form-group">
          <label htmlFor="passportNumber">Passport Number</label>
          <input
            id="passportNumber"
            type="text"
            value={passportNumber}
            onChange={(e) => setPassportNumber(e.target.value)}
            placeholder="AB123456"
          />
          {touched && getFieldError('passportNumber') && (
            <span className="error-message" role="alert">{getFieldError('passportNumber')}</span>
          )}
        </div>

        <div className="form-group">
          <label htmlFor="nationality">Nationality</label>
          <input
            id="nationality"
            type="text"
            value={nationality}
            onChange={(e) => setNationality(e.target.value)}
            placeholder="USA"
          />
          {touched && getFieldError('nationality') && (
            <span className="error-message" role="alert">{getFieldError('nationality')}</span>
          )}
        </div>

        <button type="submit" className="confirm-booking-btn">
          Proceed to Payment
        </button>
      </form>
    </div>
  );
}
