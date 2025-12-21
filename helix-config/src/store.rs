//! Global configuration storage and management.
//!
//! This module provides the central `ConfigStore` that owns all `OptionManager` instances
//! in the editor. It implements a hierarchical config system:
//!
//! ```text
//! Global Editor Config (root)
//!   ├── Language Configs (per language, inherits from global)
//!   │    └── Document Configs (per document, inherits from language)
//!   └── Language Server Configs (per LS, separate hierarchy)
//! ```
//!
//! # Architecture
//!
//! - **ConfigStore**: The central store that owns all config managers
//! - **OptionManager**: A scoped config manager with parent chain for inheritance
//! - **OptionRegistry**: Defines all available options and their validators
//!
//! # Thread Safety
//!
//! The ConfigStore is designed to be shared across threads via `Arc<ConfigStore>`.
//! Individual config scopes (OptionManagers) use RwLock<HashMap> internally for
//! concurrent access.
//!
//! # Usage
//!
//! ```ignore
//! // Initialize at startup
//! let mut registry = OptionRegistry::new();
//! init_config(&mut registry);
//!
//! // LSP registry starts empty - options are registered dynamically per language server
//! let lsp_registry = OptionRegistry::new();
//!
//! let store = ConfigStore::new(registry, lsp_registry);
//!
//! // Load configuration from TOML files
//! store.load_editor_config(Path::new("config.toml"))?;
//! store.load_languages_config(Path::new("languages.toml"))?;
//!
//! // Access editor config
//! let editor_config = store.editor();
//!
//! // Access language config
//! let rust_config = store.language("rust");
//!
//! // Create document config
//! let doc_config = store.create_document_config(doc_id, "rust");
//!
//! // Access language server config
//! let lsp_config = store.language_server("rust-analyzer");
//! ```

use std::sync::Arc;
use std::path::Path;
use std::marker::PhantomData;

use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::{OptionManager, OptionRegistry, Value, read_toml_config};
use crate::any::ConfigData;

/// Unique identifier for a document.
/// This is a stub type for now - will be replaced with the actual DocumentId type
/// from helix-view once integration happens.
pub type DocumentId = usize;

/// Identifies a value layer (storage for config values)
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct LayerId(u32);

/// Identifies a scope (layer + parent chain for inheritance)
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ScopeId(u32);

impl ScopeId {
    pub const NONE: Self = ScopeId(u32::MAX);

    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }
}

/// Identifies a language
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct LanguageId(u32);

impl LanguageId {
    pub const NONE: Self = LanguageId(u32::MAX);

    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }
}

/// Simple slot map implementation for arena-based storage
struct SlotMap<K, V> {
    slots: Vec<Option<V>>,
    free_list: Vec<u32>,
    _marker: PhantomData<K>,
}

impl<K, V> SlotMap<K, V> {
    fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
            _marker: PhantomData,
        }
    }

    fn insert(&mut self, value: V) -> K
    where
        K: From<u32>,
    {
        let index = if let Some(index) = self.free_list.pop() {
            self.slots[index as usize] = Some(value);
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Some(value));
            index
        };
        K::from(index)
    }

    fn remove(&mut self, key: K) -> Option<V>
    where
        K: Into<u32> + Copy,
    {
        let index = key.into() as usize;
        if index >= self.slots.len() {
            return None;
        }
        let value = self.slots[index].take()?;
        self.free_list.push(index as u32);
        Some(value)
    }

    fn get(&self, key: K) -> Option<&V>
    where
        K: Into<u32> + Copy,
    {
        let index = key.into() as usize;
        self.slots.get(index)?.as_ref()
    }

    fn get_mut(&mut self, key: K) -> Option<&mut V>
    where
        K: Into<u32> + Copy,
    {
        let index = key.into() as usize;
        self.slots.get_mut(index)?.as_mut()
    }
}

impl<K, V> std::fmt::Debug for SlotMap<K, V>
where
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotMap")
            .field("slots", &self.slots)
            .field("free_list", &self.free_list)
            .finish()
    }
}

// Implement From/Into for the ID types
impl From<u32> for LayerId {
    fn from(id: u32) -> Self {
        LayerId(id)
    }
}

impl From<LayerId> for u32 {
    fn from(id: LayerId) -> Self {
        id.0
    }
}

impl From<u32> for ScopeId {
    fn from(id: u32) -> Self {
        ScopeId(id)
    }
}

impl From<ScopeId> for u32 {
    fn from(id: ScopeId) -> Self {
        id.0
    }
}

impl From<u32> for LanguageId {
    fn from(id: u32) -> Self {
        LanguageId(id)
    }
}

impl From<LanguageId> for u32 {
    fn from(id: LanguageId) -> Self {
        id.0
    }
}

