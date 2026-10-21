// SPDX-License-Identifier: AGPL-3.0-or-later

//! The node tree of a decoded `.fig`: nodes by id, children in layer order, blobs and images.

use std::collections::HashMap;
use std::fmt;

use crate::figma::FigFile;
use crate::kiwi::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub session: u32,
    pub local: u32,
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.session, self.local)
    }
}

impl std::str::FromStr for NodeId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (session, local) = s
            .split_once(':')
            .ok_or_else(|| format!("{s:?} is not a node id like 12:34"))?;
        Ok(NodeId {
            session: session.parse().map_err(|e| format!("{s:?}: {e}"))?,
            local: local.parse().map_err(|e| format!("{s:?}: {e}"))?,
        })
    }
}

/// One entry of `nodeChanges`; `value` holds every other field.
pub struct Node<'a> {
    pub id: NodeId,
    pub kind: &'a str,
    pub name: &'a str,
    pub value: &'a Value,
}

pub struct Tree<'a> {
    file: &'a FigFile,
    nodes: HashMap<NodeId, Node<'a>>,
    children: HashMap<NodeId, Vec<NodeId>>,
}

impl<'a> Tree<'a> {
    #[must_use]
    pub fn new(file: &'a FigFile) -> Self {
        let mut nodes = HashMap::new();
        let mut children: HashMap<NodeId, Vec<(&str, NodeId)>> = HashMap::new();
        for value in file
            .message
            .get("nodeChanges")
            .map_or(&[][..], Value::as_array)
        {
            if value.get("phase").and_then(Value::as_str) == Some("REMOVED") {
                continue;
            }
            let Some(id) = value.get("guid").and_then(node_id) else {
                continue;
            };
            if let Some(parent) = value.get("parentIndex") {
                let position = parent.get("position").and_then(Value::as_str).unwrap_or("");
                if let Some(parent_id) = parent.get("guid").and_then(node_id) {
                    children.entry(parent_id).or_default().push((position, id));
                }
            }
            let kind = value.get("type").and_then(Value::as_str).unwrap_or("NONE");
            let name = value.get("name").and_then(Value::as_str).unwrap_or("");
            nodes.insert(
                id,
                Node {
                    id,
                    kind,
                    name,
                    value,
                },
            );
        }
        let children = children
            .into_iter()
            .map(|(parent, mut list)| {
                // Figma orders siblings by comparing their position keys as strings.
                list.sort_by(|a, b| a.0.cmp(b.0));
                (parent, list.into_iter().map(|(_, id)| id).collect())
            })
            .collect();
        Self {
            file,
            nodes,
            children,
        }
    }

    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Node<'a>> {
        self.nodes.get(&id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node<'a>> {
        self.nodes.values()
    }

    pub fn children(&self, id: NodeId) -> impl Iterator<Item = &Node<'a>> {
        self.children
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|child| self.nodes.get(child))
    }

    /// Bytes of `Message.blobs[index]`: path commands, vector networks…
    #[must_use]
    pub fn blob(&self, index: u32) -> Option<&'a [u8]> {
        match self
            .file
            .message
            .get("blobs")?
            .as_array()
            .get(index as usize)?
            .get("bytes")?
        {
            Value::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// An `images/<sha1>` entry of the archive, by the 20-byte hash stored in an image paint.
    #[must_use]
    pub fn image(&self, hash: &[u8]) -> Option<&'a [u8]> {
        self.file
            .images
            .get(&helpers4::hex::encode(hash))
            .map(Vec::as_slice)
    }
}

/// `None` for a missing GUID and for `0xFFFFFFFF:0xFFFFFFFF`, Figma's "no node" sentinel.
#[must_use]
pub fn node_id(guid: &Value) -> Option<NodeId> {
    let session = guid.get("sessionID")?.as_u32()?;
    let local = guid.get("localID")?.as_u32()?;
    if session == u32::MAX && local == u32::MAX {
        return None;
    }
    Some(NodeId { session, local })
}
