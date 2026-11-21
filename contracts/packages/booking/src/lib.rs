#![no_std]
// Booking-style contract entrypoints legitimately need many arguments;
// soroban macro-generated clients re-declare these signatures.
#![allow(clippy::too_many_arguments)]
use contract_events::{Action, Domain};
use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, symbol_short, token, Address, Env,
    Map, String, Symbol, Vec,
};

#[contractclient(name = "BookingReceiptClient")]
#[allow(clippy::too_many_arguments)]
pub trait BookingReceiptInterface {
    #[allow(clippy::too_many_arguments)]
    fn mint_receipt(
        env: Env,
        to: Address,
        booking_id: u64,
        flight_number: Symbol,
        from_airport: Symbol,
        to_airport: Symbol,
        seat: String,
        price: i128,
    ) -> u64;
}

#[contracttype]
#[derive(Clone)]
pub struct PassengerIdentity {
    pub full_name: String,
    pub document_type: Symbol, // "passport", "national_id", "driver_license"
    pub document_number: String,
    pub date_of_birth: u64,    // Unix timestamp
    pub nationality: Symbol,   // ISO 3166-1 alpha-3 country code
}

#[contracttype]
#[derive(Clone)]
pub struct Booking {
    pub booking_id: u64,
    pub passenger: Address,
    pub passenger_identity: PassengerIdentity,
    pub airline: Address,
    pub flight_number: Symbol,
    pub from_airport: Symbol,
    pub to_airport: Symbol,
    pub departure_time: u64,
    pub price: i128,
    pub token: Address,
    pub amount_escrowed: i128,
    pub status: Symbol, // "pending", "confirmed", "completed", "cancelled", "refunded"
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct BatchFailure {
    pub index: u32,
    pub booking_id: u64,
    pub reason: Symbol,
}

#[contracttype]
#[derive(Clone)]
pub struct BatchCompleteBookingsResult {
    pub completed_booking_ids: Vec<u64>,
    pub failures: Vec<BatchFailure>,
    pub total_released: i128,
}

pub struct BookingStorage;

const MAX_BATCH_SIZE: u32 = 50;

impl BookingStorage {
    pub fn get(env: &Env, booking_id: u64) -> Option<Booking> {
        env.storage().persistent().get(&booking_id)
    }

    pub fn set(env: &Env, booking_id: u64, booking: &Booking) {
        env.storage().persistent().set(&booking_id, booking);
    }

    pub fn get_trusted_oracle(env: &Env) -> Option<Address> {
        env.storage().instance().get(&symbol_short!("oracle"))
    }

    pub fn set_trusted_oracle(env: &Env, oracle: &Address) {
        env.storage()
            .instance()
            .set(&symbol_short!("oracle"), oracle);
    }

    pub fn get_receipt_contract(env: &Env) -> Option<Address> {
        env.storage().instance().get(&symbol_short!("receipt_c"))
    }

    pub fn set_receipt_contract(env: &Env, contract: &Address) {
        env.storage()
            .instance()
            .set(&symbol_short!("receipt_c"), contract);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn next_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("next_id"))
            .unwrap_or(1);
        env.storage()
            .instance()
            .set(&symbol_short!("next_id"), &(id + 1));
        id
    }
}

#[contract]
pub struct BookingContract;

