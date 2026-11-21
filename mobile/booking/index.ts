export interface PassengerIdentityInput {
  fullName: string;
  passportNumber: string;
  nationality: string;
}

export interface ValidationError {
  field: keyof PassengerIdentityInput;
  message: string;
}

export function validatePassengerIdentity(input: PassengerIdentityInput): ValidationError[] {
  const errors: ValidationError[] = [];

  const trimmedName = input.fullName.trim();
  if (!trimmedName) {
    errors.push({ field: 'fullName', message: 'Full name is required.' });
  } else if (trimmedName.length < 2) {
    errors.push({ field: 'fullName', message: 'Full name must be at least 2 characters.' });
  }

  const trimmedPassport = input.passportNumber.trim();
  if (!trimmedPassport) {
    errors.push({ field: 'passportNumber', message: 'Passport number is required.' });
  } else if (!/^[A-Z0-9]{6,12}$/i.test(trimmedPassport)) {
    errors.push({ field: 'passportNumber', message: 'Invalid passport number format (6-12 alphanumeric characters).' });
  }

  const trimmedNationality = input.nationality.trim();
  if (!trimmedNationality) {
    errors.push({ field: 'nationality', message: 'Nationality is required.' });
  } else if (trimmedNationality.length < 2) {
    errors.push({ field: 'nationality', message: 'Nationality must be a valid country code or name.' });
  }

  return errors;
}
