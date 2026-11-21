use booking::{BookingContract, BookingContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};
use token::{TRQTokenContract, TRQTokenContractClient};

use integration_tests::{generate_actors, initialize_token, new_env, register_contracts};

#[test]
fn test_pay_for_booking_then_success() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    // Create booking
    let price = 100_0000000i128;
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

    // Pay once
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Status now confirmed; next test covers panic on second payment
}

#[test]
#[should_panic(expected = "Already paid or cancelled")]
fn test_pay_for_booking_again_should_panic() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);
    let price = 100_0000000i128;
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
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);
    contracts.booking.pay_for_booking(&booking_id);
}

#[test]
#[should_panic(expected = "Booking not found")]
fn test_pay_for_booking_nonexistent_should_panic() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);
    contracts.booking.pay_for_booking(&123456789u64);
}

#[test]
#[should_panic(expected = "Invalid booking status")]
fn test_release_payment_invalid_status_should_panic() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 50_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL321"),
        &Symbol::new(&env, "SFO"),
        &Symbol::new(&env, "SEA"),
        &1705067200,
        &price,
        &contracts.token.address,
    );

    contracts.booking.release_payment_to_airline(&booking_id);
}

#[test]
fn test_release_payment_success() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 50_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL321"),
        &Symbol::new(&env, "SFO"),
        &Symbol::new(&env, "SEA"),
        &1705067200,
        &price,
        &contracts.token.address,
    );

    // Confirm but no funds (no mint/transfer) -> will panic inside token client, but simulate correct flow
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Release successfully
    contracts.booking.release_payment_to_airline(&booking_id);
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "completed"));
    assert_eq!(booking.amount_escrowed, 0);
}

#[test]
fn test_refund_passenger_window_and_status_checks() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 25_0000000i128;
    // Set departure time far ahead to keep window open
    let departure = 2_000_000_000u64;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL777"),
        &Symbol::new(&env, "DXB"),
        &Symbol::new(&env, "DEL"),
        &departure,
        &price,
        &contracts.token.address,
    );

    // Pending -> refundable, but amount_escrowed = 0
    contracts.booking.refund_passenger(&booking_id);
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(booking.amount_escrowed, 0);

    // Reset to confirmed with escrow to test transfer
    let booking_id2 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL778"),
        &Symbol::new(&env, "DXB"),
        &Symbol::new(&env, "DEL"),
        &departure,
        &price,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id2);
    assert_eq!(
        contracts.token.balance_of(&contracts.booking.address),
        price
    );
    contracts.booking.refund_passenger(&booking_id2);
    assert_eq!(contracts.token.balance_of(&actors.passenger), price);
    assert_eq!(contracts.token.balance_of(&contracts.booking.address), 0);
}

#[test]
#[should_panic]
fn test_refund_passenger_window_closed_should_panic() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);
    let price = 25_0000000i128;
    let booking_id3 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL779"),
        &Symbol::new(&env, "DXB"),
        &Symbol::new(&env, "DEL"),
        &1_000, // very soon relative to current timestamp
        &price,
        &contracts.token.address,
    );
    contracts.booking.refund_passenger(&booking_id3);
}

#[test]
fn test_cancel_and_complete_wrappers() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 80_0000000i128;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL999"),
        &Symbol::new(&env, "NRT"),
        &Symbol::new(&env, "ICN"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );

    // Cancel wrapper (pending -> refunded)
    contracts
        .booking
        .cancel_booking(&actors.passenger, &booking_id);
    let b = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(b.status, Symbol::new(&env, "refunded"));

    // Complete wrapper path
    let booking_id2 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL1000"),
        &Symbol::new(&env, "NRT"),
        &Symbol::new(&env, "ICN"),
        &2_000_000_000,
        &price,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id2);
    contracts
        .booking
        .complete_booking(&actors.airline, &booking_id2);
    let b2 = contracts.booking.get_booking(&booking_id2).unwrap();
    assert_eq!(b2.status, Symbol::new(&env, "completed"));
}

