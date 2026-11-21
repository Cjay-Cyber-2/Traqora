#![no_std]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol};

fn create_test_booking(env: &Env) -> (u64, Address, Address) {
    let passenger = Address::generate(env);
    let airline = Address::generate(env);
    let token = Address::generate(env);
    
    let booking_id = BookingContract::create_booking(
        env.clone(),
        passenger.clone(),
        airline.clone(),
        Symbol::new(env, "FL123"),
        Symbol::new(env, "JFK"),
        Symbol::new(env, "LAX"),
        env.ledger().timestamp() + 86400,
        1000,
        token.clone(),
    );
    
    // Pay for booking to confirm it
    let token_client = token::Client::new(env, &token);
    token_client.mint(&passenger, &10000);
    BookingContract::pay_for_booking(env.clone(), booking_id);
    
    (booking_id, passenger, airline)
}

#[test]n test_set_supplier_reference_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (booking_id, _passenger, airline) = create_test_booking(&env);
    
    let reference = String::from_str(&env, "PNR123456");
    BookingContract::set_supplier_reference(env.clone(), booking_id, reference.clone());
    
    let stored = BookingContract::get_booking(env.clone(), booking_id).unwrap();
    assert_eq!(stored.supplier_reference, Some(reference));
}

#[test]n test_set_supplier_reference_only_airline() {
    let env = Env::default();
    let (booking_id, passenger, airline) = create_test_booking(&env);
    
    let reference = String::from_str(&env, "PNR123456");
    
    // Passenger cannot set supplier reference
    let result = std::panic::catch_unwind(|| {
        BookingContract::set_supplier_reference(env.clone(), booking_id, reference);
    });
    assert!(result.is_err());
}

#[test]n test_set_supplier_reference_confirmed_or_completed_only() {
    let env = Env::default();
    env.mock_all_auths();
    
    let passenger = Address::generate(&env);
    let airline = Address::generate(&env);
    let token = Address::generate(&env);
    
    let booking_id = BookingContract::create_booking(
        env.clone(),
        passenger.clone(),
        airline.clone(),
        Symbol::new(&env, "FL123"),
        Symbol::new(&env, "JFK"),
        Symbol::new(&env, "LAX"),
        env.ledger().timestamp() + 86400,
        1000,
        token.clone(),
    );
    
    // Try to set reference on pending booking (not paid yet)
    let reference = String::from_str(&env, "PNR123456");
    let result = std::panic::catch_unwind(|| {
        BookingContract::set_supplier_reference(env.clone(), booking_id, reference);
    });
    assert!(result.is_err(), "Should fail on pending booking");
    
    // Pay for booking
    let token_client = token::Client::new(&env, &token);
    token_client.mint(&passenger, &10000);
    BookingContract::pay_for_booking(env.clone(), booking_id);
    
    // Now should work on confirmed booking
    let reference = String::from_str(&env, "PNR123456");
    BookingContract::set_supplier_reference(env.clone(), booking_id, reference);
    
    let stored = BookingContract::get_booking(env.clone(), booking_id).unwrap();
    assert_eq!(stored.supplier_reference, Some(String::from_str(&env, "PNR123456")));
}

#[test]n test_get_supplier_reference_passenger() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (booking_id, passenger, airline) = create_test_booking(&env);
    
    let reference = String::from_str(&env, "PNR123456");
    BookingContract::set_supplier_reference(env.clone(), booking_id, reference.clone());
    
    let result = BookingContract::get_supplier_reference(env.clone(), booking_id, passenger);
    assert_eq!(result, Some(reference));
}

#[test]n test_get_supplier_reference_airline() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (booking_id, passenger, airline) = create_test_booking(&env);
    
    let reference = String::from_str(&env, "PNR123456");
    BookingContract::set_supplier_reference(env.clone(), booking_id, reference.clone());
    
    let result = BookingContract::get_supplier_reference(env.clone(), booking_id, airline);
    assert_eq!(result, Some(reference));
}

#[test]n test_get_supplier_reference_unauthorized() {
    let env = Env::default();
    let (booking_id, _passenger, _airline) = create_test_booking(&env);
    
    let reference = String::from_str(&env, "PNR123456");
    BookingContract::set_supplier_reference(env.clone(), booking_id, reference);
    
    let unauthorized = Address::generate(&env);
    let result = std::panic::catch_unwind(|| {
        BookingContract::get_supplier_reference(env.clone(), booking_id, unauthorized);
    });
    assert!(result.is_err());
}

#[test]n test_get_supplier_reference_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (booking_id, passenger, _airline) = create_test_booking(&env);
    
    let result = BookingContract::get_supplier_reference(env.clone(), booking_id, passenger);
    assert_eq!(result, None);
}

#[test]n test_supplier_reference_redacted_in_events() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (booking_id, _passenger, airline) = create_test_booking(&env);
    
    let reference = String::from_str(&env, "PNR123456");
    BookingContract::set_supplier_reference(env.clone(), booking_id, reference);
    
    // Verify event was emitted with supplier_ref_set action
    // The reference itself should NOT be in the event (redacted)
    let events = env.events().all();
    let supplier_ref_events: Vec<_> = events
        .iter()
        .filter(|e| {
            let topics = e.topics;
            topics.get(3).map(|t| t == Symbol::new(&env, "supplier_ref_set")).unwrap_or(false)
        })
        .collect();
    
    assert_eq!(supplier_ref_events.len(), 1);
    // Event should contain: airline, timestamp, booking_id, action - but NOT the reference
    let event = &supplier_ref_events[0];
    assert_eq!(event.data.len(), 4); // actor, timestamp, booking_id, action
}