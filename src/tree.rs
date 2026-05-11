//! URL frontier — Ruby CeWL `Tree` / `TreeNode` port.
//!
//! Key insight from the Ruby source: ALL nodes are stored as **direct children**
//! of the root (`@children << child` in every branch of `push`).  The
//! parent-child nesting is only used during `push` to find the referrer node and
//! compute the new depth; it is never used during `pop`.  This keeps `pop` O(n)
//! over a flat list, exactly as Ruby does.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Referrer URL (None for the seed).
    pub key: Option<String>,
    /// This node's URL (the page to fetch).
    pub value: String,
    pub depth: i32,
    pub visited: bool,
}

impl TreeNode {
    fn new(key: Option<String>, value: String, depth: i32) -> Self {
        Self {
            key,
            value,
            depth,
            visited: false,
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    pub data: TreeNode,
    /// All frontier nodes — always stored flat here (Ruby `@children << child`).
    children: Vec<Tree>,
    pub max_depth: i32,
    pub debug: bool,
}

impl Tree {
    pub fn new(key: Option<String>, value: String, depth: i32, debug: bool) -> Self {
        Self {
            data: TreeNode::new(key, value, depth),
            children: Vec::new(),
            max_depth: 2,
            debug,
        }
    }

    /// False while any unvisited node exists (checks root + all children).
    pub fn empty(&self) -> bool {
        if !self.data.visited {
            return false;
        }
        self.children.iter().all(|c| c.data.visited)
    }

    /// Pop the first unvisited node as `{referrer => url}`.
    pub fn pop(&mut self) -> Option<HashMap<Option<String>, String>> {
        if !self.data.visited {
            self.data.visited = true;
            let mut m = HashMap::new();
            m.insert(self.data.key.clone(), self.data.value.clone());
            return Some(m);
        }
        for node in &mut self.children {
            if !node.data.visited {
                node.data.visited = true;
                let mut m = HashMap::new();
                m.insert(node.data.key.clone(), node.data.value.clone());
                return Some(m);
            }
        }
        None
    }

    /// Push `{prior_url => next_url}`.
    ///
    /// Mirrors Ruby `Tree#push` exactly:
    /// - All new nodes are appended to `self.children` (the root list).
    /// - The children list is searched only to find the referrer's current depth.
    /// - Mailto links bypass the depth cap (Ruby's explicit hack).
    pub fn push(&mut self, prior_url: Option<String>, next_url: String) {
        if self.debug {
            eprintln!("Adding {:?} => {next_url}", prior_url);
        }

        if prior_url.is_none() {
            // Seed: just set the root node.
            self.data = TreeNode::new(None, next_url, 0);
            return;
        }

        if self.max_depth == 0 {
            return;
        }

        let prior = prior_url.as_ref().unwrap();
        let is_mailto = next_url.starts_with("mailto:");

        // Case 1: prior is the root node itself.
        if prior == &self.data.value {
            let child = Tree::new(
                Some(prior.clone()),
                next_url,
                self.data.depth + 1,
                self.debug,
            );
            self.children.push(child); // append to ROOT children
            return;
        }

        // Case 2: find the referrer among existing children to get its depth,
        // then append a new node to ROOT children (never to node.children).
        for node in &self.children {
            if node.data.value == *prior {
                if is_mailto || node.data.depth < self.max_depth {
                    let new_node = Tree::new(
                        Some(prior.clone()),
                        next_url,
                        node.data.depth + 1,
                        self.debug,
                    );
                    self.children.push(new_node); // always appended to ROOT
                }
                return; // stop after first match (Ruby iterates all but we break on depth here)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_pop() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 2;
        t.push(None, "http://a/".into());
        let m = t.pop().unwrap();
        assert_eq!(m.get(&None), Some(&"http://a/".to_string()));
    }

    #[test]
    fn depth1_child() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 2;
        t.push(None, "http://a/".into());
        t.pop();
        t.push(Some("http://a/".into()), "http://a/b".into());
        let m = t.pop().unwrap();
        assert_eq!(m.get(&Some("http://a/".into())), Some(&"http://a/b".into()));
    }

    #[test]
    fn depth0_blocks_children() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 0;
        t.push(None, "http://a/".into());
        t.pop();
        t.push(Some("http://a/".into()), "http://a/b".into());
        assert!(t.pop().is_none());
    }

    #[test]
    fn mailto_bypasses_depth() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 1;
        t.push(None, "http://a/".into());
        t.pop();
        t.push(Some("http://a/".into()), "http://a/b".into());
        t.pop(); // consume depth-1 node
                 // push mailto from the depth-1 node — should bypass cap
        t.push(Some("http://a/b".into()), "mailto:foo@bar.com".into());
        let m = t.pop();
        assert!(m.is_some());
    }

    #[test]
    fn empty_after_all_visited() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 1;
        t.push(None, "http://a/".into());
        assert!(!t.empty());
        t.pop();
        assert!(t.empty());
    }
}
