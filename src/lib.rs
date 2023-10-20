#![no_std]
use alloc::string::{String, ToString};
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Map, Symbol, Vec
};

extern crate alloc;

#[contracttype]
enum DataKey {
    Genesis,
    World,
    Register,
}

type Bitmap = u128;
type Index = u128;
type Query = Bitmap;

#[contracttype]
pub struct World {
    name: String,
    counter: Index,
    entities: Map<Index, (Bitmap, Vec<Address>)>,
    systems: Map<Query, Address>
}

trait Registered {
    fn register(env: &Env, address: Address) -> Option<Bitmap>;
    fn unregister(env: &Env, system: Address);
}

trait System {
    fn add_system(self, query: Query, address: Address) -> Self;
    fn remove_system(self, query: Query) -> Self;
}

impl System for World {
    fn add_system(mut self, query: Query, system: Address) -> Self {
        self.systems.set(query, system);
        self
    }

    fn remove_system(mut self, query: Query) -> Self {
        self.systems.remove(query);
        self
    }
}

impl World {
    fn spawn<R: Registered>(mut self, env: &Env, components: Vec<Address>) -> (bool, Self) {
        let mut bitmap = None;
        let mut filtered_components = Vec::new(&env);

        for component in components.into_iter() {
            if let Some(a) = R::register(env, component.clone()) {
                bitmap = match bitmap {
                    None => Some(a),
                    Some(b) => Some(a + b),
                };
                filtered_components.push_back(component);
            }
        }

        if let Some(bitmap) = bitmap {
            self.counter = self.counter + 1;
            self.entities.set(self.counter, (bitmap, filtered_components));
            return (true, self);
        }

        (false, self)
    }

    fn despawn<R: Registered>(self, env: &Env, component: Address) -> Self {
        R::unregister(env, component);
        self
    }
}
#[contracttype]
pub struct Register {
    counter: Bitmap,
    addresses: Vec<Address>,
    map: Map<Bitmap, Address>,
}

impl Registered for Register {
    fn register(env: &Env, address: Address) -> Option<Bitmap> {
        let mut register: Register =
            env.storage()
                .instance()
                .get(&DataKey::Register)
                .unwrap_or_else(|| Register {
                    counter: 0,
                    addresses: Vec::new(env),
                    map: Map::new(env),
                });

        if !register.addresses.contains(address.clone()) {
            register.counter = register.counter + 1;
            register.addresses.push_back(address.clone());
            register.map.set(register.counter, address);

            return Some(1 << register.counter);
        }

        None
    }

    fn unregister(env: &Env, address: Address) {
        let mut register: Register =
            env.storage()
                .instance()
                .get(&DataKey::Register)
                .expect("best to have a register before we unregister!");

        if let Ok(index) = register.addresses.binary_search(address.clone()) {
            register.addresses.remove(index);
        }
    }
}

#[contract]
pub struct Contract;
#[contractimpl]
impl Contract {

    fn check_genesis(env: &Env) -> bool {
        env.storage().instance().get(&DataKey::Genesis).unwrap_or(false)
    }
    /// The genesis of the world, ran once, in which we set a name for the world
    pub fn genesis(env: Env, name: Symbol) {
        if !Self::check_genesis(&env) {
            env.storage().instance().set(&DataKey::Genesis, &true);
            let world = World {
                name: name.to_string(),
                entities: Map::new(&env),
                counter: Default::default(),
                systems: Map::new(&env),
            };
            env.storage().instance().set(&DataKey::World, &world);
        }
    }

    /// Get the world
    pub fn get_world(env: Env) -> World {
        env.storage()
            .instance()
            .get(&DataKey::World)
            .expect("Seems we genesis has yet to happen :)")
    }

    /// Spawn an entity in the world with a list of components
    pub fn spawn(env: Env, components: Vec<Address>) {
        if Self::check_genesis(&env) {

            let (updated, world) = env
                .storage()
                .instance()
                .get::<_, World>(&DataKey::World)
                .expect("what happened to my world!")
                .spawn::<Register>(&env, components);

            if updated {
                env.storage().instance().set(&DataKey::World, &world);
            }
        }
    }

    /// Despawn an entity in the world
    pub fn despawn(env: Env, component: Address) {
        if Self::check_genesis(&env) {
            env.storage()
                .instance()
                .get::<_, World>(&DataKey::World)
                .expect("what happened to my world!")
                .despawn::<Register>(&env, component);
        }
    }

    /// Add system to world
    pub fn add_system(env: Env, query: Query, system: Address) {
        if Self::check_genesis(&env) {
            env.storage()
                .instance()
                .get::<_, World>(&DataKey::World)
                .expect("what happened to my world!")
                .add_system(query, system);
        }
    }

    pub fn remove_system(env: Env, query: Query) {
        if Self::check_genesis(&env) {
            env.storage()
                .instance()
                .get::<_, World>(&DataKey::World)
                .expect("what happened to my world!")
                .remove_system(query);
        }
    }
}
#[test]
fn hello() {
    let env = Env::default();
    let contract_id = env.register_contract(None, Contract);
    let client = ContractClient::new(&env, &contract_id);

    // let words = client.hello(&symbol_short!("Dev"));
    // assert_eq!(
    //     words,
    //     vec![&env, symbol_short!("Hello"), symbol_short!("Dev"),]
    // );
}
