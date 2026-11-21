use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{testutils::Ledger, Symbol, String};

#[test]
fn test_payment_escrow_flow() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128; // 100 USDC
    let departure_time = 1704067200;

    // 1. Create booking with valid passenger identity
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &946684800, // 2000-01-01
        &Symbol::new(&env, "USA"),
    );

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(booking.passenger_identity.full_name, String::from_str(&env, "John Doe"));
    assert_eq!(booking.passenger_identity.document_type, Symbol::new(&env, "passport"));
    assert_eq!(booking.passenger_identity.document_number, String::from_str(&env, "A1234567"));
    assert_eq!(booking.passenger_identity.date_of_birth, 946684800);
    assert_eq!(booking.passenger_identity.nationality, Symbol::new(&env, "USA"));

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
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &946684800, // 2000-01-01
        &Symbol::new(&env, "USA"),
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

#[test]
#[should_panic(expected = "Invalid full name length")]
fn test_create_booking_invalid_full_name_empty() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    // Empty full name should panic
    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, ""),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &946684800,
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Invalid full name length")]
fn test_create_booking_invalid_full_name_too_long() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;
    let long_name = "A".repeat(101);

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, &long_name),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &946684800,
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Invalid document type")]
fn test_create_booking_invalid_document_type() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "invalid_type"),
        &String::from_str(&env, "A1234567"),
        &946684800,
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Invalid document number length")]
fn test_create_booking_invalid_document_number_empty() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, ""),
        &946684800,
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Invalid document number length")]
fn test_create_booking_invalid_document_number_too_long() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;
    let long_doc = "A".repeat(51);

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, &long_doc),
        &946684800,
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Date of birth cannot be in the future")]
fn test_create_booking_future_date_of_birth() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &1800000000, // Future date
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Passenger must be at least 2 years old")]
fn test_create_booking_passenger_too_young() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &1690000000, // Less than 2 years ago
        &Symbol::new(&env, "USA"),
    );
}

#[test]
#[should_panic(expected = "Nationality must be 3-letter ISO code")]
fn test_create_booking_invalid_nationality_length() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &946684800,
        &Symbol::new(&env, "US"), // 2 chars, not 3
    );
}

#[test]
#[should_panic(expected = "Nationality code must be uppercase letters")]
fn test_create_booking_invalid_nationality_lowercase() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "John Doe"),
        &Symbol::new(&env, "passport"),
        &String::from_str(&env, "A1234567"),
        &946684800,
        &Symbol::new(&env, "usa"), // lowercase
    );
}

#[test]
fn test_create_booking_valid_national_id() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "Jane Smith"),
        &Symbol::new(&env, "national_id"),
        &String::from_str(&env, "ID987654321"),
        &946684800,
        &Symbol::new(&env, "GBR"),
    );

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.passenger_identity.document_type, Symbol::new(&env, "national_id"));
    assert_eq!(booking.passenger_identity.nationality, Symbol::new(&env, "GBR"));
}

#[test]
fn test_create_booking_valid_driver_license() {
    let env = new_env();
    env.ledger().set_timestamp(1700000000);

    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure_time = 1705000000;

    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure_time,
        &price,
        &contracts.token.address,
        &String::from_str(&env, "Alice Johnson"),
        &Symbol::new(&env, "driver_license"),
        &String::from_str(&env, "DL123456789"),
        &946684800,
        &Symbol::new(&env, "CAN"),
    );

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.passenger_identity.document_type, Symbol::new(&env, "driver_license"));
    assert_eq!(booking.passenger_identity.nationality, Symbol::new(&env, "CAN"));
}