/// Layer stores the actual config values
struct Layer {
    values: HashMap<Arc<str>, ConfigData>,
}

impl Layer {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layer")
            .field("values", &self.values)
            .finish()
    }
}

/// ScopeNode defines the inheritance chain
struct ScopeNode {
    layer: LayerId,
    parent: ScopeId,  // NONE for root
}

impl std::fmt::Debug for ScopeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeNode")
            .field("layer", &self.layer)
            .field("parent", &self.parent)
            .finish()
    }
}

/// Language entry
struct LanguageEntry {
    name: Arc<str>,
    scope: ScopeId,
}

impl std::fmt::Debug for LanguageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageEntry")
            .field("name", &self.name)
            .field("scope", &self.scope)
            .finish()
    }
}

/// The central configuration store that owns all `OptionManager` instances.
///
/// This is the single source of truth for all configuration in the editor.
/// It manages:
/// - The global editor config
/// - Per-language configs (inheriting from global)
/// - Per-language-server configs (separate hierarchy with its own registry)
/// - Per-document configs (stored in a secondary map, inheriting from language)
///
/// # Thread Safety
///
/// ConfigStore uses interior mutability via RwLock for concurrent access.
/// It can be safely shared across threads using `Arc<ConfigStore>`.
#[derive(Debug)]
pub struct ConfigStore {
    /// The option registry for editor/language/document config
    registry: Arc<OptionRegistry>,

    /// The option registry for language server config (separate set of options)
    lsp_registry: Arc<OptionRegistry>,

    /// Arena storage for layers (config values)
    layers: RwLock<SlotMap<LayerId, RwLock<Layer>>>,

    /// Arena storage for scopes (layer + parent chain)
    scopes: RwLock<SlotMap<ScopeId, ScopeNode>>,

    /// Global scope (root of hierarchy)
    global_layer: LayerId,
    global_scope: ScopeId,

    /// Language management with Arc<str> names
    languages: RwLock<SlotMap<LanguageId, LanguageEntry>>,
    language_by_name: RwLock<HashMap<Arc<str>, LanguageId>>,

    /// Document layers (documents use LayerId, not ScopeId directly)
    documents: RwLock<HashMap<DocumentId, LayerId>>,

    /// LSPs keep string-based naming (not 1:1 mapping like languages)
    language_servers: RwLock<HashMap<Arc<str>, ScopeId>>,

    /// Legacy OptionManager for backwards compatibility
    /// The root editor configuration (global scope)
    editor: Arc<OptionManager>,

    /// Per-language OptionManagers for backwards compatibility
    legacy_languages: RwLock<HashMap<Arc<str>, Arc<OptionManager>>>,

    /// Per-language-server OptionManagers for backwards compatibility
    legacy_language_servers: RwLock<HashMap<Arc<str>, Arc<OptionManager>>>,

    /// Per-document OptionManagers for backwards compatibility
    legacy_documents: RwLock<HashMap<DocumentId, Arc<OptionManager>>>,
}

