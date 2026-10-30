// SPDX-License-Identifier: AGPL-3.0-or-later

//! Instance overrides: what a component's descendant looks like inside a given instance.
//!
//! An instance stores no children: it draws its component's. What differs per instance lives on
//! the instance, keyed by a `guidPath`, the override keys of the nested instances down to the
//! node, then the node's own:
//!
//! - `symbolData.symbolOverrides`: what the designer changed (text, visibility, fills…);
//! - `derivedSymbolData`: what Figma computed from it (sizes and positions after auto layout,
//!   geometry, and the glyph outlines of overridden text), so no layout or font is needed.
//!
//! [`Overrides`] follows a walk down nested instances and gives each node its fields as seen
//! through every instance above it, the outermost instance winning.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::figma::tree::{Node, NodeId, Tree, node_id};
use crate::kiwi::Value;

/// One instance's override entries, by `guidPath`.
type Entries = HashMap<Vec<NodeId>, Vec<Value>>;

/// The instances a walk is inside, outermost first.
#[derive(Default)]
pub struct Overrides {
    /// Per instance: its override entries by path, and where its paths start in `keys`.
    scopes: Vec<(Entries, usize)>,
    /// Override keys of the instances entered so far.
    keys: Vec<NodeId>,
    /// Keys to read other nodes' overrides under: see [`Overrides::alias`].
    aliases: HashMap<NodeId, NodeId>,
}

impl Overrides {
    /// Enters `instance` (already resolved with [`Overrides::resolve`]): its descendants are
    /// resolved through its overrides until [`Overrides::leave`].
    pub fn enter(&mut self, instance: &Node<'_>, resolved: &Value) {
        self.keys.push(key(instance));
        let mut entries = Entries::new();
        // What the designer set, then what Figma derived from it, which wins.
        let lists = [
            resolved
                .get("symbolData")
                .and_then(|s| s.get("symbolOverrides")),
            resolved.get("derivedSymbolData"),
        ];
        for entry in lists.into_iter().flatten().flat_map(Value::as_array) {
            let path: Vec<NodeId> = entry
                .get("guidPath")
                .and_then(|p| p.get("guids"))
                .map_or(&[][..], Value::as_array)
                .iter()
                .filter_map(node_id)
                .collect();
            if !path.is_empty() {
                entries.entry(path).or_default().push(entry.clone());
            }
        }
        self.scopes.push((entries, self.keys.len()));
    }

    /// Reads the overrides of `to`'s nodes under the keys of the `from` nodes that have the same
    /// names down from the root, as Figma keeps overrides when an instance changes variant.
    pub fn alias(&mut self, tree: &Tree<'_>, from: NodeId, to: NodeId) {
        let mut named = HashMap::new();
        name_paths(tree, from, &mut String::new(), &mut |path, node| {
            named.insert(path.to_owned(), key(node));
        });
        name_paths(tree, to, &mut String::new(), &mut |path, node| {
            if let Some(&key) = named.get(path) {
                self.aliases.insert(self::key(node), key);
            }
        });
    }

    pub fn leave(&mut self) {
        self.scopes.pop();
        self.keys.pop();
    }

    /// `node`'s fields inside the instances entered: its own, overridden by the innermost
    /// instance's entries, then by each outer one's.
    #[must_use]
    pub fn resolve<'v>(&self, node: &Node<'v>) -> Cow<'v, Value> {
        let own = key(node);
        let own = self.aliases.get(&own).copied().unwrap_or(own);
        let mut resolved: Option<Value> = None;
        for (entries, start) in self.scopes.iter().rev() {
            let mut path = self.keys[*start..].to_vec();
            path.push(own);
            for entry in entries.get(&path).into_iter().flatten() {
                merge(resolved.get_or_insert_with(|| node.value.clone()), entry);
            }
        }
        resolved.map_or(Cow::Borrowed(node.value), Cow::Owned)
    }
}

/// The key a node is known by in `guidPath`s: its `overrideKey`, else its id.
fn key(node: &Node<'_>) -> NodeId {
    node.value
        .get("overrideKey")
        .and_then(node_id)
        .unwrap_or(node.id)
}

/// Calls `visit` with each descendant of `root` and its path of names from it (`Label`,
/// `Row/Label`…).
fn name_paths(
    tree: &Tree<'_>,
    root: NodeId,
    path: &mut String,
    visit: &mut impl FnMut(&str, &Node<'_>),
) {
    for child in tree.children(root) {
        let length = path.len();
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(child.name);
        visit(path, child);
        name_paths(tree, child.id, path, visit);
        path.truncate(length);
    }
}

/// Copies every field of `entry` but its `guidPath` onto `target`.
fn merge(target: &mut Value, entry: &Value) {
    let (Value::Object(fields), Value::Object(changes)) = (target, entry) else {
        return;
    };
    for (name, value) in changes {
        if &**name == "guidPath" {
            continue;
        }
        match fields.iter_mut().find(|(n, _)| n == name) {
            Some((_, old)) => *old = value.clone(),
            None => fields.push((Arc::clone(name), value.clone())),
        }
    }
}
