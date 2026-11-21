use booking::{BookingContract, BookingContractClient};
use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};
use token::{TRQTokenContract, TRQTokenContractClient};

#[test]
fn test_booking_timeout_and_cancellation_recovery() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = 2_000_000_000u64;

    // Create booking in pending state (simulating supplier/payment timeout preparation)
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "TIMEOUT1"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // Recover by cancelling the pending booking due to timeout
    contracts
        .booking
        .cancel_booking(&actors.passenger, &booking_id);
    let recovered_booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(recovered_booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(recovered_booking.amount_escrowed, 0);
}

#[test]
fn test_contract_pending_and_partial_completion_recovery() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 50_0000000i128;
    let departure = 2_000_000_000u64;

    let b1 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "PART1"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "ORD"),
        &departure,
        &price,
        &contracts.token.address,
    );

    let b2 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "PART2"),
        &Symbol::new(&env, "ORD"),
        &Symbol::new(&env, "SFO"),
        &departure,
        &price,
        &contracts.token.address,
    );

    // Fund and complete b1 normally
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&b1);
    contracts.booking.release_payment_to_airline(&b1);
    assert_eq!(
        contracts.booking.get_booking(&b1).unwrap().status,
        Symbol::new(&env, "completed")
    );

    // Leave b2 pending/unpaid and verify recovery via passenger cancellation
    contracts.booking.cancel_booking(&actors.passenger, &b2);
    assert_eq!(
        contracts.booking.get_booking(&b2).unwrap().status,
        Symbol::new(&env, "refunded")
    );
}
