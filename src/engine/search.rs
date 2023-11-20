use super::Engine;

impl Engine {
    /// Start the search. Runs to infinity if `depth == None`,
    /// otherwise runs to depth `Some(depth)`.
    pub fn search(&self, depth: Option<u8>) {
        if let Some(depth) = depth {
            println!("Run to depth {depth}; not implemented");
        } else {
            println!("Run to depth infinite; not implemented");
        }
    }
}
