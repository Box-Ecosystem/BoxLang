//! Macro hygiene system for BoxLang
//!
//! Hygiene ensures that identifiers introduced by macros don't conflict
//! with identifiers in the surrounding code or other macro expansions.

use crate::ast::Ident;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for a syntax context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxContext(u64);

impl SyntaxContext {
    /// Create a new fresh syntax context
    pub fn fresh() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// The root context (no hygiene)
    pub fn root() -> Self {
        Self(0)
    }

    /// Check if this is the root context
    pub fn is_root(&self) -> bool {
        self.0 == 0
    }
}

impl Default for SyntaxContext {
    fn default() -> Self {
        Self::root()
    }
}

/// A hygienic identifier that carries syntax context
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HygienicIdent {
    /// The textual name
    pub name: Ident,
    /// The syntax context (for hygiene)
    pub context: SyntaxContext,
    /// Whether this identifier is opaque (cannot be matched from outside)
    pub opaque: bool,
}

impl HygienicIdent {
    /// Create a new hygienic identifier with fresh context
    pub fn fresh(name: Ident) -> Self {
        Self {
            name,
            context: SyntaxContext::fresh(),
            opaque: true,
        }
    }

    /// Create a new hygienic identifier with root context
    pub fn global(name: Ident) -> Self {
        Self {
            name,
            context: SyntaxContext::root(),
            opaque: false,
        }
    }

    /// Create with specific context
    pub fn with_context(name: Ident, context: SyntaxContext) -> Self {
        Self {
            name,
            context,
            opaque: false,
        }
    }

    /// Make this identifier opaque
    pub fn make_opaque(mut self) -> Self {
        self.opaque = true;
        self
    }

    /// Check if this identifier can match another
    ///
    /// Two identifiers match if:
    /// 1. They have the same name and both are global (root context), OR
    /// 2. They have the same name and same context, OR
    /// 3. One is opaque and matches the other's name and context
    pub fn can_match(&self, other: &HygienicIdent) -> bool {
        if self.name != other.name {
            return false;
        }

        // Both global
        if self.context.is_root() && other.context.is_root() {
            return true;
        }

        // Same context
        if self.context == other.context {
            return true;
        }

        // One is opaque and the other is from the same context
        if self.opaque && self.context == other.context {
            return true;
        }
        if other.opaque && other.context == self.context {
            return true;
        }

        false
    }

    /// Convert to string representation (for debugging)
    pub fn to_debug_string(&self) -> String {
        if self.context.is_root() {
            format!("{}#global", self.name)
        } else {
            format!(
                "{}#{}({})",
                self.name,
                self.context.0,
                if self.opaque { "opaque" } else { "transparent" }
            )
        }
    }
}

impl std::fmt::Display for HygienicIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Hygiene table that tracks identifier mappings across macro expansions
#[derive(Debug, Default)]
pub struct HygieneTable {
    /// Mapping from original identifiers to their hygienic versions
    mappings: HashMap<(Ident, SyntaxContext), HygienicIdent>,
    /// Parent-child context relationships
    context_hierarchy: HashMap<SyntaxContext, SyntaxContext>,
}

impl HygieneTable {
    /// Create a new hygiene table
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            context_hierarchy: HashMap::new(),
        }
    }

    /// Register a new context as a child of a parent context
    pub fn register_context(&mut self, child: SyntaxContext, parent: SyntaxContext) {
        self.context_hierarchy.insert(child, parent);
    }

    /// Create a fresh hygienic identifier for a name in a given context
    pub fn fresh_ident(&mut self, name: Ident, context: SyntaxContext) -> HygienicIdent {
        let key = (name.clone(), context);

        if let Some(existing) = self.mappings.get(&key) {
            existing.clone()
        } else {
            let fresh = HygienicIdent::fresh(name);
            self.mappings.insert(key, fresh.clone());
            fresh
        }
    }

    /// Get or create a hygienic identifier
    pub fn get_or_create(&mut self, name: Ident, context: SyntaxContext) -> HygienicIdent {
        let key = (name.clone(), context);

        self.mappings
            .entry(key)
            .or_insert_with(|| HygienicIdent::with_context(name, context))
            .clone()
    }

    /// Check if an identifier is visible from a given context
    pub fn is_visible(&self, ident: &HygienicIdent, from_context: SyntaxContext) -> bool {
        // Global identifiers are always visible
        if ident.context.is_root() {
            return true;
        }

        // Same context
        if ident.context == from_context {
            return true;
        }

        // Check if the identifier's context is an ancestor of the from_context
        let mut current = Some(from_context);
        while let Some(ctx) = current {
            if ctx == ident.context {
                return true;
            }
            current = self.context_hierarchy.get(&ctx).copied();
        }

        false
    }

    /// Rename an identifier to avoid conflicts
    pub fn rename(&mut self, ident: &HygienicIdent, new_name: Ident) -> HygienicIdent {
        let renamed = HygienicIdent {
            name: new_name,
            context: ident.context,
            opaque: ident.opaque,
        };

        let key = (ident.name.clone(), ident.context);
        self.mappings.insert(key, renamed.clone());

        renamed
    }
}

