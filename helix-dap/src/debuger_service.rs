use crate::{Client, StackFrame};
use std::collections::HashMap;

/// DebuggerService is a struct that manages and owns multiple debugger clients
/// This holds the responsibility of managing the lifecycle of each client
/// plus showing the heirarcihical nature betweeen them
pub struct DebuggerService {
    /// The top-level debuggers being held by this service
    debuggers: HashMap<usize, Client>,
    /// The active debugger client
    ///
    /// TODO: You can have multiple active debuggers, so the concept of a single active debugger
    /// may need to be changed
    current_debugger_id: Option<usize>,
    /// This is used to generate unique ids for each debugger
    pub counter: usize,
}

impl DebuggerService {
    /// Creates a new DebuggerService instance
    pub fn new() -> Self {
        Self {
            debuggers: HashMap::new(),
            current_debugger_id: None,
            counter: 0,
        }
    }

    pub fn add_debugger(&mut self, id: usize, client: Client) {
        self.debuggers.insert(id, client);
    }

    pub fn remove_debugger(&mut self, id: usize) {
        self.debuggers.remove(&id);
    }

    pub fn get_debugger(&self, id: usize) -> Option<&Client> {
        // Then check the children of each debugger
        for (_, debugger) in self.debuggers.iter() {
            if debugger.id() == id {
                return Some(debugger);
            }
            // Check if the debugger has a child with the given id
            if let Some(child_debugger) = debugger.get_child(id) {
                return Some(child_debugger);
            }
        }
        None
    }

    pub fn get_debugger_mut(&mut self, id: usize) -> Option<&mut Client> {
        // Then check the children of each debugger
        for (_, debugger) in self.debuggers.iter_mut() {
            if debugger.id() == id {
                return Some(debugger);
            }

            if let Some(child_debugger) = debugger.get_child_mut(id) {
                return Some(child_debugger);
            }
        }

        None
    }

    pub fn get_active_debugger(&self) -> Option<&Client> {
        self.current_debugger_id
            .and_then(|id| self.get_debugger(id))
    }

    pub fn get_active_debugger_mut(&mut self) -> Option<&mut Client> {
        self.current_debugger_id
            .and_then(|id| self.get_debugger_mut(id))
    }

    pub fn set_active_debugger(&mut self, id: usize) {
        if self.get_debugger(id).is_some() {
            self.current_debugger_id = Some(id);
        } else {
            self.current_debugger_id = None;
        }
    }

    pub fn unset_active_debugger(&mut self) {
        self.current_debugger_id = None;
    }

    pub fn current_stack_frame(&self) -> Option<&StackFrame> {
        self.get_active_debugger()
            .and_then(|debugger| debugger.current_stack_frame())
    }
}

impl Default for DebuggerService {
    fn default() -> Self {
        Self::new()
    }
}
