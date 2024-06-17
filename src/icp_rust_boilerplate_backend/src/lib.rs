#[macro_use]
extern crate serde;
extern crate candid;
extern crate ic_cdk;
extern crate ic_stable_structures;

use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::collections::BTreeMap;
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(
    candid::CandidType, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Default, Debug,
)]
enum BookingStatus {
    #[default]
    Available,
    Booked,
}

// Station Struct
#[derive(candid::CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
struct Station {
    id: u64,
    name: String,
    funds: u64,
    train_ids: Vec<u64>,
}

// Train Struct
#[derive(candid::CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
struct Train {
    id: u64,
    departure_station: String,
    arrival_station: String,
    seats: BTreeMap<u64, BookingStatus>,
    price: u64,
    schedule: u64,
}

// Ticket Struct
#[derive(candid::CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
struct Ticket {
    id: u64,
    train_id: u64,
    owner: String,
    seat_number: u64,
    launch_time: u64,
}

// AdminCap Struct
#[derive(candid::CandidType, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
struct AdminCap {
    admin_id: u64,
}

// Payloads

// Station Payload
#[derive(candid::CandidType, Serialize, Deserialize, Clone, Debug)]
struct StationPayload {
    name: String,
    funds: u64,
}

// Train Payload
#[derive(candid::CandidType, Serialize, Deserialize, Clone, Debug)]
struct TrainPayload {
    departure_station: String,
    arrival_station: String,
    seat_count: u64,
    price: u64,
    schedule: u64,
}

// Ticket Payload
#[derive(candid::CandidType, Serialize, Deserialize, Clone, Debug)]
struct TicketPayload {
    train_id: u64,
    owner: String,
    seat_number: u64,
}

// Close Train Payload
#[derive(candid::CandidType, Serialize, Deserialize, Clone, Debug)]
struct CloseTrainPayload {
    train_id: u64,
}

// Error types
#[derive(candid::CandidType, Deserialize, Serialize, Debug)]
enum Error {
    NotFound { msg: String },
    UnAuthorized { msg: String },
    InvalidInput { msg: String },
    AlreadyBooked { msg: String },
    TrainDeparted { msg: String },
}

impl Storable for Station {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Station {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Train {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Train {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Ticket {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Ticket {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for AdminCap {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for AdminCap {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STATIONS_STORAGE: RefCell<StableBTreeMap<u64, Station, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static TRAINS_STORAGE: RefCell<StableBTreeMap<u64, Train, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static TICKETS_STORAGE: RefCell<StableBTreeMap<u64, Ticket, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));

    static ADMINS_STORAGE: RefCell<StableBTreeMap<u64, AdminCap, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4)))
    ));
}

#[ic_cdk::update]
fn init_system(admin_id: u64, station_payload: StationPayload) -> Result<String, String> {
    let admin_cap = AdminCap { admin_id };

    ADMINS_STORAGE.with(|storage| storage.borrow_mut().insert(admin_id, admin_cap));

    let station_id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let station = Station {
        id: station_id,
        name: station_payload.name,
        funds: station_payload.funds,
        train_ids: vec![],
    };

    STATIONS_STORAGE.with(|storage| storage.borrow_mut().insert(station_id, station));

    Ok(format!(
        "System initialized with admin {} and station {}",
        admin_id, station_id
    ))
}

#[ic_cdk::update]
fn create_train(admin_id: u64, payload: TrainPayload) -> Result<Train, Error> {
    let admin_exists = ADMINS_STORAGE.with(|storage| storage.borrow().contains_key(&admin_id));

    if !admin_exists {
        return Err(Error::UnAuthorized {
            msg: "Unauthorized access".to_string(),
        });
    }

    let departure_station_exists = STATIONS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, station)| station.name == payload.departure_station)
    });
    let arrival_station_exists = STATIONS_STORAGE.with(|storage| {
        storage
            .borrow()
            .iter()
            .any(|(_, station)| station.name == payload.arrival_station)
    });
    
    if !departure_station_exists || !arrival_station_exists {
        return Err(Error::InvalidInput {
            msg: "Invalid station name(s)".to_string(),
        });
    }

    let train_id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let seats = (1..=payload.seat_count)
        .map(|seat| (seat, BookingStatus::Available))
        .collect();

    let train = Train {
        id: train_id,
        departure_station: payload.departure_station.clone(),
        arrival_station: payload.arrival_station.clone(),
        seats,
        price: payload.price,
        schedule: payload.schedule,
    };

    TRAINS_STORAGE.with(|storage| storage.borrow_mut().insert(train_id, train.clone()));

    STATIONS_STORAGE.with(|storage| {
        let mut storage_borrow = storage.borrow_mut();
        let departure_station_opt = storage_borrow
            .iter()
            .find(|(_, station)| station.name == payload.departure_station);
        if let Some((departure_station_id, _)) = departure_station_opt {
            let mut station = storage_borrow.remove(&departure_station_id).unwrap();
            station.train_ids.push(train_id);
            storage_borrow.insert(departure_station_id, station);
        }
    });

    Ok(train)
}