#[allow(clippy::too_many_arguments)]
#[contractimpl]
impl BookingContract {
    // Register the trusted oracle contract address
    pub fn initialize_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();
        BookingStorage::set_trusted_oracle(&env, &oracle);
        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Oracle,
            (admin, env.ledger().timestamp(), oracle),
        );
    }

    pub fn set_receipt_contract(env: Env, admin: Address, receipt_contract: Address) {
        admin.require_auth();
        BookingStorage::set_receipt_contract(&env, &receipt_contract);
    }

    /// Initialize booking - starts in "pending" status until paid
    ///
    /// # Passenger Identity Validation
    /// All passenger identity fields are required and validated:
    /// - `passenger_full_name`: 1-100 characters
    /// - `passenger_document_type`: Must be one of `passport`, `national_id`, `driver_license`
    /// - `passenger_document_number`: 1-50 characters
    /// - `passenger_date_of_birth`: Unix timestamp in the past, passenger must be at least 2 years old
    /// - `passenger_nationality`: 3-letter uppercase ISO 3166-1 alpha-3 country code
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `passenger` - Passenger address (must authorize)
    /// * `airline` - Airline address
    /// * `flight_number` - Flight number symbol
    /// * `from_airport` - Origin airport symbol
    /// * `to_airport` - Destination airport symbol
    /// * `departure_time` - Unix timestamp of departure
    /// * `price` - Booking price in token units
    /// * `token` - Token contract address
    /// * `passenger_full_name` - Passenger's full legal name
    /// * `passenger_document_type` - Document type: passport, national_id, or driver_license
    /// * `passenger_document_number` - Document number
    /// * `passenger_date_of_birth` - Date of birth as Unix timestamp
    /// * `passenger_nationality` - ISO 3166-1 alpha-3 country code (3 uppercase letters)
    ///
    /// # Returns
    /// * `u64` - The created booking ID
    ///
    /// # Panics
    /// Panics if any passenger identity validation fails
    #[allow(clippy::too_many_arguments)]
    pub fn create_booking(
        env: Env,
        passenger: Address,
        airline: Address,
        flight_number: Symbol,
        from_airport: Symbol,
        to_airport: Symbol,
        departure_time: u64,
        price: i128,
        token: Address,
        passenger_full_name: String,
        passenger_document_type: Symbol,
        passenger_document_number: String,
        passenger_date_of_birth: u64,
        passenger_nationality: Symbol,
    ) -> u64 {
        passenger.require_auth();

        // Validate required passenger identity fields
        Self::validate_passenger_identity(
            &env,
            &passenger_full_name,
            &passenger_document_type,
            &passenger_document_number,
            passenger_date_of_birth,
            &passenger_nationality,
        );

        let booking_id = BookingStorage::next_id(&env);

        let passenger_identity = PassengerIdentity {
            full_name: passenger_full_name,
            document_type: passenger_document_type,
            document_number: passenger_document_number,
            date_of_birth: passenger_date_of_birth,
            nationality: passenger_nationality,
        };

        let booking = Booking {
            booking_id,
            passenger,
            passenger_identity,
            airline,
            flight_number,
            from_airport,
            to_airport,
            departure_time,
            price,
            token,
            amount_escrowed: 0,
            status: symbol_short!("pending"),
            created_at: env.ledger().timestamp(),
        };

        BookingStorage::set(&env, booking_id, &booking);

        // Canonical schema: (actor, timestamp, primary_id, ...payload)
        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Created,
            (
                booking.passenger.clone(),
                env.ledger().timestamp(),
                booking_id,
                booking.airline.clone(),
                booking.flight_number.clone(),
                booking.price,
            ),
        );

        booking_id
    }

    // Accept payment for the booking and hold in escrow
    pub fn pay_for_booking(env: Env, booking_id: u64) {
        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");

        assert!(
            booking.status == symbol_short!("pending"),
            "Already paid or cancelled"
        );

        booking.passenger.require_auth();

        let token_client = token::Client::new(&env, &booking.token);

        // Transfer tokens from passenger to this contract
        token_client.transfer(
            &booking.passenger,
            &env.current_contract_address(),
            &booking.price,
        );

        booking.amount_escrowed = booking.price;
        booking.status = symbol_short!("confirmed");

        BookingStorage::set(&env, booking_id, &booking);

        if let Some(receipt_contract) = BookingStorage::get_receipt_contract(&env) {
            let client = BookingReceiptClient::new(&env, &receipt_contract);
            client.mint_receipt(
                &booking.passenger,
                &booking_id,
                &booking.flight_number,
                &booking.from_airport,
                &booking.to_airport,
                &String::from_str(&env, "TBD"), // Seat is assigned later or TBD initially
                &booking.price,
            );
        }

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Paid,
            (
                booking.passenger.clone(),
                env.ledger().timestamp(),
                booking_id,
                booking.price,
            ),
        );
    }

    // Release payment to airline - post-flight settlement
    pub fn release_payment_to_airline(env: Env, booking_id: u64) {
        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");

        booking.airline.require_auth();

        assert!(
            booking.status == symbol_short!("confirmed"),
            "Invalid booking status"
        );
        assert!(booking.amount_escrowed > 0, "No funds in escrow");

        let token_client = token::Client::new(&env, &booking.token);

        token_client.transfer(
            &env.current_contract_address(),
            &booking.airline,
            &booking.amount_escrowed,
        );

        let released_amount = booking.amount_escrowed;
        booking.amount_escrowed = 0;
        booking.status = symbol_short!("completed");

        BookingStorage::set(&env, booking_id, &booking);

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Released,
            (
                booking.airline.clone(),
                env.ledger().timestamp(),
                booking_id,
                released_amount,
            ),
        );
    }

    // Refund passenger for cancelled bookings
    pub fn refund_passenger(env: Env, booking_id: u64) {
        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");

        let current_time = env.ledger().timestamp();

        // For simplicity, require passenger auth and check window
        // In a real app, airline could also trigger this
        booking.passenger.require_auth();
        assert!(
            current_time < booking.departure_time - 86400,
            "Cancellation window closed"
        );

        assert!(
            booking.status == symbol_short!("confirmed")
                || booking.status == symbol_short!("pending"),
            "Booking cannot be refunded"
        );

        if booking.amount_escrowed > 0 {
            let token_client = token::Client::new(&env, &booking.token);
            token_client.transfer(
                &env.current_contract_address(),
                &booking.passenger,
                &booking.amount_escrowed,
            );
        }

        let refunded_amount = booking.amount_escrowed;
        booking.amount_escrowed = 0;
        booking.status = symbol_short!("refunded");

        BookingStorage::set(&env, booking_id, &booking);

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Refunded,
            (
                booking.passenger.clone(),
                env.ledger().timestamp(),
                booking_id,
                refunded_amount,
            ),
        );
    }

    // Renew the TTL (Time To Live) for a booking entry to prevent pruning
    // of long-lived persistent storage. This must be called before the entry's
    // TTL expires to keep the booking record readable for the booking and
    // dispute lifecycle.
    //
    // # Arguments
    // * `booking_id` - The booking ID whose TTL should be renewed
    pub fn renew_booking_ttl(env: Env, booking_id: u64) {
        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");
        booking.passenger.require_auth();
        env.storage()
            .persistent()
            .extend_ttl(&booking_id, /* delta = */ 0);
    }

    // Validate required passenger identity fields
    fn validate_passenger_identity(
        env: &Env,
        full_name: &String,
        document_type: &Symbol,
        document_number: &String,
        date_of_birth: u64,
        nationality: &Symbol,
    ) {
        // Full name must not be empty and must be reasonable length
        assert!(
            full_name.len() > 0 && full_name.len() <= 100,
            "Invalid full name length"
        );

        // Document type must be one of the accepted types
        let valid_doc_types = Map::new(env);
        let valid_doc_types = valid_doc_types.set(symbol_short!("passport"), true);
        let valid_doc_types = valid_doc_types.set(symbol_short!("national_id"), true);
        let valid_doc_types = valid_doc_types.set(symbol_short!("driver_license"), true);
        assert!(
            valid_doc_types.get(document_type.clone()).unwrap_or(false),
            "Invalid document type"
        );

        // Document number must not be empty and must be reasonable length
        assert!(
            document_number.len() > 0 && document_number.len() <= 50,
            "Invalid document number length"
        );

        // Date of birth must be in the past (not future) and person must be at least 2 years old
        let current_time = env.ledger().timestamp();
        let min_age_seconds: u64 = 2 * 365 * 24 * 60 * 60; // 2 years in seconds
        assert!(date_of_birth < current_time, "Date of birth cannot be in the future");
        assert!(
            current_time - date_of_birth >= min_age_seconds,
            "Passenger must be at least 2 years old"
        );

        // Nationality must be a valid ISO 3166-1 alpha-3 code (3 uppercase letters)
        let nat_str: String = nationality.to_string();
        assert!(nat_str.len() == 3, "Nationality must be 3-letter ISO code");
        // Basic check: all characters should be uppercase letters
        let nat_bytes = nat_str.to_bytes();
        let mut i = 0;
        while i < nat_bytes.len() {
            let b = nat_bytes.get(i).unwrap();
            assert!(b >= 65 && b <= 90, "Nationality code must be uppercase letters");
            i += 1;
        }
    }

    // Helper to get booking details
    pub fn get_booking(env: Env, booking_id: u64) -> Option<Booking> {
        BookingStorage::get(&env, booking_id)
    }

    // Original API wrappers for backward compatibility
    pub fn cancel_booking(env: Env, passenger: Address, booking_id: u64) {
        passenger.require_auth();
        Self::refund_passenger(env, booking_id);
    }

    pub fn complete_booking(env: Env, airline: Address, booking_id: u64) {
        airline.require_auth();
        Self::release_payment_to_airline(env, booking_id);
    }

    // Settle cancellation payouts from escrow according to refund basis points.
    // `passenger_refund_bps` is in basis points (10000 = 100%).
    pub fn settle_cancellation(
        env: Env,
        booking_id: u64,
        caller: Address,
        passenger_refund_bps: u32,
    ) -> (i128, i128) {
        assert!(passenger_refund_bps <= 10_000, "Invalid refund bps");

        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");

        assert!(
            booking.status != symbol_short!("cancelled")
                && booking.status != symbol_short!("refunded")
                && booking.status != symbol_short!("completed"),
            "Booking cannot be cancelled"
        );

        caller.require_auth();
        assert!(
            caller == booking.passenger || caller == booking.airline,
            "Not authorized to cancel"
        );

        assert!(
            booking.status == symbol_short!("confirmed")
                || booking.status == symbol_short!("pending"),
            "Invalid booking status"
        );

        let escrowed = booking.amount_escrowed;
        let mut passenger_refund = 0i128;
        let mut airline_amount = 0i128;

        if escrowed > 0 {
            passenger_refund = escrowed * (passenger_refund_bps as i128) / 10_000;
            airline_amount = escrowed - passenger_refund;

            let token_client = token::Client::new(&env, &booking.token);

            if passenger_refund > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &booking.passenger,
                    &passenger_refund,
                );
            }

            if airline_amount > 0 {
                token_client.transfer(
                    &env.current_contract_address(),
                    &booking.airline,
                    &airline_amount,
                );
            }
        }

        booking.amount_escrowed = 0;
        booking.status = symbol_short!("cancelled");
        BookingStorage::set(&env, booking_id, &booking);

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Cancelled,
            (
                caller,
                env.ledger().timestamp(),
                booking_id,
                passenger_refund,
                airline_amount,
            ),
        );

        (passenger_refund, airline_amount)
    }

    // Batch post-flight settlement with partial failure handling.
    // Gas savings come from one auth check and a single transaction envelope.
    pub fn batch_complete_bookings(
        env: Env,
        airline: Address,
        booking_ids: Vec<u64>,
    ) -> BatchCompleteBookingsResult {
        airline.require_auth();
        assert!(!booking_ids.is_empty(), "Empty batch");
        assert!(booking_ids.len() <= MAX_BATCH_SIZE, "Batch too large");

        let mut completed_booking_ids = Vec::new(&env);
        let mut failures = Vec::new(&env);
        let mut total_released: i128 = 0;

        let mut i: u32 = 0;
        while i < booking_ids.len() {
            let booking_id = booking_ids.get(i).unwrap();
            let mut booking = match BookingStorage::get(&env, booking_id) {
                Some(existing) => existing,
                None => {
                    failures.push_back(BatchFailure {
                        index: i,
                        booking_id,
                        reason: symbol_short!("missing"),
                    });
                    i += 1;
                    continue;
                }
            };

            if booking.airline != airline {
                failures.push_back(BatchFailure {
                    index: i,
                    booking_id,
                    reason: symbol_short!("unauth"),
                });
                i += 1;
                continue;
            }

            if booking.status != symbol_short!("confirmed") {
                failures.push_back(BatchFailure {
                    index: i,
                    booking_id,
                    reason: symbol_short!("bad_stat"),
                });
                i += 1;
                continue;
            }

            if booking.amount_escrowed <= 0 {
                failures.push_back(BatchFailure {
                    index: i,
                    booking_id,
                    reason: symbol_short!("no_funds"),
                });
                i += 1;
                continue;
            }

            let token_client = token::Client::new(&env, &booking.token);
            token_client.transfer(
                &env.current_contract_address(),
                &booking.airline,
                &booking.amount_escrowed,
            );

            let released_amount = booking.amount_escrowed;
            total_released += released_amount;
            booking.amount_escrowed = 0;
            booking.status = symbol_short!("completed");
            BookingStorage::set(&env, booking_id, &booking);
            completed_booking_ids.push_back(booking_id);

            contract_events::emit(
                &env,
                Domain::Booking,
                Action::Released,
                (
                    booking.airline.clone(),
                    env.ledger().timestamp(),
                    booking_id,
                    released_amount,
                ),
            );

            i += 1;
        }

        BatchCompleteBookingsResult {
            completed_booking_ids,
            failures,
            total_released,
        }
    }

    // Oracle-triggered settlement: called by the oracle contract after flight completion consensus
    pub fn oracle_release_payment(env: Env, oracle: Address, booking_id: u64) {
        oracle.require_auth();
        let trusted = BookingStorage::get_trusted_oracle(&env).expect("Oracle not configured");
        assert!(oracle == trusted, "Unauthorized oracle");

        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");

        assert!(
            booking.status == symbol_short!("confirmed"),
            "Invalid booking status"
        );
        assert!(booking.amount_escrowed > 0, "No funds in escrow");

        let token_client = token::Client::new(&env, &booking.token);
        token_client.transfer(
            &env.current_contract_address(),
            &booking.airline,
            &booking.amount_escrowed,
        );

        let released_amount = booking.amount_escrowed;
        booking.amount_escrowed = 0;
        booking.status = symbol_short!("completed");
        BookingStorage::set(&env, booking_id, &booking);

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Released,
            (
                oracle,
                env.ledger().timestamp(),
                booking_id,
                released_amount,
            ),
        );
    }

    // Oracle-triggered refund: called by the oracle contract after airline cancellation consensus
    pub fn oracle_refund_airline_cancel(env: Env, oracle: Address, booking_id: u64) {
        oracle.require_auth();
        let trusted = BookingStorage::get_trusted_oracle(&env).expect("Oracle not configured");
        assert!(oracle == trusted, "Unauthorized oracle");

        let mut booking = BookingStorage::get(&env, booking_id).expect("Booking not found");

        assert!(
            booking.status == symbol_short!("confirmed")
                || booking.status == symbol_short!("pending"),
            "Booking cannot be refunded"
        );

        if booking.amount_escrowed > 0 {
            let token_client = token::Client::new(&env, &booking.token);
            token_client.transfer(
                &env.current_contract_address(),
                &booking.passenger,
                &booking.amount_escrowed,
            );
        }

        let refunded_amount = booking.amount_escrowed;
        booking.amount_escrowed = 0;
        booking.status = symbol_short!("refunded");
        BookingStorage::set(&env, booking_id, &booking);

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Refunded,
            (
                oracle,
                env.ledger().timestamp(),
                booking_id,
                refunded_amount,
            ),
        );
    }
}
