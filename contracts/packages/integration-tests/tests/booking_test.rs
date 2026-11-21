use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{testutils::Ledger, Symbol, String};
use booking::PassengerIdentity;

#[test]
fn create_booking_with_valid_passengers_succeeds() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    let passenger_identity = PassengerIdentity {
        full_name: String::from_str(&env, "John Doe"),
        date_of_birth: 631152000, // 1990-01-01
        nationality: String::from_str(&env, "US"),
        document_type: String::from_str(&env, "passport"),
        document_number: String::from_str(&env, "A1234567"),
        email: String::from_str(&env, "john.doe@example.com"),
        phone: String::from_str(&env, "+15551234567"),
    };

    let result = contracts.booking.create_booking_with_identity(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &passenger_identity,
    );

    assert!(result.is_ok());
    let booking_id = result.unwrap();
    assert!(booking_id > 0);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.passenger, actors.passenger);
}

#[test]
fn create_booking_with_invalid_email_rejected() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    let mut passenger_identity = PassengerIdentity {
        full_name: String::from_str(&env, "John Doe"),
        date_of_birth: 631152000,
        nationality: String::from_str(&env, "US"),
        document_type: String::from_str(&env, "passport"),
        document_number: String::from_str(&env, "A1234567"),
        email: String::from_str(&env, "invalid-email"), // Invalid: no @
        phone: String::from_str(&env, "+15551234567"),
    };

    let result = contracts.booking.create_booking_with_identity(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &passenger_identity,
    );

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), booking::ValidationError::InvalidEmail);
}

#[test]
fn create_booking_with_missing_document_rejected() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    let mut passenger_identity = PassengerIdentity {
        full_name: String::from_str(&env, "John Doe"),
        date_of_birth: 631152000,
        nationality: String::from_str(&env, "US"),
        document_type: String::from_str(&env, "passport"),
        document_number: String::from_str(&env, ""), // Missing document number
        email: String::from_str(&env, "john.doe@example.com"),
        phone: String::from_str(&env, "+15551234567"),
    };

    let result = contracts.booking.create_booking_with_identity(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &passenger_identity,
    );

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), booking::ValidationError::MissingField);
}

#[test]
fn create_booking_with_underage_passenger_rejected() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    // Born 1 year ago (under 2 years old)
    let current_time = env.ledger().timestamp();
    let one_year_ago = current_time - 365 * 24 * 60 * 60;

    let passenger_identity = PassengerIdentity {
        full_name: String::from_str(&env, "Baby Doe"),
        date_of_birth: one_year_ago,
        nationality: String::from_str(&env, "US"),
        document_type: String::from_str(&env, "passport"),
        document_number: String::from_str(&env, "A1234567"),
        email: String::from_str(&env, "baby.doe@example.com"),
        phone: String::from_str(&env, "+15551234567"),
    };

    let result = contracts.booking.create_booking_with_identity(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &passenger_identity,
    );

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), booking::ValidationError::UnderAge);
}

#[test]
fn test_payment_escrow_flow() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128; // 100 USDC

    // 1. Create booking
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &1704067200,
        &price,
        &contracts.token.address,
    );

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // 2. Mint tokens to passenger and Pay
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "confirmed"));
    assert_eq!(booking.amount_escrowed, price);
    assert_eq!(contracts.token.balance_of(&actors.passenger), 0);
    assert_eq!(
        contracts.token.balance_of(&contracts.booking.address),
        price
    );
    assert_eq!(contracts.token.balance_of(&actors.airline), 0);

    // 3. Release escrow to the airline after fulfillment.
    contracts.booking.release_payment_to_airline(&booking_id);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "completed"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&contracts.booking.address), 0);
    assert_eq!(contracts.token.balance_of(&actors.airline), price);
}

#[test]
fn test_refund_flow() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000; // Far in the future

    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
    );

    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Refund passenger
    contracts.booking.refund_passenger(&booking_id);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(booking.amount_escrowed, 0);
}
