use std::num::NonZeroUsize;

use lru::LruCache;
use ulid::Ulid;

use crate::fingerprint::ReferenceSample;

pub trait Store {
    fn set_reference_sample(&mut self, id: Ulid, fingerprints: ReferenceSample);
    fn get_reference_sample(&mut self, id: &Ulid) -> Option<&ReferenceSample>;
}

pub struct PostgresStore {}

impl Store for PostgresStore {
    fn set_reference_sample(&mut self, _: Ulid, _: ReferenceSample) {
        unimplemented!()
    }

    fn get_reference_sample(&mut self, _: &Ulid) -> Option<&ReferenceSample> {
        unimplemented!()
    }
}

pub struct MemoryStore {
    cache: LruCache<Ulid, ReferenceSample>,
}

impl MemoryStore {
    pub fn new(cap: NonZeroUsize) -> Self {
        MemoryStore {
            cache: LruCache::new(cap),
        }
    }
}

impl Store for MemoryStore {
    fn set_reference_sample(&mut self, id: Ulid, reference_sample: ReferenceSample) {
        self.cache.put(id, reference_sample);
    }

    fn get_reference_sample(&mut self, id: &Ulid) -> Option<&ReferenceSample> {
        self.cache.get(id)
    }
}
