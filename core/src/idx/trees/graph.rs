use crate::idx::trees::hnsw::ElementId;
use std::collections::hash_map::Entry as HEntry;
use std::collections::{HashMap, HashSet};

pub(super) struct UndirectedGraph {
	m_max: usize,
	nodes: HashMap<ElementId, HashSet<ElementId>>,
}

impl From<usize> for UndirectedGraph {
	fn from(m_max: usize) -> Self {
		Self {
			m_max,
			nodes: HashMap::new(),
		}
	}
}

impl UndirectedGraph {
	pub(super) fn get_edges(&self, node: &ElementId) -> Option<&HashSet<ElementId>> {
		self.nodes.get(node)
	}

	#[cfg(test)]
	pub(super) fn add_edge(&mut self, node1: ElementId, node2: ElementId) {
		if node1 != node2 {
			self.nodes.entry(node1).or_default().insert(node2);
			self.nodes.entry(node2).or_default().insert(node1);
		}
	}

	pub(super) fn add_empty_node(&mut self, node: ElementId) -> bool {
		if let HEntry::Vacant(e) = self.nodes.entry(node) {
			e.insert(HashSet::with_capacity(self.m_max));
			true
		} else {
			false
		}
	}
	pub(super) fn add_node(
		&mut self,
		node: ElementId,
		edges: HashSet<ElementId>,
	) -> Option<Vec<ElementId>> {
		let edges: Vec<ElementId> = if let HEntry::Vacant(e) = self.nodes.entry(node) {
			e.insert(edges).iter().copied().collect()
		} else {
			return None;
		};
		// Bidirectional
		for &e in &edges {
			self.nodes.entry(e).or_default().insert(node);
		}
		Some(edges)
	}

	pub(super) fn set_node(&mut self, node: ElementId, edges: HashSet<ElementId>) {
		let (to_add, to_remove) = match self.nodes.entry(node) {
			HEntry::Occupied(mut e) => {
				let old_edges = e.get();
				let mut to_remove = Vec::with_capacity(1);
				for old_edge in old_edges {
					if !edges.contains(old_edge) {
						to_remove.push(*old_edge);
					}
				}
				let mut to_add = Vec::with_capacity(1);
				for new_edge in &edges {
					if !old_edges.contains(new_edge) {
						to_add.push(*new_edge);
					}
				}
				e.insert(edges);
				(to_add, to_remove)
			}
			HEntry::Vacant(e) => {
				let to_add: Vec<ElementId> = e.insert(edges).iter().copied().collect();
				(to_add, vec![])
			}
		};
		for n in to_add {
			self.nodes.entry(n).or_default().insert(node);
		}
		for n in to_remove {
			self.nodes.entry(n).or_default().remove(&node);
		}
	}

	pub(super) fn remove_node(&mut self, node: &ElementId) -> Option<HashSet<ElementId>> {
		if let Some(edges) = self.nodes.remove(node) {
			for edge in &edges {
				if let Some(edges_to_node) = self.nodes.get_mut(edge) {
					edges_to_node.remove(node);
				}
			}
			Some(edges)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::idx::trees::graph::UndirectedGraph;
	use crate::idx::trees::hnsw::ElementId;
	use std::collections::{HashMap, HashSet};

	impl UndirectedGraph {
		pub(in crate::idx::trees) fn len(&self) -> usize {
			self.nodes.len()
		}

		pub(in crate::idx::trees) fn nodes(&self) -> &HashMap<ElementId, HashSet<ElementId>> {
			&self.nodes
		}
		pub(in crate::idx::trees) fn check(&self, g: Vec<(ElementId, Vec<ElementId>)>) {
			for (n, e) in g {
				let edges: HashSet<ElementId> = e.into_iter().collect();
				assert_eq!(self.get_edges(&n), Some(&edges), "{n}");
			}
		}
	}

	#[test]
	fn test_undirected_graph() {
		// Graph creation
		let mut g: UndirectedGraph = 10.into();
		assert_eq!(g.m_max, 10);

		// Adding an empty node
		let res = g.add_empty_node(0);
		assert!(res);
		g.check(vec![(0, vec![])]);

		// Adding the same node
		let res = g.add_empty_node(0);
		assert!(!res);
		g.check(vec![(0, vec![])]);

		// Adding a node with one edge
		let res = g.add_node(1, HashSet::from([0]));
		assert_eq!(res, Some(vec![0]));
		g.check(vec![(0, vec![1]), (1, vec![0])]);

		// Adding the same node
		let res = g.add_node(1, HashSet::from([2]));
		assert_eq!(res, None);
		g.check(vec![(0, vec![1]), (1, vec![0])]);

		// Adding a node with two edges
		let res = g.add_node(2, HashSet::from([0, 1]));
		assert_eq!(
			res.map(|mut v| {
				v.sort();
				v
			}),
			Some(vec![0, 1])
		);
		g.check(vec![(0, vec![1, 2]), (1, vec![0, 2]), (2, vec![0, 1])]);

		// Adding a node with two edges
		let res = g.add_node(3, HashSet::from([1, 2]));
		assert_eq!(
			res.map(|mut v| {
				v.sort();
				v
			}),
			Some(vec![1, 2])
		);
		g.check(vec![(0, vec![1, 2]), (1, vec![0, 2, 3]), (2, vec![0, 1, 3]), (3, vec![1, 2])]);

		// Change the edges of a node
		g.set_node(3, HashSet::from([0]));
		g.check(vec![(0, vec![1, 2, 3]), (1, vec![0, 2]), (2, vec![0, 1]), (3, vec![0])]);

		// Add an edge
		g.add_edge(2, 3);
		g.check(vec![(0, vec![1, 2, 3]), (1, vec![0, 2]), (2, vec![0, 1, 3]), (3, vec![0, 2])]);

		// Remove a node
		let res = g.remove_node(&2);
		assert_eq!(
			res.map(|v| {
				let mut v: Vec<ElementId> = v.into_iter().collect();
				v.sort();
				v
			}),
			Some(vec![0, 1, 3])
		);
		g.check(vec![(0, vec![1, 3]), (1, vec![0]), (3, vec![0])]);

		// Remove again
		let res = g.remove_node(&2);
		assert_eq!(res, None);

		// Set a non existing node
		g.set_node(2, HashSet::from([1]));
		g.check(vec![(0, vec![1, 3]), (1, vec![0, 2]), (2, vec![1]), (3, vec![0])]);
	}
}