impl ConfigStore {
    /// Creates a new ConfigStore with the given registries.
    ///
    /// The editor registry must have all options registered before creating the store.
    /// Use `init_config()` to initialize the standard editor options.
    ///
    /// The LSP registry starts empty - options are registered dynamically per language
    /// server when configs are loaded via TOML or `get_or_create_language_server()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut registry = OptionRegistry::new();
    /// init_config(&mut registry);
    ///
    /// // LSP registry starts empty - populated dynamically per language server
    /// let lsp_registry = OptionRegistry::new();
    ///
    /// let store = ConfigStore::new(registry, lsp_registry);
    /// ```
    pub fn new(registry: OptionRegistry, lsp_registry: OptionRegistry) -> Self {
        let registry = Arc::new(registry);
        let lsp_registry = Arc::new(lsp_registry);
        let editor = registry.global_scope();

        // Create arena storage
        let mut layers = SlotMap::new();
        let mut scopes = SlotMap::new();

        // Create global layer with defaults from registry
        let global_layer = layers.insert(RwLock::new(Layer::new()));

        // Create global scope
        let global_scope = scopes.insert(ScopeNode {
            layer: global_layer,
            parent: ScopeId::NONE,
        });

        Self {
            registry,
            lsp_registry,
            layers: RwLock::new(layers),
            scopes: RwLock::new(scopes),
            global_layer,
            global_scope,
            languages: RwLock::new(SlotMap::new()),
            language_by_name: RwLock::new(HashMap::new()),
            documents: RwLock::new(HashMap::new()),
            language_servers: RwLock::new(HashMap::new()),
            editor,
            legacy_languages: RwLock::new(HashMap::new()),
            legacy_language_servers: RwLock::new(HashMap::new()),
            legacy_documents: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new empty layer
    pub fn create_layer(&self) -> LayerId {
        let mut layers = self.layers.write();
        layers.insert(RwLock::new(Layer::new()))
    }

    /// Create a scope with inheritance
    pub fn create_scope(&self, layer: LayerId, parent: ScopeId) -> ScopeId {
        let mut scopes = self.scopes.write();
        scopes.insert(ScopeNode { layer, parent })
    }

    /// Get or create a language, returning its LanguageId
    pub fn get_or_create_language(&self, name: impl Into<Arc<str>>) -> LanguageId {
        let name = name.into();

        // Try read lock first
        {
            let guard = self.language_by_name.read();
            if let Some(&id) = guard.get(&*name) {
                return id;
            }
        }

        // Need write locks to insert
        let mut name_map = self.language_by_name.write();
        let mut languages = self.languages.write();

        // Double-check in case another thread inserted while we waited
        if let Some(&id) = name_map.get(&*name) {
            return id;
        }

        // Create layer for this language
        let layer = {
            let mut layers = self.layers.write();
            layers.insert(RwLock::new(Layer::new()))
        };

        // Create scope that inherits from global
        let scope = {
            let mut scopes = self.scopes.write();
            scopes.insert(ScopeNode {
                layer,
                parent: self.global_scope,
            })
        };

        // Create language entry
        let id = languages.insert(LanguageEntry {
            name: name.clone(),
            scope,
        });

        name_map.insert(name, id);
        id
    }

    /// Lookup a language by name
    pub fn language_id(&self, name: &str) -> Option<LanguageId> {
        let guard = self.language_by_name.read();
        guard.get(name).copied()
    }

    /// Lookup a language name by ID
    pub fn language_name(&self, id: LanguageId) -> Option<Arc<str>> {
        let guard = self.languages.read();
        guard.get(id).map(|entry| entry.name.clone())
    }

    /// Get the scope for a language
    pub fn language_scope(&self, id: LanguageId) -> ScopeId {
        let guard = self.languages.read();
        guard.get(id).map(|entry| entry.scope).unwrap_or(ScopeId::NONE)
    }

    /// Create a document layer for a specific language
    pub fn create_document_layer(&self, id: DocumentId, _language: LanguageId) -> LayerId {
        // Create a new layer for this document
        let layer = {
            let mut layers = self.layers.write();
            layers.insert(RwLock::new(Layer::new()))
        };

        // Store the document layer
        let mut documents = self.documents.write();
        documents.insert(id, layer);

        layer
    }

    /// Get the scope for a document+language combination
    pub fn document_scope(&self, doc_id: DocumentId, language: LanguageId) -> ScopeId {
        // Get document layer
        let layer = {
            let documents = self.documents.read();
            *documents.get(&doc_id).unwrap_or(&self.global_layer)
        };

        // Get language scope as parent
        let parent = self.language_scope(language);

        // Create a scope that combines document layer with language parent
        let mut scopes = self.scopes.write();
        scopes.insert(ScopeNode { layer, parent })
    }

    /// Get config data by walking the parent chain
    pub fn get_data(&self, scope: ScopeId, option: &str) -> Option<crate::Guard<'_, ConfigData>> {
        use parking_lot::RwLockReadGuard;

        let mut current_scope = scope;
        loop {
            if current_scope.is_none() {
                return None;
            }

            // Get the scope node
            let (layer_id, parent) = {
                let scopes = self.scopes.read();
                let node = scopes.get(current_scope)?;
                (node.layer, node.parent)
            };

            // Try to get the value from this layer
            let layers = self.layers.read();
            if let Some(layer_lock) = layers.get(layer_id) {
                let layer_guard = layer_lock.read();
                if let Ok(mapped) = RwLockReadGuard::try_map(layer_guard, |layer| {
                    layer.values.get(option)
                }) {
                    // We found the value - but we need to return it while keeping layers lock alive
                    // This is tricky, so for now we'll just check existence
                    drop(mapped);
                    // Return None for now - this needs proper implementation
                    // The issue is we need to keep both locks alive
                }
            }

            // Move to parent
            current_scope = parent;
        }
    }

    /// Returns a reference to the editor option registry.
    ///
    /// The registry is needed for operations like `set()` that need to validate
    /// option names and values for editor/language/document config.
    pub fn registry(&self) -> &Arc<OptionRegistry> {
        &self.registry
    }

    /// Returns a reference to the language server option registry.
    ///
    /// The LSP registry is needed for operations like `set()` on language server
    /// configs. Language servers have a separate set of options from the editor.
    pub fn lsp_registry(&self) -> &Arc<OptionRegistry> {
        &self.lsp_registry
    }

    /// Returns the global editor configuration.
    ///
    /// This is the root of the config hierarchy. All language and document configs
    /// inherit from this.
    pub fn editor(&self) -> &Arc<OptionManager> {
        &self.editor
    }

    /// Returns the configuration for a specific language, if it exists.
    ///
    /// Language configs inherit from the global editor config.
    /// Returns `None` if no config has been created for this language yet.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(rust_config) = store.language("rust") {
    ///     // Use rust-specific config
    /// }
    /// ```
    pub fn language(&self, name: &str) -> Option<Arc<OptionManager>> {
        let guard = self.legacy_languages.read();
        guard.get(name).map(Arc::clone)
    }

    /// Creates or retrieves a language configuration (legacy OptionManager version).
    ///
    /// If the language config doesn't exist, it creates a new one that inherits
    /// from the global editor config.
    ///
    /// This is typically called when loading languages.toml or when a buffer
    /// with a new language is opened.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rust_config = store.get_or_create_language_legacy("rust");
    /// ```
    fn get_or_create_language_legacy(&self, name: impl Into<Arc<str>>) -> Arc<OptionManager> {
        let name = name.into();

        // Try read lock first
        {
            let guard = self.legacy_languages.read();
            if let Some(config) = guard.get(&*name) {
                return Arc::clone(config);
            }
        }

        // Need write lock to insert
        let mut guard = self.legacy_languages.write();

        // Double-check in case another thread inserted while we waited
        if let Some(config) = guard.get(&*name) {
            return Arc::clone(config);
        }

        // Create a new language scope that inherits from global
        let config = Arc::new(self.editor.create_scope());
        guard.insert(name.clone(), Arc::clone(&config));

        // Also create in new arena system
        self.get_or_create_language(name);

        config
    }

    /// Returns all registered language names.
    ///
    /// This is useful for iterating over all language configs, e.g., when
    /// applying a global config change that affects all languages.
    pub fn language_names(&self) -> Vec<Arc<str>> {
        let guard = self.language_by_name.read();
        guard.keys().cloned().collect()
    }

    /// Returns the configuration for a specific language server, if it exists.
    ///
    /// Language server configs have their own hierarchy separate from the main
    /// editor config (they don't inherit from the global editor config).
    ///
    /// Returns `None` if no config has been created for this language server yet.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(lsp_config) = store.language_server("rust-analyzer") {
    ///     let timeout = lsp_config.get::<Duration>("timeout");
    /// }
    /// ```
    pub fn language_server(&self, name: &str) -> Option<Arc<OptionManager>> {
        let guard = self.legacy_language_servers.read();
        guard.get(name).map(Arc::clone)
    }

    /// Creates or retrieves a language server configuration.
    ///
    /// If the language server config doesn't exist, it creates a new one with
    /// defaults from the registry.
    ///
    /// Language server configs use a separate hierarchy and are initialized with
    /// `init_language_server_config()` which registers LSP-specific options.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let lsp_config = store.get_or_create_language_server("rust-analyzer");
    /// ```
    pub fn get_or_create_language_server(&self, name: impl Into<Arc<str>>) -> Arc<OptionManager> {
        let name = name.into();

        // Try read lock first
        {
            let guard = self.legacy_language_servers.read();
            if let Some(config) = guard.get(&*name) {
                return Arc::clone(config);
            }
        }

        // Need write lock to insert
        let mut guard = self.legacy_language_servers.write();

        // Double-check in case another thread inserted while we waited
        if let Some(config) = guard.get(&*name) {
            return Arc::clone(config);
        }

        // Language server configs use the LSP registry (separate from editor config)
        // They don't inherit from the global editor config
        let config = Arc::new(self.lsp_registry.global_scope().create_scope());
        guard.insert(name.clone(), Arc::clone(&config));

        // Also create in new arena system
        // For LSPs, we just create a scope in the language_servers map
        let layer = {
            let mut layers = self.layers.write();
            layers.insert(RwLock::new(Layer::new()))
        };
        let scope = {
            let mut scopes = self.scopes.write();
            scopes.insert(ScopeNode {
                layer,
                parent: ScopeId::NONE, // LSP configs don't inherit from global
            })
        };
        self.language_servers.write().insert(name, scope);

        config
    }

    /// Returns all registered language server names.
    pub fn language_server_names(&self) -> Vec<Arc<str>> {
        let guard = self.language_servers.read();
        guard.keys().cloned().collect()
    }

    /// Returns the configuration for a specific document, if it exists.
    ///
    /// Document configs inherit from their language config (which inherits from global).
    /// This is a secondary storage - documents will clone the Arc for internal use.
    ///
    /// Returns `None` if no config has been created for this document yet.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(doc_config) = store.document(doc_id) {
    ///     // Use document-specific config
    /// }
    /// ```
    pub fn document(&self, id: DocumentId) -> Option<Arc<OptionManager>> {
        let guard = self.legacy_documents.read();
        guard.get(&id).map(Arc::clone)
    }

    /// Creates a new document configuration that inherits from the given language.
    ///
    /// This should be called when a new document is created. The returned config
    /// should be cloned into the Document struct.
    ///
    /// The document config will inherit from the language config, which inherits
    /// from the global config. This creates a three-level hierarchy:
    /// Global -> Language -> Document
    ///
    /// # Example
    ///
    /// ```ignore
    /// let doc_config = store.create_document_config(doc_id, "rust");
    /// // Clone into Document
    /// document.config = Arc::clone(&doc_config);
    /// ```
    pub fn create_document_config(
        &self,
        id: DocumentId,
        language: &str,
    ) -> Arc<OptionManager> {
        let language_config = self.get_or_create_language_legacy(language);

        let doc_config = Arc::new(language_config.create_scope());

        let mut guard = self.legacy_documents.write();
        guard.insert(id, Arc::clone(&doc_config));

        // Also create in new arena system
        if let Some(lang_id) = self.language_id(language) {
            self.create_document_layer(id, lang_id);
        }

        doc_config
    }

    /// Removes a document configuration from the store.
    ///
    /// This should be called when a document is closed to free up resources.
    /// The Document itself may still hold an Arc to the config until it's dropped.
    ///
    /// # Example
    ///
    /// ```ignore
    /// store.remove_document(doc_id);
    /// ```
    pub fn remove_document(&self, id: DocumentId) -> Option<Arc<OptionManager>> {
        let mut guard = self.legacy_documents.write();
        let result = guard.remove(&id);

        // Also remove from new arena system
        self.documents.write().remove(&id);

        result
    }

    /// Updates a document's language, creating a new config scope.
    ///
    /// This is called when a document's language changes (e.g., when the user
    /// manually sets the language with `:set-language`).
    ///
    /// Returns the new config that should be used for the document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let new_config = store.update_document_language(doc_id, "python");
    /// document.config = new_config;
    /// ```
    pub fn update_document_language(
        &self,
        id: DocumentId,
        new_language: &str,
    ) -> Arc<OptionManager> {
        // Remove old config
        self.remove_document(id);

        // Create new config with new language
        self.create_document_config(id, new_language)
    }

    /// Resolves a config by name for use with `:set` commands.
    ///
    /// Supports the following name patterns:
    /// - `"editor"` or empty - returns the global editor config
    /// - `"language:rust"` - returns the rust language config
    /// - `"lsp:rust-analyzer"` - returns the rust-analyzer language server config
    /// - `"document:123"` - returns the config for document ID 123
    ///
    /// This enables commands like:
    /// - `:set editor indent.width 4`
    /// - `:set language:rust indent.width 2`
    /// - `:set lsp:rust-analyzer timeout 5000`
    ///
    /// Returns `None` if the named config doesn't exist.
    pub fn resolve(&self, name: &str) -> Option<Arc<OptionManager>> {
        if name.is_empty() || name == "editor" {
            return Some(Arc::clone(&self.editor));
        }

        if let Some(lang_name) = name.strip_prefix("language:") {
            return self.language(lang_name);
        }

        if let Some(lsp_name) = name.strip_prefix("lsp:") {
            return self.language_server(lsp_name);
        }

        if let Some(doc_id_str) = name.strip_prefix("document:") {
            if let Ok(doc_id) = doc_id_str.parse::<DocumentId>() {
                return self.document(doc_id);
            }
        }

        None
    }

    /// Resolves a config by name, creating it if it doesn't exist.
    ///
    /// Similar to `resolve()`, but will create language or language server
    /// configs if they don't exist. Document configs cannot be auto-created
    /// as they require a language name.
    ///
    /// Returns `None` only for invalid names or non-existent document IDs.
    pub fn resolve_or_create(&self, name: &str) -> Option<Arc<OptionManager>> {
        if name.is_empty() || name == "editor" {
            return Some(Arc::clone(&self.editor));
        }

        if let Some(lang_name) = name.strip_prefix("language:") {
            return Some(self.get_or_create_language_legacy(lang_name));
        }

        if let Some(lsp_name) = name.strip_prefix("lsp:") {
            return Some(self.get_or_create_language_server(lsp_name));
        }

        if let Some(doc_id_str) = name.strip_prefix("document:") {
            if let Ok(doc_id) = doc_id_str.parse::<DocumentId>() {
                return self.document(doc_id);
            }
        }

        None
    }

    /// Clear all editor configuration values, resetting to defaults.
    ///
    /// This clears the global layer, effectively resetting all editor config
    /// options to their default values (as specified in the OptionRegistry).
    ///
    /// This is useful when reloading config files, to ensure that options that
    /// were previously set but are no longer in the config file are reset to defaults.
    pub fn clear_editor_config(&self) {
        let layers = self.layers.read();
        if let Some(layer) = layers.get(self.global_layer) {
            layer.write().values.clear();
        }
    }

    /// Load editor configuration from a TOML file (typically config.toml).
    ///
    /// This loads the global editor configuration. The TOML file can have config
    /// options at the root level or under an `[editor]` section.
    ///
    /// # Example TOML structure
    ///
    /// ```toml
    /// # Root level config
    /// scrolloff = 10
    ///
    /// [editor]
    /// # Or under [editor] section
    /// line-number = "relative"
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is invalid
    /// - Any config values are invalid
    ///
    /// # Example
    ///
    /// ```ignore
    /// store.load_editor_config(Path::new("/path/to/config.toml"))?;
    /// ```
    pub fn load_editor_config(&self, path: &Path) -> anyhow::Result<()> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", path.display(), e))?;

        let toml_value: toml::Value = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file {}: {}", path.display(), e))?;

        // Convert toml::Value to our Value type
        let value: Value = toml_value.try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert config: {}", e))?;

        let Value::Map(map_box) = value else {
            anyhow::bail!("Config file must be a TOML table");
        };
        let mut map = *map_box;

        // If there's an [editor] section, merge it into root level
        // This allows both flat config and [editor] section to work
        if let Some(Value::Map(editor_map)) = map.shift_remove("editor") {
            // Merge editor section values into the root map
            for (key, val) in editor_map.into_iter() {
                map.entry(key).or_insert(val);
            }
        }

        // Apply all config values to the editor scope
        read_toml_config(map, &self.editor, &self.registry)?;

        Ok(())
    }

