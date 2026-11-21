#![no_std]
use contract_events::{Action, Domain};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Symbol};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookingState {
    Pending,
    Confirmed,
    Cancelled,
    Refunded,
}

#[contracttype]
#[derive(Clone)]
pub struct Booking {
    pub booking_id: u64,
    pub passenger: Address,
    pub flight_id: Symbol,
    pub seat: Symbol,
    pub amount: i128,
    pub escrowed_amount: i128,
    pub state: BookingState,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplierBookingKey {
    pub supplier_id: String,
    pub idempotency_key: String,
}

#[contracttype]
#[derive(Clone)]
pub struct SupplierBookingSubmittedEvent {
    pub supplier_id: String,
    pub idempotency_key: String,
    pub booking_id: u64,
    pub is_duplicate: bool,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    NextId,
    Booking(u64),
    SupplierBooking(SupplierBookingKey),
}

#[contract]
pub struct FlightBookingContract;

#[contractimpl]
impl FlightBookingContract {
    pub fn reserve_seat(
        env: Env,
        passenger: Address,
        flight_id: Symbol,
        seat: Symbol,
        amount: i128,
    ) -> u64 {
        passenger.require_auth();
        assert!(amount > 0, "Invalid amount");

        let seat_key = (symbol_short!("seat"), flight_id.clone(), seat.clone());
        if env.storage().persistent().has(&seat_key) {
            panic!("Seat already reserved");
        }

        let booking_id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
        let created_at = env.ledger().timestamp();

        let booking = Booking {
            booking_id,
            passenger: passenger.clone(),
            flight_id: flight_id.clone(),
            seat: seat.clone(),
            amount,
            escrowed_amount: amount,
            state: BookingState::Confirmed,
            created_at,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Booking(booking_id), &booking);
        env.storage().persistent().set(&seat_key, &booking_id);
        env.storage()
            .instance()
            .set(&DataKey::NextId, &(booking_id + 1));

        contract_events::emit(
            &env,
            Domain::Seat,
            Action::Reserved,
            (
                passenger.clone(),
                created_at,
                booking_id,
                flight_id,
                seat,
                amount,
            ),
        );

        booking_id
    }

    pub fn submit_booking(
        env: Env,
        supplier_id: String,
        idempotency_key: String,
        passenger: Address,
        flight_id: Symbol,
        seat: Symbol,
        amount: i128,
    ) -> u64 {
        assert!(amount > 0, "Invalid amount");

        let supplier_key = SupplierBookingKey {
            supplier_id: supplier_id.clone(),
            idempotency_key: idempotency_key.clone(),
        };
        let data_key = DataKey::SupplierBooking(supplier_key.clone());

        if let Some(existing_booking_id) = env.storage().persistent().get::<_, u64>(&data_key) {
            contract_events::emit(
                &env,
                Domain::Booking,
                Action::Submitted,
                SupplierBookingSubmittedEvent {
                    supplier_id,
                    idempotency_key,
                    booking_id: existing_booking_id,
                    is_duplicate: true,
                },
            );
            return existing_booking_id;
        }

        let seat_key = (symbol_short!("seat"), flight_id.clone(), seat.clone());
        if env.storage().persistent().has(&seat_key) {
            panic!("Seat already reserved");
        }

        let booking_id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
        let created_at = env.ledger().timestamp();

        let booking = Booking {
            booking_id,
            passenger: passenger.clone(),
            flight_id: flight_id.clone(),
            seat: seat.clone(),
            amount,
            escrowed_amount: amount,
            state: BookingState::Confirmed,
            created_at,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Booking(booking_id), &booking);
        env.storage().persistent().set(&seat_key, &booking_id);
        env.storage().persistent().set(&data_key, &booking_id);
        env.storage()
            .instance()
            .set(&DataKey::NextId, &(booking_id + 1));

        contract_events::emit(
            &env,
            Domain::Booking,
            Action::Submitted,
            SupplierBookingSubmittedEvent {
                supplier_id,
                idempotency_key,
                booking_id,
                is_duplicate: false,
            },
        );

        booking_id
    }

    pub fn get_booking(env: Env, booking_id: u64) -> Option<Booking> {
        env.storage()
            .persistent()
            .get(&DataKey::Booking(booking_id))
    }

    pub fn reconcile_supplier_booking(env: Env, supplier_id: String, idempotency_key: String) -> Option<u64> {
        let supplier_key = SupplierBookingKey {
            supplier_id,
            idempotency_key,
        };
        let data_key = DataKey::SupplierBooking(supplier_key);
        env.storage().persistent().get(&data_key)
    }

    pub fn init_upgrade_owner(env: Env, owner: Address) {
        access::UpgradeTimelock::init_upgrade_owner(&env, &owner);
    }
}