#[test]
fn test_batch_complete_bookings_partial_failure() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price_ok = 30_0000000i128;
    let price_pending = 40_0000000i128;

    let booking_ok = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "BOK1"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &2_100_000_000,
        &price_ok,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price_ok);
    contracts.booking.pay_for_booking(&booking_ok);

    let booking_pending = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "BOK2"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "SFO"),
        &2_100_000_000,
        &price_pending,
        &contracts.token.address,
    );

    let other_airline = Address::generate(&env);
    let other_booking = contracts.booking.create_booking(
        &actors.passenger,
        &other_airline,
        &Symbol::new(&env, "BOK3"),
        &Symbol::new(&env, "MIA"),
        &Symbol::new(&env, "ORD"),
        &2_100_000_000,
        &price_ok,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price_ok);
    contracts.booking.pay_for_booking(&other_booking);

    let mut ids = Vec::new(&env);
    ids.push_back(booking_ok);
    ids.push_back(booking_pending);
    ids.push_back(999_999u64);
    ids.push_back(other_booking);

    let result = contracts
        .booking
        .batch_complete_bookings(&actors.airline, &ids);
    assert_eq!(result.completed_booking_ids.len(), 1);
    assert_eq!(result.failures.len(), 3);
    assert_eq!(result.total_released, price_ok);

    let completed = contracts.booking.get_booking(&booking_ok).unwrap();
    assert_eq!(completed.status, Symbol::new(&env, "completed"));

    let pending = contracts.booking.get_booking(&booking_pending).unwrap();
    assert_eq!(pending.status, Symbol::new(&env, "pending"));

    let untouched_other = contracts.booking.get_booking(&other_booking).unwrap();
    assert_eq!(untouched_other.status, Symbol::new(&env, "confirmed"));
}

#[test]
#[should_panic(expected = "Batch too large")]
fn test_batch_complete_bookings_enforces_max_batch_size() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let mut ids = Vec::new(&env);
    let mut i = 0;
    while i < 51 {
        ids.push_back(i as u64 + 1);
        i += 1;
    }

    contracts
        .booking
        .batch_complete_bookings(&actors.airline, &ids);
}

#[test]
fn test_recover_supplier_timeout() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = env.ledger().timestamp() + 86400; // 24h from now
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Simulate time passing beyond timeout threshold (departure + 24h)
    env.ledger().with_mut(|li| {
        li.timestamp = departure + 86400 + 1;
    });

    // Recover from supplier timeout
    contracts.booking.recover_supplier_timeout(&booking_id);
    
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "refunded"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&actors.passenger), price);
}

#[test]
#[should_panic(expected = "Timeout threshold not reached")]
fn test_recover_supplier_timeout_too_early() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    let departure = env.ledger().timestamp() + 86400;
    let booking_id = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL123"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );

    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Try to recover before timeout threshold
    contracts.booking.recover_supplier_timeout(&booking_id);
}

#[test]
fn test_recover_payment_pending_timeout() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
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

    // Booking is in pending status
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "pending"));
    assert_eq!(booking.amount_escrowed, 0);

    // Advance time past grace period (24 hours)
    env.ledger().with_mut(|li| {
        li.timestamp = 1704067200 + 86400 + 1;
    });

    // Recover payment pending - should cancel booking
    contracts.booking.recover_payment_pending(&booking_id);
    
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "cancelled"));
    assert_eq!(booking.amount_escrowed, 0);
}

#[test]
#[should_panic(expected = "Grace period not elapsed")]
fn test_recover_payment_pending_within_grace_period() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
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

    // Try to recover before grace period elapsed
    contracts.booking.recover_payment_pending(&booking_id);
}

#[test]
fn test_recover_partial_completion() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
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

    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Partial fulfillment: 3 of 5 legs completed (60% to airline, 40% refund to passenger)
    contracts.booking.recover_partial_completion(&booking_id, &3u32, &5u32);
    
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "completed"));
    assert_eq!(booking.amount_escrowed, 0);
    let airline_share = price * 3 / 5;
    let passenger_refund = price - airline_share;
    assert_eq!(contracts.token.balance_of(&actors.airline), airline_share);
    assert_eq!(contracts.token.balance_of(&actors.passenger), passenger_refund);
}

#[test]
fn test_recover_partial_completion_full() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
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

    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);

    // Full fulfillment: 4 of 4 legs completed (100% to airline)
    contracts.booking.recover_partial_completion(&booking_id, &4u32, &4u32);
    
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "completed"));
    assert_eq!(booking.amount_escrowed, 0);
    assert_eq!(contracts.token.balance_of(&actors.airline), price);
    assert_eq!(contracts.token.balance_of(&actors.passenger), 0);
}

