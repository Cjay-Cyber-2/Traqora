#![no_std]
use contract_events::{Action, Domain};
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

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
    pub supplier_booking_reference: Option<soroban_sdk::String>,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    NextId,
    Booking(u64),
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
            supplier_booking_reference: None,
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

    pub fn get_booking(env: Env, booking_id: u64) -> Option<Booking> {
        env.storage()
            .persistent()
            .get(&DataKey::Booking(booking_id))
    }

    pub fn record_supplier_booking_reference(
        env: Env,
        booking_id: u64,
        caller: Address,
        supplier_booking_reference: soroban_sdk::String,
    ) {
        caller.require_auth();
        let mut booking = Self::get_booking(env.clone(), booking_id).expect("Booking not found");
        assert!(caller == booking.passenger, "Not authorized");

        booking.supplier_booking_reference = Some(supplier_booking_reference);
        env.storage()
            .persistent()
            .set(&DataKey::Booking(booking_id), &booking);

        contract_events::emit(
            &env,
            Domain::Seat,
            Action::Updated,
            (
                caller,
                env.ledger().timestamp(),
                booking_id,
                symbol_short!("sup_ref"),
            ),
        );
    }

    pub fn init_upgrade_owner(env: Env, owner: Address) {
        access::UpgradeTimelock::init_upgrade_owner(&env, &owner);
    }
}