    /// Load language configurations from a TOML file (typically languages.toml).
    ///
    /// This loads per-language and per-language-server configurations. The TOML
    /// file should have:
    /// - `[[language]]` array for language configs
    /// - `[language-server.NAME]` sections for language server configs
    ///
    /// Note: Language-specific fields like `name`, and language-server fields like
    /// `command`, `args`, etc. are not config options and will be filtered out
    /// automatically. Only actual configuration options are applied.
    ///
    /// # Example TOML structure
    ///
    /// ```toml
    /// [[language]]
    /// name = "rust"
    /// indent = { tab-width = 4, unit = "    " }
    ///
    /// [language-server.rust-analyzer]
    /// command = "rust-analyzer"  # This will be filtered out
    /// args = ["--log-file", "/tmp/ra.log"]  # This will be filtered out
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is invalid
    /// - Any config values are invalid
    /// - A language entry is missing a "name" field
    ///
    /// # Example
    ///
    /// ```ignore
    /// store.load_languages_config(Path::new("/path/to/languages.toml"))?;
    /// ```
    pub fn load_languages_config(&self, path: &Path) -> anyhow::Result<()> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read languages config file {}: {}", path.display(), e))?;

        let toml_value: toml::Value = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Failed to parse languages config file {}: {}", path.display(), e))?;

