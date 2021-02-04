use actix::{Actor, Context, Supervised, Addr, Arbiter, Supervisor, System, SystemRegistry, SystemService};
use std::any::{TypeId, Any};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use parking_lot::Mutex;


static SREG: Lazy<Mutex<HashMap<usize, PatchedSystemRegistry>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));


#[derive(Debug)]
struct PatchedSystemRegistry {
    system: Arbiter,
    registry: HashMap<TypeId, Box<dyn Any + Send>>,
}


impl PatchedSystemRegistry {
    pub(crate) fn new(system: Arbiter) -> Self {
        Self {
            system,
            registry: HashMap::default(),
        }
    }

    /// Return address of the service. If service actor is not running
    /// it get started in the system.
    pub fn get<A: SystemService + Actor<Context = Context<A>>>(&mut self) -> Addr<A> {
        if let Some(addr) = self.registry.get(&TypeId::of::<A>()) {
            match addr.downcast_ref::<Addr<A>>() {
                Some(addr) => return addr.clone(),
                None => panic!("Got unknown value: {:?}", addr),
            }
        }

        let addr = A::start_service(&self.system);
        self.registry
            .insert(TypeId::of::<A>(), Box::new(addr.clone()));
        addr
    }

    /// Check if actor is in registry, if so, return its address
    pub fn query<A: SystemService + Actor<Context = Context<A>>>(
        &self,
    ) -> Option<Addr<A>> {
        if let Some(addr) = self.registry.get(&TypeId::of::<A>()) {
            match addr.downcast_ref::<Addr<A>>() {
                Some(addr) => return Some(addr.clone()),
                None => return None,
            }
        }

        None
    }

    /// Add new actor to the registry by address, panic if actor is already running
    pub fn set<A: SystemService + Actor<Context = Context<A>>>(addr: Addr<A>) {
        System::with_current(|sys| {
            let mut sreg = SREG.lock();
            let reg = sreg
                .entry(sys.id())
                .or_insert_with(|| PatchedSystemRegistry::new(sys.arbiter().clone()));

            if let Some(addr) = reg.registry.get(&TypeId::of::<A>()) {
                if addr.downcast_ref::<Addr<A>>().is_some() {
                    panic!("Actor already started");
                }
            }

            reg.registry.insert(TypeId::of::<A>(), Box::new(addr));
        })
    }
}


/// Trait defines custom system's service.
pub trait CustomSystemService: Actor<Context = Context<Self>> + SystemService {
    /// Construct and start system service with arguments
    fn start_service_with(f: impl Fn() -> Self + std::marker::Sync + 'static + std::marker::Send) -> Addr<Self> {
        let sys = System::current();
        let arbiter = sys.arbiter();
        let addr = Supervisor::start_in_arbiter(arbiter, move |ctx| {
            let mut act = f();
            act.service_started(ctx);
            act
        });
        Self::add_to_registry(addr)
    }

    fn custom_service_started(&mut self, ctx: &mut Context<Self>) {}

    fn add_to_registry(addr: Addr<Self>) -> Addr<Self> {
        System::with_current(|sys| {
            let mut sreg = SREG.lock();
            let reg = sreg
                .entry(sys.id())
                .or_insert_with(|| PatchedSystemRegistry::new(sys.arbiter().clone()));
            reg.registry.insert(TypeId::of::<Self>(), Box::new(addr.clone()));
            addr
        })
    }

    /// Get actor's address from system registry
    fn from_custom_registry() -> Addr<Self> {
        System::with_current(|sys| {
            let mut sreg = SREG.lock();
            let reg = sreg
                .entry(sys.id())
                .or_insert_with(|| PatchedSystemRegistry::new(sys.arbiter().clone()));

            if let Some(addr) = reg.registry.get(&TypeId::of::<Self>()) {
                if let Some(addr) = addr.downcast_ref::<Addr<Self>>() {
                    return addr.clone();
                }
            }

            panic!("Please start Actor before asking for it in registry!");
        })
    }
}



