use booking::{BookingContract, BookingContractClient};
use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};
use token::{TRQTokenContract, TRQTokenContractClient};

#[test]
fn test_booking_failure_recovery_documentation_scenarios() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 50_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FLRECOV"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &1704067200,
        &price,
        &contracts.token.address,
    );

    // Verify pending state before payment (Payment Pending recovery scenario)
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // Cancel pending booking (simulating timeout / abandonment recovery)
    contracts
        .booking
        .cancel_booking(&actors.passenger, &booking_id);
    let canceled_booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(canceled_booking.status, Symbol::new(&env, "refunded"));
}