/// Hygiene resolver for macro expansion
pub struct HygieneResolver {
    table: HygieneTable,
    current_context: SyntaxContext,
}

impl HygieneResolver {
    /// Create a new hygiene resolver
    pub fn new() -> Self {
        Self {
            table: HygieneTable::new(),
            current_context: SyntaxContext::root(),
        }
    }

    /// Enter a new macro expansion context
    pub fn enter_macro(&mut self) -> SyntaxContext {
        let new_context = SyntaxContext::fresh();
        self.table
            .register_context(new_context, self.current_context);
        self.current_context = new_context;
        new_context
    }

    /// Exit the current macro expansion context
    pub fn exit_macro(&mut self) {
        if let Some(parent) = self
            .table
            .context_hierarchy
            .get(&self.current_context)
            .copied()
        {
            self.current_context = parent;
        }
    }

    /// Get the current context
    pub fn current_context(&self) -> SyntaxContext {
        self.current_context
    }

    /// Create a fresh identifier in the current context
    pub fn fresh_ident(&mut self, name: Ident) -> HygienicIdent {
        self.table.fresh_ident(name, self.current_context)
    }

    /// Resolve an identifier in the current context
    pub fn resolve(&mut self, name: Ident) -> HygienicIdent {
        self.table.get_or_create(name, self.current_context)
    }

    /// Check if two identifiers can match
    pub fn can_match(&self, a: &HygienicIdent, b: &HygienicIdent) -> bool {
        a.can_match(b)
    }
}

impl Default for HygieneResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_context_fresh() {
        let ctx1 = SyntaxContext::fresh();
        let ctx2 = SyntaxContext::fresh();

        assert_ne!(ctx1, ctx2);
        assert!(!ctx1.is_root());
        assert!(!ctx2.is_root());
    }

    #[test]
    fn test_hygienic_ident_match() {
        let global1 = HygienicIdent::global("x".into());
        let global2 = HygienicIdent::global("x".into());
        let fresh1 = HygienicIdent::fresh("x".into());
        let fresh2 = HygienicIdent::fresh("x".into());

        // Global identifiers with same name match
        assert!(global1.can_match(&global2));

        // Fresh identifiers don't match each other
        assert!(!fresh1.can_match(&fresh2));

        // Fresh doesn't match global
        assert!(!fresh1.can_match(&global1));
    }

    #[test]
    fn test_hygiene_resolver() {
        let mut resolver = HygieneResolver::new();

        // Create identifier in root context
        let root_ident = resolver.resolve("x".into());
        assert!(root_ident.context.is_root());

        // Enter macro context
        let macro_ctx = resolver.enter_macro();
        assert!(!macro_ctx.is_root());

        // Create identifier in macro context
        let macro_ident = resolver.resolve("x".into());
        assert_eq!(macro_ident.context, macro_ctx);

        // They should not match (different contexts)
        assert!(!resolver.can_match(&root_ident, &macro_ident));

        // Exit macro
        resolver.exit_macro();
        assert_eq!(resolver.current_context(), SyntaxContext::root());
    }

    #[test]
    fn test_hygiene_table_visibility() {
        let mut table = HygieneTable::new();

        let ctx1 = SyntaxContext::fresh();
        let ctx2 = SyntaxContext::fresh();

        table.register_context(ctx2, ctx1);

        let ident = HygienicIdent::with_context("x".into(), ctx1);

        // Visible from same context
        assert!(table.is_visible(&ident, ctx1));

        // Visible from child context
        assert!(table.is_visible(&ident, ctx2));

        // Not visible from unrelated context
        let ctx3 = SyntaxContext::fresh();
        assert!(!table.is_visible(&ident, ctx3));
    }
}