#[test]
fn test_recover_contract_pending_payment_succeeds() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
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

    // Simulate contract pending: booking confirmed but amount_escrowed = 0
    // We need to directly manipulate storage to create this state
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id);
    
    // Manually set amount_escrowed to 0 in storage to simulate contract pending state
    let mut booking = contracts.booking.get_booking(&booking_id).unwrap();
    booking.amount_escrowed = 0;
    // Write back to storage
    use booking::BookingStorage;
    BookingStorage::set(&env, booking_id, &booking);
    
    // Passenger has balance, so recovery should succeed
    contracts.booking.recover_contract_pending(&booking_id);
    
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "confirmed"));
    assert_eq!(booking.amount_escrowed, price);
    assert_eq!(contracts.token.balance_of(&contracts.booking.address), price);
    assert_eq!(contracts.token.balance_of(&actors.passenger), 0);
}

#[test]
fn test_recover_contract_pending_payment_fails_cancels() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
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

    // Create contract pending state: confirmed with zero escrow
    let mut booking = contracts.booking.get_booking(&booking_id).unwrap();
    booking.amount_escrowed = 0;
    booking.status = Symbol::new(&env, "confirmed");
    use booking::BookingStorage;
    BookingStorage::set(&env, booking_id, &booking);

    // Passenger has NO balance (never minted tokens to them)
    // recover_contract_pending should fail and cancel the booking
    contracts.booking.recover_contract_pending(&booking_id);
    
    let booking = contracts.booking.get_booking(&booking_id).unwrap();
    assert_eq!(booking.status, Symbol::new(&env, "cancelled"));
    assert_eq!(booking.amount_escrowed, 0);
}

#[test]
fn test_get_recovery_status() {
    let env = new_env();
    let actors = generate_actors(&env);
    let contracts = register_contracts(&env);
    initialize_token(&env, &contracts.token, &actors.admin);

    let price = 100_0000000i128;
    
    // Test 1: Normal pending booking (within grace period)
    let booking_id1 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL1"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &1704067200,
        &price,
        &contracts.token.address,
    );
    let status1 = contracts.booking.get_recovery_status(&booking_id1);
    // RecoveryStatus::Normal = 0
    assert_eq!(status1, 0);

    // Test 2: Payment pending timeout (pending + 24h elapsed)
    let booking_id2 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL2"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &1704067200,
        &price,
        &contracts.token.address,
    );
    env.ledger().with_mut(|li| {
        li.timestamp = env.ledger().timestamp() + 86400 + 1;
    });
    let status2 = contracts.booking.get_recovery_status(&booking_id2);
    // RecoveryStatus::PaymentPendingTimeout = 1
    assert_eq!(status2, 1);

    // Test 3: Normal confirmed booking with escrow (before departure)
    let booking_id3 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL3"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &1704067200,
        &price,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id3);
    let status3 = contracts.booking.get_recovery_status(&booking_id3);
    assert_eq!(status3, 0); // Normal

    // Test 4: Contract pending (confirmed but zero escrow)
    let booking_id4 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL4"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &1704067200,
        &price,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id4);
    // Manually set amount_escrowed=0 in storage
    let mut booking4 = contracts.booking.get_booking(&booking_id4).unwrap();
    booking4.amount_escrowed = 0;
    use booking::BookingStorage;
    BookingStorage::set(&env, booking_id4, &booking4);
    let status4 = contracts.booking.get_recovery_status(&booking_id4);
    // RecoveryStatus::ContractPending = 2
    assert_eq!(status4, 2);
    
    // Test 5: Supplier timeout (confirmed, has escrow, past departure)
    let departure = env.ledger().timestamp() + 86400;
    let booking_id5 = contracts.booking.create_booking(
        &actors.passenger,
        &actors.airline,
        &Symbol::new(&env, "FL5"),
        &Symbol::new(&env, "JFK"),
        &Symbol::new(&env, "LAX"),
        &departure,
        &price,
        &contracts.token.address,
    );
    contracts
        .token
        .mint(&actors.admin, &actors.passenger, &price);
    contracts.booking.pay_for_booking(&booking_id5);
    
    // Before departure - Normal
    let status5a = contracts.booking.get_recovery_status(&booking_id5);
    assert_eq!(status5a, 0);
    
    // After departure - SupplierTimeout
    env.ledger().with_mut(|li| {
        li.timestamp = departure + 1;
    });
    let status5b = contracts.booking.get_recovery_status(&booking_id5);
    // RecoveryStatus::SupplierTimeout = 3
    assert_eq!(status5b, 3);

    // Test 6: Completed booking - Normal
    contracts.booking.release_payment_to_airline(&booking_id5);
    let status6 = contracts.booking.get_recovery_status(&booking_id5);
    assert_eq!(status6, 0);
}
