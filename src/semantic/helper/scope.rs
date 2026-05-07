use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ScopeStack<T> {
    scopes: Vec<HashMap<String, T>>,
}

impl<T> Default for ScopeStack<T> {
    fn default() -> Self {
        Self { scopes: Vec::new() }
    }
}

impl<T: Clone> ScopeStack<T> {
    pub fn clear(&mut self) {
        self.scopes.clear();
    }

    pub fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    pub fn insert_current(&mut self, name: String, value: T) {
        self.scopes
            .last_mut()
            .expect("at least one scope should be present")
            .insert(name, value);
    }

    pub fn assign_at(&mut self, scope_index: usize, name: String, value: T) {
        self.scopes[scope_index].insert(name, value);
    }

    pub fn lookup(&self, name: &str) -> Option<T> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    pub fn find_scope_index(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.contains_key(name).then_some(idx))
    }

    pub fn lookup_with_index(&self, name: &str) -> Option<(usize, T)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.get(name).cloned().map(|value| (idx, value)))
    }

    pub fn contains_in_current(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }
}
