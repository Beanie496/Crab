use super::Engine;

impl Engine {
    /// Start the search. Runs to infinity if `depth == None`,
    /// otherwise runs to depth `Some(depth)`.
    #[inline]
    pub fn search(&self, depth: Option<u8>) {
        depth.map_or_else(
            || {
                println!("Run to depth infinite; not implemented");
            },
            |depth| {
                println!("Run to depth {depth}; not implemented");
            },
        );
    }
}