        // Convert toml::Value to our Value type
        let value: Value = toml_value.try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert languages config: {}", e))?;

        let Value::Map(map_box) = value else {
            anyhow::bail!("Languages config file must be a TOML table");
        };
        let map = *map_box;

        // Process [[language]] array
        if let Some(Value::List(languages)) = map.get("language") {
            for lang_value in languages {
                let Value::Map(lang_map_box) = lang_value else {
                    anyhow::bail!("Each [[language]] entry must be a table");
                };
                let lang_map = lang_map_box.as_ref();

                // Extract the language name
                let Some(Value::String(name)) = lang_map.get("name") else {
                    anyhow::bail!("Each [[language]] entry must have a 'name' field");
                };

                // Get or create the language config (legacy)
                let lang_config = self.get_or_create_language_legacy(name.to_string());

                // Apply all config values for this language
                // Filter out the "name" field as it's not a config option
                let mut filtered_map = lang_map.clone();
                filtered_map.shift_remove("name");
                read_toml_config(filtered_map, &lang_config, &self.registry)?;
            }
        }

        // Process [language-server.NAME] sections
        if let Some(Value::Map(lsp_servers_box)) = map.get("language-server") {
            for (server_name, server_value) in lsp_servers_box.iter() {
                let Value::Map(server_map_box) = server_value else {
                    anyhow::bail!("Language server config for '{}' must be a table", server_name);
                };
                let server_map = server_map_box.as_ref();

                // Get or create the language server config
                let lsp_config = self.get_or_create_language_server(server_name.to_string());

                // Apply all config values for this language server
                // Filter out language server specific fields that aren't config options
                // These include: command, args, timeout, config, environment, etc.
                let mut filtered_map = server_map.clone();
                // Common language server fields to filter
                for key in ["command", "args", "timeout", "config", "environment"].iter() {
                    filtered_map.shift_remove(*key);
                }
                // Use lsp_registry for language server configs
                read_toml_config(filtered_map, &lsp_config, &self.lsp_registry)?;
            }
        }

