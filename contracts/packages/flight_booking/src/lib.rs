#![no_std]
use contract_events::{Action, Domain};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, String};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookingState {
    Pending,
    Confirmed,
    Cancelled,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PassengerIdentity {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub document_type: String,
    pub document_number: String,
    pub date_of_birth: String,
    pub nationality: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    MissingField,
    InvalidEmail,
    InvalidPhone,
    InvalidDocumentNumber,
    InvalidDateOfBirth,
    InvalidNationality,
    InvalidName,
}

#[contracttype]
#[derive(Clone)]
pub struct Booking {
    pub booking_id: u64,
    pub passenger: Address,
    pub flight_id: Symbol,
    pub seat: Symbol,
    pub amount: i128,
    pub escrowed_amount: i128,
    pub state: BookingState,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    NextId,
    Booking(u64),
}

fn validate_passenger_identity(identity: &PassengerIdentity) -> Result<(), ValidationError> {
    // Validate full_name: non-empty, reasonable length
    if identity.full_name.len() == 0 || identity.full_name.len() > 100 {
        return Err(ValidationError::InvalidName);
    }

    // Validate email: basic format check
    if identity.email.len() == 0 || identity.email.len() > 254 {
        return Err(ValidationError::InvalidEmail);
    }
    let email_str = identity.email.to_string();
    if !email_str.contains('@') || !email_str.contains('.') {
        return Err(ValidationError::InvalidEmail);
    }

    // Validate phone: non-empty, reasonable length (E.164 format roughly)
    if identity.phone.len() == 0 || identity.phone.len() > 20 {
        return Err(ValidationError::InvalidPhone);
    }
    let phone_str = identity.phone.to_string();
    if !phone_str.starts_with('+') {
        return Err(ValidationError::InvalidPhone);
    }

    // Validate document_type: non-empty, known types
    if identity.document_type.len() == 0 {
        return Err(ValidationError::MissingField);
    }
    let doc_type = identity.document_type.to_string();
    if doc_type != "passport" && doc_type != "national_id" && doc_type != "driver_license" {
        return Err(ValidationError::InvalidDocumentNumber);
    }

    // Validate document_number: non-empty, alphanumeric
    if identity.document_number.len() == 0 || identity.document_number.len() > 50 {
        return Err(ValidationError::InvalidDocumentNumber);
    }
    let doc_num = identity.document_number.to_string();
    if !doc_num.chars().all(|c| c.is_alphanumeric()) {
        return Err(ValidationError::InvalidDocumentNumber);
    }

    // Validate date_of_birth: non-empty, ISO 8601 format (YYYY-MM-DD)
    if identity.date_of_birth.len() != 10 {
        return Err(ValidationError::InvalidDateOfBirth);
    }
    let dob = identity.date_of_birth.to_string();
    if !dob.chars().nth(4).map_or(true, |c| c == '-') ||
       !dob.chars().nth(7).map_or(true, |c| c == '-') {
        return Err(ValidationError::InvalidDateOfBirth);
    }
    // Basic check that parts are numeric
    let parts: Vec<&str> = dob.split('-').collect();
    if parts.len() != 3 ||
       parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 ||
       !parts[0].chars().all(|c| c.is_ascii_digit()) ||
       !parts[1].chars().all(|c| c.is_ascii_digit()) ||
       !parts[2].chars().all(|c| c.is_ascii_digit()) {
        return Err(ValidationError::InvalidDateOfBirth);
    }

    // Validate nationality: non-empty, ISO 3166-1 alpha-2 or alpha-3
    if identity.nationality.len() == 0 || identity.nationality.len() > 3 {
        return Err(ValidationError::InvalidNationality);
    }
    let nat = identity.nationality.to_string();
    if !nat.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError::InvalidNationality);
    }

    Ok(())
}

#[contract]
pub struct FlightBookingContract;

#[contractimpl]
impl FlightBookingContract {
    pub fn reserve_seat(
        env: Env,
        passenger: Address,
        flight_id: Symbol,
        seat: Symbol,
        amount: i128,
        passenger_identity: PassengerIdentity,
    ) -> u64 {
        passenger.require_auth();
        assert!(amount > 0, "Invalid amount");

        // Validate passenger identity before proceeding to payment/escrow
        match validate_passenger_identity(&passenger_identity) {
            Ok(()) => {},
            Err(e) => panic_with_error!(&env, e),
        }

        let seat_key = (symbol_short!("seat"), flight_id.clone(), seat.clone());
        if env.storage().persistent().has(&seat_key) {
            panic!("Seat already reserved");
        }

        let booking_id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
        let created_at = env.ledger().timestamp();

        let booking = Booking {
            booking_id,
            passenger: passenger.clone(),
            flight_id: flight_id.clone(),
            seat: seat.clone(),
            amount,
            escrowed_amount: amount,
            state: BookingState::Confirmed,
            created_at,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Booking(booking_id), &booking);
        env.storage().persistent().set(&seat_key, &booking_id);
        env.storage()
            .instance()
            .set(&DataKey::NextId, &(booking_id + 1));

        contract_events::emit(
            &env,
            Domain::Seat,
            Action::Reserved,
            (
                passenger.clone(),
                created_at,
                booking_id,
                flight_id,
                seat,
                amount,
            ),
        );

        booking_id
    }

    pub fn get_booking(env: Env, booking_id: u64) -> Option<Booking> {
        env.storage()
            .persistent()
            .get(&DataKey::Booking(booking_id))
    }

    pub fn init_upgrade_owner(env: Env, owner: Address) {
        access::UpgradeTimelock::init_upgrade_owner(&env, &owner);
    }
}
