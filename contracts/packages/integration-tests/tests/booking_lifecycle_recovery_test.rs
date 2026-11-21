use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, Symbol};
use booking::BookingStatus;

#[test]
fn test_supplier_timeout_refund_before_departure() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64; // Far future
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL100"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    // Simulate supplier timeout: booking stuck in Pending, no payment
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));

    // Passenger can refund (cancel) while still pending
    contracts.booking.refund_passenger(&booking_id);
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
}

#[test]
fn test_supplier_timeout_refund_after_payment_before_departure() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64; // Far future
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL101"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    // Payment made, supplier timeout (airline doesn't fulfill)
    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "confirmed"));
    assert_eq!(booking.amount_escrowed, price);

    // Passenger refunds before departure
    contracts.booking.refund_passenger(&booking_id);
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&actors.passenger), price);
}

#[test]
#[should_panic(expected = "Refund window closed")]
fn test_supplier_timeout_refund_window_closed_after_departure() {
    let env = new_env();
    env.ledger().set_timestamp(1_500_000_000); // Set time past departure
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = 1_000_000_000u64; // Past
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL102"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Refund should fail - window closed
    contracts.booking.refund_passenger(&booking_id);
}

#[test]
fn test_payment_pending_expiry() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL103"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    // Booking in pending, no payment yet
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // Passenger can cancel (refund) while pending
    contracts.booking.cancel_booking(&actors.passenger, &booking_id);
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
}

#[test]
fn test_payment_pending_then_pay_succeeds() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL104"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    // Payment after some time in pending
    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "confirmed"));
    assert_eq!(booking.amount_escrowed, price);
}

#[test]
#[should_panic(expected = "Already paid or cancelled")]
fn test_payment_pending_duplicate_payment_fails() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL105"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);
    contracts.booking.pay_for_booking(&booking_id); // Should panic
}

#[test]
fn test_contract_pending_reconciliation() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL106"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Simulate backend reading on-chain state for reconciliation
    let on_chain_booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(on_chain_booking.status, Symbol::new(&env, "confirmed"));
    assert_eq!(on_chain_booking.amount_escrowed, price);
    assert_eq!(on_chain_booking.passenger, actors.passenger);
    assert_eq!(on_chain_booking.airline, actors.airline);

    // Events emitted for indexing/reconciliation
    // (booking, created) and (booking, paid) should be in event log
}

#[test]
fn test_partial_completion_batch_mixed_results() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price1 = 100_0000000i128;
    let price2 = 200_0000000i128;
    let price3 = 150_0000000i128;
    let departure = 2_000_000_000u64;

    // Create 3 bookings
    let id1 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL200"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price1,
        &contracts.token.address,
    );
    let id2 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL201"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "SFO"),
        &departure,
        &price2,
        &contracts.token.address,
    );
    let id3 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL202"),
        &Symbol::new(&env, "MIA"),
        &Symbol::new(&env, "ORD"),
        &departure,
        &price3,
        &contracts.token.address,
    );

    // Pay for all
    contracts.token.mint(&actors.admin, &actors.passenger, &(price1 + price2 + price3));
    contracts.booking.pay_for_booking(&id1);
    contracts.booking.pay_for_booking(&id2);
    contracts.booking.pay_for_booking(&id3);

    // Simulate partial completion: airline fulfills 1 and 3, but 2 has issue
    contracts.booking.release_payment_to_airline(&id1);
    // id2 NOT released - simulate airline issue
    contracts.booking.release_payment_to_airline(&id3);

    let b1 = contracts.booking.get_booking(&id1).unwrap();
    let b2 = contracts.booking.get_booking(&id2).unwrap();
    let b3 = contracts.booking.get_booking(&id3).unwrap();

    assert_eq!(b1.status, Symbol::new(&env, "completed"));
    assert_eq!(b2.status, Symbol::new(&env, "confirmed")); // Still pending release
    assert_eq!(b3.status, Symbol::new(&env, "completed"));

    // Recovery: retry release for id2
    contracts.booking.release_payment_to_airline(&id2);
    let b2_retry = contracts.booking.get_booking(&id2).unwrap();
    assert_eq!(b2_retry.status, Symbol::new(&env, "completed"));
}

#[test]
fn test_oracle_refund_airline_cancel() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    // Set up oracle
    contracts.booking.initialize_oracle(&actors.oracle);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL300"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Airline cancels flight - oracle triggers refund
    contracts.booking.oracle_refund_airline_cancel(&booking_id);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&actors.passenger), price);
}

#[test]
fn test_oracle_release_payment() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    // Set up oracle
    contracts.booking.initialize_oracle(&actors.oracle);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL301"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Oracle confirms flight completion
    contracts.booking.oracle_release_payment(&booking_id);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "completed"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&actors.airline), price);
}

#[test]
fn test_cancellation_settlement_partial() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL400"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Settle cancellation: split between passenger and airline
    // (e.g., 50% refund to passenger, 50% to airline as fee)
    let passenger_refund = 50_0000000i128;
    let airline_amount = 50_0000000i128;
    contracts.booking.settle_cancellation(&booking_id, &passenger_refund, &airline_amount);

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "cancelled"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&actors.passenger), passenger_refund);
    assert_eq!(contracts.token.balance_of(&actors.airline), airline_amount);
}

#[test]
fn test_booking_status_enum_values() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL500"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    // Verify all status transitions use correct Symbol values
    let pending = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(pending.status, Symbol::new(&env, "pending"));

    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);
    let confirmed = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(confirmed.status, Symbol::new(&env, "confirmed"));

    contracts.booking.refund_passenger(&booking_id);
    let refunded = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(refunded.status, Symbol::new(&env, "refunded"));

    // Test completed path
    let booking_id2 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL501"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );
    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id2);
    contracts.booking.release_payment_to_airline(&booking_id2);
    let completed = contracts.booking.get_booking(&booking_id2).unwrap();
    assert_eq!(completed.status, Symbol::new(&env, "completed"));

    // Test cancelled path
    let booking_id3 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL502"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );
    contracts.token.mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id3);
    contracts.booking.settle_cancellation(&booking_id3, &price, &0);
    let cancelled = contracts.booking.get_booking(&booking_id3).unwrap();
    assert_eq!(cancelled.status, Symbol::new(&env, "cancelled"));
}