#[ic_cdk::update]
fn buy_ticket(payload: TicketPayload) -> Result<Ticket, Error> {
    let train_exists =
        TRAINS_STORAGE.with(|storage| storage.borrow().contains_key(&payload.train_id));

    if !train_exists {
        return Err(Error::NotFound {
            msg: "Train not found".to_string(),
        });
    }

    let mut train =
        TRAINS_STORAGE.with(|storage| storage.borrow().get(&payload.train_id).unwrap().clone());
    
    // if train.schedule <= time() {
    //     return Err(Error::TrainDeparted {
    //         msg: "Train has already departed".to_string(),
    //     });
    // }

    if train.seats.get(&payload.seat_number) != Some(&BookingStatus::Available) {
        return Err(Error::AlreadyBooked {
            msg: "Seat already booked".to_string(),
        });
    }

    train
        .seats
        .insert(payload.seat_number, BookingStatus::Booked);

    TRAINS_STORAGE.with(|storage| storage.borrow_mut().insert(payload.train_id, train));

    let ticket_id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment ID counter");

    let ticket = Ticket {
        id: ticket_id,
        train_id: payload.train_id,
        owner: payload.owner,
        seat_number: payload.seat_number,
        launch_time: time(),
    };

    TICKETS_STORAGE.with(|storage| storage.borrow_mut().insert(ticket_id, ticket.clone()));

    Ok(ticket)
}

#[ic_cdk::update]
fn refund_ticket(ticket_id: u64) -> Result<String, Error> {
    let ticket_exists = TICKETS_STORAGE.with(|storage| storage.borrow().contains_key(&ticket_id));

    if !ticket_exists {
        return Err(Error::NotFound {
            msg: "Ticket not found".to_string(),
        });
    }

    let ticket = TICKETS_STORAGE.with(|storage| storage.borrow().get(&ticket_id).unwrap().clone());

    if ticket.launch_time <= time() {
        return Err(Error::TrainDeparted {
            msg: "Train has already departed".to_string(),
        });
    }

    let mut train =
        TRAINS_STORAGE.with(|storage| storage.borrow().get(&ticket.train_id).unwrap().clone());

    train
        .seats
        .insert(ticket.seat_number, BookingStatus::Available);

    TRAINS_STORAGE.with(|storage| storage.borrow_mut().insert(ticket.train_id, train));
    TICKETS_STORAGE.with(|storage| storage.borrow_mut().remove(&ticket_id));

    Ok(format!("Ticket {} refunded successfully", ticket_id))
}

#[ic_cdk::update]
fn close_train(admin_id: u64, payload: CloseTrainPayload) -> Result<String, Error> {
    let admin_exists = ADMINS_STORAGE.with(|storage| storage.borrow().contains_key(&admin_id));

    if !admin_exists {
        return Err(Error::UnAuthorized {
            msg: "Unauthorized access".to_string(),
        });
    }

    let train_exists =
        TRAINS_STORAGE.with(|storage| storage.borrow().contains_key(&payload.train_id));

    if !train_exists {
        return Err(Error::NotFound {
            msg: "Train not found".to_string(),
        });
    }

    let train =
        TRAINS_STORAGE.with(|storage| storage.borrow().get(&payload.train_id).unwrap().clone());

    STATIONS_STORAGE.with(|storage| {
        let mut storage_borrow = storage.borrow_mut();
        let departure_station_opt = storage_borrow
            .iter()
            .find(|(_, station)| station.name == train.departure_station);
        if let Some((departure_station_id, _)) = departure_station_opt {
            let mut station = storage_borrow.remove(&departure_station_id).unwrap();
            station.train_ids.retain(|&id| id != payload.train_id);
            storage_borrow.insert(departure_station_id, station);
        }
    });

    TRAINS_STORAGE.with(|storage| storage.borrow_mut().remove(&payload.train_id));

    Ok(format!("Train {} closed successfully", payload.train_id))
}

#[ic_cdk::query]
fn view_train(train_id: u64) -> Result<Train, Error> {
    let train_exists = TRAINS_STORAGE.with(|storage| storage.borrow().contains_key(&train_id));

    if !train_exists {
        return Err(Error::NotFound {
            msg: "Train not found".to_string(),
        });
    }

    let train = TRAINS_STORAGE.with(|storage| storage.borrow().get(&train_id).unwrap().clone());

    Ok(train)
}

#[ic_cdk::query]
fn view_station(station_id: u64) -> Result<Station, Error> {
    let station_exists =
        STATIONS_STORAGE.with(|storage| storage.borrow().contains_key(&station_id));

    if !station_exists {
        return Err(Error::NotFound {
            msg: "Station not found".to_string(),
        });
    }

    let station =
        STATIONS_STORAGE.with(|storage| storage.borrow().get(&station_id).unwrap().clone());

    Ok(station)
}

ic_cdk::export_candid!();
