use booking::{BookingContract, BookingContractClient};
use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{Address, Env, Symbol, Vec};
use token::{TRQTokenContract, TRQTokenContractClient};

#[test]
fn test_booking_payment_pending_timeout_recovery_flow() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL900"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // Simulate cancellation/recovery of pending booking without funds locked
    contracts
        .booking
        .cancel_booking(&actors.passenger, &booking_id);
    let recovered_booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(recovered_booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(recovered_booking.amount_escrowed, 0);
}

#[test]
#[should_panic(expected = "Booking not found")]
fn test_supplier_timeout_nonexistent_booking_recovery() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    // Attempting to query or release non-existent booking resulting from timeout
    contracts.booking.get_booking(&999999u64).unwrap();
}
