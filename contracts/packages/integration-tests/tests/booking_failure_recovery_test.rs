use booking::{BookingContract, BookingContractClient};
use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};
use token::{TRQTokenContract, TRQTokenContractClient};

#[test]
fn test_booking_lifecycle_failure_recovery_pending_to_paid_flow() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 50_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "RECOV1"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    // Verify initial pending state (simulating payment pending / timeout recovery prep)
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // Recovery: Mint and pay successfully after initial pending delay
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    let recovered_booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(recovered_booking.status, Symbol::new(&env, "confirmed"));
    assert_eq!(recovered_booking.amount_escrowed, price);
}

#[test]
#[should_panic(expected = "Already paid or cancelled")]
fn test_booking_lifecycle_duplicate_payment_recovery_rejection() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 50_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "RECOV2"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "SFO"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &(price * 2));
    contracts.booking.pay_for_booking(&booking_id);
    // Attempting recovery re-payment should fail safely
    contracts.booking.pay_for_booking(&booking_id);
}