        Ok(())
    }

    /// Load editor configuration from a TOML file if it exists.
    ///
    /// This is a convenience wrapper around `load_editor_config` that returns
    /// `Ok(())` if the file doesn't exist, rather than an error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Won't error if file doesn't exist
    /// store.load_editor_config_if_exists(Path::new("/path/to/config.toml"))?;
    /// ```
    pub fn load_editor_config_if_exists(&self, path: &Path) -> anyhow::Result<()> {
        if path.exists() {
            self.load_editor_config(path)
        } else {
            Ok(())
        }
    }

    /// Load language configurations from a TOML file if it exists.
    ///
    /// This is a convenience wrapper around `load_languages_config` that returns
    /// `Ok(())` if the file doesn't exist, rather than an error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Won't error if file doesn't exist
    /// store.load_languages_config_if_exists(Path::new("/path/to/languages.toml"))?;
    /// ```
    pub fn load_languages_config_if_exists(&self, path: &Path) -> anyhow::Result<()> {
        if path.exists() {
            self.load_languages_config(path)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_store_creation() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        assert!(Arc::ptr_eq(store.editor(), store.editor()));
    }

    #[test]
    fn test_language_config_creation() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        assert!(store.language("rust").is_none());

        let rust_config = store.get_or_create_language_legacy("rust");
        assert!(store.language("rust").is_some());

        let rust_config2 = store.get_or_create_language_legacy("rust");
        assert!(Arc::ptr_eq(&rust_config, &rust_config2));
    }

    #[test]
    fn test_language_server_config_creation() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        assert!(store.language_server("rust-analyzer").is_none());

        let lsp_config = store.get_or_create_language_server("rust-analyzer");
        assert!(store.language_server("rust-analyzer").is_some());

        let lsp_config2 = store.get_or_create_language_server("rust-analyzer");
        assert!(Arc::ptr_eq(&lsp_config, &lsp_config2));
    }

    #[test]
    fn test_document_config_lifecycle() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        let doc_id = 1;
        assert!(store.document(doc_id).is_none());

        let doc_config = store.create_document_config(doc_id, "rust");
        assert!(store.document(doc_id).is_some());

        let doc_config2 = store.document(doc_id).unwrap();
        assert!(Arc::ptr_eq(&doc_config, &doc_config2));

        store.remove_document(doc_id);
        assert!(store.document(doc_id).is_none());
    }

    #[test]
    fn test_document_language_update() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        let doc_id = 1;
        let rust_config = store.create_document_config(doc_id, "rust");

        let python_config = store.update_document_language(doc_id, "python");

        // Should be a different config
        assert!(!Arc::ptr_eq(&rust_config, &python_config));

        // Document should have the new config
        let doc_config = store.document(doc_id).unwrap();
        assert!(Arc::ptr_eq(&python_config, &doc_config));
    }

    #[test]
    fn test_resolve_editor() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        let editor1 = store.resolve("editor").unwrap();
        let editor2 = store.resolve("").unwrap();

        assert!(Arc::ptr_eq(&editor1, store.editor()));
        assert!(Arc::ptr_eq(&editor2, store.editor()));
    }

    #[test]
    fn test_resolve_language() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        store.get_or_create_language_legacy("rust");

        let rust_config = store.resolve("language:rust").unwrap();
        assert!(Arc::ptr_eq(&rust_config, &store.language("rust").unwrap()));

        assert!(store.resolve("language:nonexistent").is_none());
    }

    #[test]
    fn test_resolve_language_server() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        store.get_or_create_language_server("rust-analyzer");

        let lsp_config = store.resolve("lsp:rust-analyzer").unwrap();
        assert!(Arc::ptr_eq(
            &lsp_config,
            &store.language_server("rust-analyzer").unwrap()
        ));

        assert!(store.resolve("lsp:nonexistent").is_none());
    }

    #[test]
    fn test_resolve_document() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        let doc_id = 42;
        store.create_document_config(doc_id, "rust");

        let doc_config = store.resolve("document:42").unwrap();
        assert!(Arc::ptr_eq(&doc_config, &store.document(doc_id).unwrap()));

        assert!(store.resolve("document:999").is_none());
        assert!(store.resolve("document:invalid").is_none());
    }

    #[test]
    fn test_resolve_or_create() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        // Language should be created
        assert!(store.language("python").is_none());
        let python_config = store.resolve_or_create("language:python").unwrap();
        assert!(store.language("python").is_some());
        assert!(Arc::ptr_eq(&python_config, &store.language("python").unwrap()));

        // LSP should be created
        assert!(store.language_server("pylsp").is_none());
        let lsp_config = store.resolve_or_create("lsp:pylsp").unwrap();
        assert!(store.language_server("pylsp").is_some());
        assert!(Arc::ptr_eq(&lsp_config, &store.language_server("pylsp").unwrap()));

        // Document should NOT be created (requires language)
        assert!(store.resolve_or_create("document:999").is_none());
    }

    #[test]
    fn test_language_names() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        store.get_or_create_language("rust");
        store.get_or_create_language("python");
        store.get_or_create_language("javascript");

        let mut names = store.language_names();
        names.sort();

        assert_eq!(names.len(), 3);
        assert!(names.contains(&Arc::from("rust")));
        assert!(names.contains(&Arc::from("python")));
        assert!(names.contains(&Arc::from("javascript")));
    }

    #[test]
    fn test_language_server_names() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        store.get_or_create_language_server("rust-analyzer");
        store.get_or_create_language_server("pylsp");

        let mut names = store.language_server_names();
        names.sort();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&Arc::from("rust-analyzer")));
        assert!(names.contains(&Arc::from("pylsp")));
    }

    #[test]
    fn test_load_editor_config_from_toml() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut registry = OptionRegistry::new();
        crate::init_config(&mut registry);
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        // Create a temporary TOML config file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[editor]").unwrap();
        writeln!(temp_file, "scrolloff = 10").unwrap();
        writeln!(temp_file, "mouse = false").unwrap();
        temp_file.flush().unwrap();

        // Load the config
        store.load_editor_config(temp_file.path()).unwrap();

        // Verify the config was loaded
        // Note: We can't directly test the values without the trait accessors,
        // but we can verify it doesn't error
    }

    #[test]
    fn test_load_languages_config_from_toml() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut registry = OptionRegistry::new();
        crate::init_config(&mut registry);
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        // Create a temporary TOML config file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[[language]]").unwrap();
        writeln!(temp_file, "name = \"rust\"").unwrap();
        writeln!(temp_file, "").unwrap();
        writeln!(temp_file, "[[language]]").unwrap();
        writeln!(temp_file, "name = \"python\"").unwrap();
        writeln!(temp_file, "").unwrap();
        writeln!(temp_file, "[language-server.rust-analyzer]").unwrap();
        writeln!(temp_file, "command = \"rust-analyzer\"").unwrap();
        temp_file.flush().unwrap();

        // Load the config
        store.load_languages_config(temp_file.path()).unwrap();

        // Verify languages were created
        assert!(store.language("rust").is_some());
        assert!(store.language("python").is_some());
        assert!(store.language_server("rust-analyzer").is_some());
    }

    #[test]
    fn test_load_config_if_exists() {
        use std::path::PathBuf;

        let mut registry = OptionRegistry::new();
        crate::init_config(&mut registry);
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        // Non-existent file should not error
        let non_existent = PathBuf::from("/tmp/non_existent_config_file_12345.toml");
        assert!(store.load_editor_config_if_exists(&non_existent).is_ok());
        assert!(store.load_languages_config_if_exists(&non_existent).is_ok());
    }

    #[test]
    fn test_arena_language_creation() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        // Create a language using the arena-based method
        let rust_id = store.get_or_create_language("rust");
        assert!(!rust_id.is_none());

        // Should be able to look it up by name
        assert_eq!(store.language_id("rust"), Some(rust_id));

        // Name should match
        assert_eq!(store.language_name(rust_id).as_deref(), Some("rust"));

        // Creating again should return the same ID
        let rust_id2 = store.get_or_create_language("rust");
        assert_eq!(rust_id, rust_id2);

        // Scope should not be NONE
        let scope = store.language_scope(rust_id);
        assert!(!scope.is_none());
    }

    #[test]
    fn test_arena_document_layer() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        let doc_id = 42;
        let rust_id = store.get_or_create_language("rust");

        // Create a document layer
        let layer = store.create_document_layer(doc_id, rust_id);

        // Should be able to get document scope
        let scope = store.document_scope(doc_id, rust_id);
        assert!(!scope.is_none());
    }

    #[test]
    fn test_arena_scope_creation() {
        let registry = OptionRegistry::new();
        let lsp_registry = OptionRegistry::new();
        let store = ConfigStore::new(registry, lsp_registry);

        // Create a layer
        let layer = store.create_layer();

        // Create a scope with the global scope as parent
        let scope = store.create_scope(layer, store.global_scope);
        assert!(!scope.is_none());

        // Create another scope with the first as parent
        let child_scope = store.create_scope(layer, scope);
        assert!(!child_scope.is_none());
        assert_ne!(scope, child_scope);
    }
}
