#![no_std]
use soroban_sdk::{testutils::Address as _, Address, Env, String};
use crate::{PassengerIdentity, ValidationError};

// Validation functions for passenger identity
pub fn validate_passenger_identity(identity: &PassengerIdentity) -> Result<(), ValidationError> {
    // Validate full_name - not empty, reasonable length
    if identity.full_name.is_empty() || identity.full_name.len() > 100 {
        return Err(ValidationError::InvalidName);
    }
    
    // Validate email - basic format check
    if identity.email.is_empty() || !identity.email.contains('@') || !identity.email.contains('.') {
        return Err(ValidationError::InvalidEmail);
    }
    
    // Validate phone - basic format check (E.164 format: + followed by digits)
    if identity.phone.is_empty() || identity.phone.len() < 10 || identity.phone.len() > 15 {
        return Err(ValidationError::InvalidPhone);
    }
    if !identity.phone.chars().next().map_or(false, |c| c == '+') {
        return Err(ValidationError::InvalidPhone);
    }
    
    // Validate document_type - must be one of known types
    let doc_type = identity.document_type.as_str();
    if doc_type != "passport" && doc_type != "national_id" && doc_type != "driver_license" {
        return Err(ValidationError::InvalidDocumentNumber);
    }
    
    // Validate document_number - not empty, reasonable length
    if identity.document_number.is_empty() || identity.document_number.len() > 50 {
        return Err(ValidationError::InvalidDocumentNumber);
    }
    
    // Validate date_of_birth - YYYY-MM-DD format
    if identity.date_of_birth.len() != 10 {
        return Err(ValidationError::InvalidDateOfBirth);
    }
    let dob = identity.date_of_birth.as_str();
    if &dob[4..5] != "-" || &dob[7..8] != "-" {
        return Err(ValidationError::InvalidDateOfBirth);
    }
    // Basic check that parts are numeric
    let year = &dob[0..4];
    let month = &dob[5..7];
    let day = &dob[8..10];
    if !year.chars().all(|c| c.is_ascii_digit()) 
        || !month.chars().all(|c| c.is_ascii_digit()) 
        || !day.chars().all(|c| c.is_ascii_digit()) {
        return Err(ValidationError::InvalidDateOfBirth);
    }
    
    // Validate nationality - ISO 3166-1 alpha-2 or alpha-3 code
    let nat_len = identity.nationality.len();
    if nat_len != 2 && nat_len != 3 {
        return Err(ValidationError::InvalidNationality);
    }
    if !identity.nationality.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError::InvalidNationality);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_identity(env: &Env) -> PassengerIdentity {
        PassengerIdentity {
            full_name: String::from_str(env, "John Doe"),
            email: String::from_str(env, "john.doe@example.com"),
            phone: String::from_str(env, "+15551234567"),
            document_type: String::from_str(env, "passport"),
            document_number: String::from_str(env, "A12345678"),
            date_of_birth: String::from_str(env, "1990-01-15"),
            nationality: String::from_str(env, "USA"),
        }
    }

    #[test]
    fn test_valid_passport_identity() {
        let env = Env::default();
        let identity = create_valid_identity(&env);
        assert!(validate_passenger_identity(&identity).is_ok());
    }

    #[test]
    fn test_valid_national_id_identity() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.document_type = String::from_str(&env, "national_id");
        identity.document_number = String::from_str(&env, "ID123456789");
        assert!(validate_passenger_identity(&identity).is_ok());
    }

    #[test]
    fn test_valid_driver_license_identity() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.document_type = String::from_str(&env, "driver_license");
        identity.document_number = String::from_str(&env, "D1234567");
        assert!(validate_passenger_identity(&identity).is_ok());
    }

    #[test]
    fn test_empty_full_name_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.full_name = String::from_str(&env, "");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidName)
        );
    }

    #[test]
    fn test_too_long_full_name_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.full_name = String::from_str(&env, "a".repeat(101));
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidName)
        );
    }

    #[test]
    fn test_invalid_email_missing_at_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.email = String::from_str(&env, "john.doeexample.com");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn test_invalid_email_missing_dot_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.email = String::from_str(&env, "john@doeexamplecom");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn test_empty_email_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.email = String::from_str(&env, "");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn test_invalid_phone_too_short_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.phone = String::from_str(&env, "+123");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidPhone)
        );
    }

    #[test]
    fn test_invalid_phone_too_long_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.phone = String::from_str(&env, "+1234567890123456"); // 16 chars
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidPhone)
        );
    }

    #[test]
    fn test_invalid_phone_missing_plus_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.phone = String::from_str(&env, "15551234567");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidPhone)
        );
    }

    #[test]
    fn test_empty_phone_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.phone = String::from_str(&env, "");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidPhone)
        );
    }

    #[test]
    fn test_invalid_document_type_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.document_type = String::from_str(&env, "invalid_type");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidDocumentNumber)
        );
    }

    #[test]
    fn test_empty_document_number_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.document_number = String::from_str(&env, "");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidDocumentNumber)
        );
    }

    #[test]
    fn test_too_long_document_number_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.document_number = String::from_str(&env, "a".repeat(51));
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidDocumentNumber)
        );
    }

    #[test]
    fn test_invalid_date_of_birth_wrong_format_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.date_of_birth = String::from_str(&env, "15-01-1990");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidDateOfBirth)
        );
    }

    #[test]
    fn test_invalid_date_of_birth_wrong_length_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.date_of_birth = String::from_str(&env, "1990-1-15");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidDateOfBirth)
        );
    }

    #[test]
    fn test_invalid_date_of_birth_non_numeric_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.date_of_birth = String::from_str(&env, "1990-ab-cd");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidDateOfBirth)
        );
    }

    #[test]
    fn test_invalid_nationality_wrong_length_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.nationality = String::from_str(&env, "US"); // 2 chars is valid
        assert!(validate_passenger_identity(&identity).is_ok());
        
        identity.nationality = String::from_str(&env, "U"); // 1 char invalid
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidNationality)
        );
        
        identity.nationality = String::from_str(&env, "USAA"); // 4 chars invalid
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidNationality)
        );
    }

    #[test]
    fn test_invalid_nationality_lowercase_fails() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.nationality = String::from_str(&env, "usa");
        assert_eq!(
            validate_passenger_identity(&identity),
            Err(ValidationError::InvalidNationality)
        );
    }

    #[test]
    fn test_valid_alpha2_nationality() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.nationality = String::from_str(&env, "GB");
        assert!(validate_passenger_identity(&identity).is_ok());
        
        identity.nationality = String::from_str(&env, "DE");
        assert!(validate_passenger_identity(&identity).is_ok());
    }

    #[test]
    fn test_valid_alpha3_nationality() {
        let env = Env::default();
        let mut identity = create_valid_identity(&env);
        identity.nationality = String::from_str(&env, "USA");
        assert!(validate_passenger_identity(&identity).is_ok());
        
        identity.nationality = String::from_str(&env, "GBR");
        assert!(validate_passenger_identity(&identity).is_ok());
    }
}
