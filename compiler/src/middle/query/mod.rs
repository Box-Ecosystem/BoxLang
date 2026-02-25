//! Query System Module
//!
//! The query system provides incremental compilation support:
//! - Fine-grained dependency tracking
//! - Result caching
//! - Persistent storage

use std::collections::HashMap;
use std::hash::Hash;

/// Query ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryId(pub u64);

/// Query result storage
trait QueryStorage<Q: Query> {
    fn try_fetch(&self, key: &Q::Key) -> Option<Q::Value>;
    fn store(&mut self, key: Q::Key, value: Q::Value);
}

/// A query trait
pub trait Query: Sized {
    /// Input key type
    type Key: Clone + Hash + Eq + 'static;
    /// Output value type
    type Value: Clone + 'static;
    /// Query name
    const NAME: &'static str;

    /// Execute the query
    fn execute(db: &Database, key: Self::Key) -> Self::Value;
}

/// Query database
pub struct Database {
    /// Storage for query results
    storage: HashMap<String, Box<dyn std::any::Any>>,
}

impl Database {
    /// Create a new database
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    /// Execute a query
    pub fn query<Q: Query + 'static>(&mut self, key: Q::Key) -> Q::Value {
        // Check cache first
        if let Some(cached) = self.get_cached::<Q>(&key) {
            return cached;
        }

        // Execute query
        let value = Q::execute(self, key.clone());

        // Store result
        self.store_result::<Q>(key, value.clone());

        value
    }

    /// Get cached result
    fn get_cached<Q: Query>(&self, key: &Q::Key) -> Option<Q::Value> {
        self.storage
            .get(Q::NAME)
            .and_then(|s| s.downcast_ref::<HashMap<Q::Key, Q::Value>>())
            .and_then(|map| map.get(key).cloned())
    }

    /// Store query result
    fn store_result<Q: Query>(&mut self, key: Q::Key, value: Q::Value) {
        let entry = self
            .storage
            .entry(Q::NAME.to_string())
            .or_insert_with(|| Box::new(HashMap::<Q::Key, Q::Value>::new()));

        if let Some(map) = entry.downcast_mut::<HashMap<Q::Key, Q::Value>>() {
            map.insert(key, value);
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

/// Define a query
#[macro_export]
macro_rules! define_query {
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident($db:ident: $db_ty:ty, $key:ident: $key_ty:ty) -> $value_ty:ty $body:block
    ) => {
        $(#[$meta])*
        #[derive(Clone)]
        $vis struct $name;

        impl $crate::middle::query::Query for $name {
            type Key = $key_ty;
            type Value = $value_ty;
            const NAME: &'static str = stringify!($name);

            fn execute($db: &$db_ty, $key: Self::Key) -> Self::Value $body
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new();
        assert!(db.storage.is_empty());
    }
}
