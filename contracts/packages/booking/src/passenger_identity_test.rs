#![no_std]
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use crate::{PassengerIdentity, ValidationError};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_env() -> Env {
        Env::default()
    }

    fn valid_passenger_identity(env: &Env) -> PassengerIdentity {
        PassengerIdentity {
            full_name: String::from_str(env, "John Doe"),
            date_of_birth: 631152000, // 1990-01-01
            nationality: String::from_str(env, "US"),
            document_type: String::from_str(env, "passport"),
            document_number: String::from_str(env, "A1234567"),
            email: String::from_str(env, "john.doe@example.com"),
            phone: String::from_str(env, "+15551234567"),
        }
    }

    #[test]
    fn test_valid_passport_identity() {
        let env = create_env();
        let identity = valid_passenger_identity(&env);
        let result = identity.validate(&env);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_valid_national_id_identity() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "national_id");
        identity.document_number = String::from_str(&env, "12345678");
        let result = identity.validate(&env);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_missing_full_name() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.full_name = String::from_str(&env, "");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::MissingField));
    }

    #[test]
    fn test_missing_nationality() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.nationality = String::from_str(&env, "");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::MissingField));
    }

    #[test]
    fn test_missing_document_type() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::MissingField));
    }

    #[test]
    fn test_missing_document_number() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_number = String::from_str(&env, "");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::MissingField));
    }

    #[test]
    fn test_missing_email() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::MissingField));
    }

    #[test]
    fn test_missing_phone() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.phone = String::from_str(&env, "");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::MissingField));
    }

    #[test]
    fn test_invalid_email_no_at() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "johndoe.example.com");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_invalid_email_multiple_at() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "john@doe@example.com");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_invalid_email_no_domain() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "john@");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_invalid_email_no_local() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "@example.com");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_invalid_email_no_dot_in_domain() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "john@example");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_invalid_email_dot_at_start_of_domain() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "john@.example.com");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_invalid_email_dot_at_end_of_domain() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.email = String::from_str(&env, "john@example.com.");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn test_valid_emails() {
        let env = create_env();
        let valid_emails = vec![
            "john.doe@example.com",
            "john+tag@example.co.uk",
            "john123@test-domain.org",
            "a@b.co",
        ];
        for email_str in valid_emails {
            let mut identity = valid_passenger_identity(&env);
            identity.email = String::from_str(&env, email_str);
            let result = identity.validate(&env);
            assert_eq!(result, Ok(()), "Email '{}' should be valid", email_str);
        }
    }

    #[test]
    fn test_invalid_phone_no_plus() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.phone = String::from_str(&env, "15551234567");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidPhone));
    }

    #[test]
    fn test_invalid_phone_too_short() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.phone = String::from_str(&env, "+");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidPhone));
    }

    #[test]
    fn test_invalid_phone_too_long() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.phone = String::from_str(&env, "+1234567890123456"); // 16 digits
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidPhone));
    }

    #[test]
    fn test_invalid_phone_non_digit() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.phone = String::from_str(&env, "+1555123456a");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidPhone));
    }

    #[test]
    fn test_valid_phones() {
        let env = create_env();
        let valid_phones = vec![
            "+15551234567",
            "+442071234567",
            "+81312345678",
            "+1", // minimum 1 digit
            "+123456789012345", // maximum 15 digits
        ];
        for phone_str in valid_phones {
            let mut identity = valid_passenger_identity(&env);
            identity.phone = String::from_str(&env, phone_str);
            let result = identity.validate(&env);
            assert_eq!(result, Ok(()), "Phone '{}' should be valid", phone_str);
        }
    }

    #[test]
    fn test_date_of_birth_in_future() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        // Set date of birth to year 3000
        identity.date_of_birth = 32503680000;
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDateOfBirth));
    }

    #[test]
    fn test_date_of_birth_now() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.date_of_birth = env.ledger().timestamp();
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDateOfBirth));
    }

    #[test]
    fn test_under_age() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        // Born 1 year ago
        identity.date_of_birth = env.ledger().timestamp() - 365 * 24 * 60 * 60;
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::UnderAge));
    }

    #[test]
    fn test_exactly_two_years_old() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        // Born exactly 2 years ago
        identity.date_of_birth = env.ledger().timestamp() - 2 * 365 * 24 * 60 * 60;
        let result = identity.validate(&env);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_invalid_passport_too_short() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "passport");
        identity.document_number = String::from_str(&env, "A1234"); // 5 chars
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_invalid_passport_too_long() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "passport");
        identity.document_number = String::from_str(&env, "A123456789"); // 10 chars
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_invalid_passport_non_alphanumeric() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "passport");
        identity.document_number = String::from_str(&env, "A123-567");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_valid_passport_boundaries() {
        let env = create_env();
        let valid_numbers = vec!["A12345", "A12345678", "ABCDEF12", "123456789"];
        for num in valid_numbers {
            let mut identity = valid_passenger_identity(&env);
            identity.document_type = String::from_str(&env, "passport");
            identity.document_number = String::from_str(&env, num);
            let result = identity.validate(&env);
            assert_eq!(result, Ok(()), "Passport '{}' should be valid", num);
        }
    }

    #[test]
    fn test_invalid_national_id_too_short() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "national_id");
        identity.document_number = String::from_str(&env, "1234567"); // 7 digits
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_invalid_national_id_too_long() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "national_id");
        identity.document_number = String::from_str(&env, "1234567890123"); // 13 digits
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_invalid_national_id_non_digit() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "national_id");
        identity.document_number = String::from_str(&env, "1234567A");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_valid_national_id_boundaries() {
        let env = create_env();
        let valid_numbers = vec!["12345678", "123456789012"];
        for num in valid_numbers {
            let mut identity = valid_passenger_identity(&env);
            identity.document_type = String::from_str(&env, "national_id");
            identity.document_number = String::from_str(&env, num);
            let result = identity.validate(&env);
            assert_eq!(result, Ok(()), "National ID '{}' should be valid", num);
        }
    }

    #[test]
    fn test_invalid_document_type() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "drivers_license");
        let result = identity.validate(&env);
        assert_eq!(result, Err(ValidationError::InvalidDocumentNumber));
    }

    #[test]
    fn test_case_insensitive_document_type() {
        let env = create_env();
        let mut identity = valid_passenger_identity(&env);
        identity.document_type = String::from_str(&env, "PASSPORT");
        let result = identity.validate(&env);
        assert_eq!(result, Ok(()));

        identity.document_type = String::from_str(&env, "National_ID");
        identity.document_number = String::from_str(&env, "12345678");
        let result = identity.validate(&env);
        assert_eq!(result, Ok(()));
    }
}
