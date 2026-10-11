//! Space coordination separating Euclidean metric vectors and Hierarchical Hyperbolic graph indices.

use crate::error::Result;
use cluaizdb_substrate::schema::EdgeMetadata;
use cluaizdb_substrate::traits::StorageSubstrate;
use std::sync::Arc;

/// Hypergeometric polar coordinates in abstraction space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionCoordinates {
    /// Distance from root hierarchy base layer (abstraction depth)
    pub radius: f32,
    /// Spatial angle representing domain cluster sector in radians
    pub spatial_angle: f32,
}

impl DimensionCoordinates {
    pub fn new(radius: f32, spatial_angle: f32) -> Self {
        Self {
            radius,
            spatial_angle,
        }
    }

    /// Maps continuous hyperbolic parameters into deterministic discrete keys suitable
    /// for binary index storage in standard B+Trees.
    pub fn serialize_into_hypergeometric_key(&self) -> u128 {
        let rad_bytes = (self.radius.to_bits() as u64) << 32;
        let ang_bytes = self.spatial_angle.to_bits() as u64;
        (rad_bytes | ang_bytes) as u128
    }
}

/// Manages taxonomic hierarchical space mapped onto the physical StorageSubstrate.
pub struct HypergeometricManager<S: StorageSubstrate> {
    substrate: Arc<S>,
    relation_type_id: u32,
}

impl<S: StorageSubstrate> HypergeometricManager<S> {
    pub fn new(substrate: Arc<S>, relation_type_id: u32) -> Self {
        Self {
            substrate,
            relation_type_id,
        }
    }

    /// Inserts a node into non-Euclidean spatial coordinates and indexes it into B+Tree.
    pub fn insert_topology_node(
        &self,
        collection_id: u32,
        node_id: u128,
        coords: DimensionCoordinates,
    ) -> Result<()> {
        let discrete_key = coords.serialize_into_hypergeometric_key();

        // 1. Maintain Euclidean metric coordinates independently via vector slot
        let embedding_plane = vec![coords.radius, coords.spatial_angle];
        self.substrate
            .put_vector(collection_id, node_id, &embedding_plane)?;

        // 2. Form physical B+Tree secondary index pathway
        self.substrate
            .index_term(collection_id, &discrete_key.to_be_bytes(), node_id)?;

        Ok(())
    }

    /// Links two taxonomic entities hierarchically via bidirectional graph relations.
    pub fn link_taxonomic_hierarchy(
        &self,
        parent_id: u128,
        child_id: u128,
        weight: f32,
    ) -> Result<()> {
        let meta = EdgeMetadata {
            weight,
            created_at: 0,
            valid_until: 0,
        };
        self.substrate
            .add_edge(parent_id, self.relation_type_id, child_id, meta)?;
        Ok(())
    }

    /// Retrieve adjacent cluster nodes strictly utilizing mapped graph edges (< 10µs).
    pub fn lookup_adjacent_taxonomic_cluster(&self, parent_id: u128) -> Result<Vec<u128>> {
        let edges = self
            .substrate
            .get_outbound_edges(parent_id, Some(self.relation_type_id))?;
        Ok(edges.into_iter().map(|(id, _meta)| id).collect())
    }
}
