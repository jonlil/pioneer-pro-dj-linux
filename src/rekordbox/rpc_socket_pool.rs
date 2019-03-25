extern crate parking_lot;

use std::error;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use parking_lot::{Mutex, MutexGuard};

#[derive(Debug)]
pub struct Port {
    port: u16,
}

#[derive(Debug)]
struct PoolInternals {
    ports: Vec<Port>,
    last_error: Option<String>,
}

#[derive(Debug)]
struct SharedPool {
    internals: Mutex<PoolInternals>,
}

fn allocate_ports(
    shared: &Arc<SharedPool>,
    internals: &mut PoolInternals,
    ports: std::ops::RangeInclusive<u16>,
) {
    for p in ports {
        let port = Port {
            port: p,
        };
        internals.ports.push(port);
    }
}

#[derive(Debug)]
pub struct Pool(Arc<SharedPool>);

impl Clone for Pool {
    fn clone(&self) -> Pool {
        Pool(self.0.clone())
    }
}

impl Pool {
    pub fn new(port_range: std::ops::RangeInclusive<u16>) -> Result<Pool, &'static str>
    {
        let internals = PoolInternals {
            ports: Vec::with_capacity(25 as usize),
            last_error: None,
        };

        let shared = Arc::new(SharedPool {
            internals: Mutex::new(internals),
        });

        allocate_ports(&shared, &mut shared.internals.lock(), port_range);

        Ok(Pool(shared))
    }

    fn put_back(&self, mut port: u16) {
        let mut internals = self.0.internals.lock();
        internals.ports.push(Port {
            port: port,
        });
    }

    pub fn get(&self) -> Result<PooledPort, &'static str> {
        self.get_timeout()
    }

    pub fn get_timeout(&self) -> Result<PooledPort, &'static str> {
        let retries: usize = 24;
        let mut tries: usize = 0;

        loop {
            match self.try_get_inner(self.0.internals.lock()) {
                Ok(port) => return Ok(port),
                Err(i) => {},
            }

            if tries >= retries {
                return Err("Failed getting port from pool");
            }
        }
    }

    pub fn try_get(&self) -> Option<PooledPort> {
        unimplemented!()
    }

    fn try_get_inner<'a>(
        &'a self,
        mut internals: MutexGuard<'a, PoolInternals>,
    ) -> Result<PooledPort, &'static str> {
        if let Some(mut port) = internals.ports.pop() {
            drop(internals);

            return Ok(PooledPort {
                pool: self.clone(),
                port: Some(port.port),
            });
        } else {
            return Err("Unable to get port from pool");
        }
    }
}

#[derive(Debug)]
pub struct PooledPort {
    pool: Pool,
    port: Option<u16>,
}

impl PooledPort {
    pub fn get_port(&self) -> u16 {
        self.port.unwrap()
    }
}

impl Drop for PooledPort {
    fn drop(&mut self) {
        self.pool.put_back(self.port.take().unwrap());
    }
}

impl Deref for PooledPort {
    type Target = u16;
    fn deref(&self) -> &u16 {
        &self.port.as_ref().unwrap()
    }
}
