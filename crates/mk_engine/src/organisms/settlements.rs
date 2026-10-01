use crate::agents::GridPosition;
use crate::humans::HumanSystem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteBeing {
    pub id: u64,
    pub kind: SiteKind,
    pub position: GridPosition,
    pub occupants: Vec<u64>,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SiteKind {
    Shelter,
    Settlement,
}

/// Constructed structures on one cell at which a shelter becomes a
/// settlement.
pub const SETTLEMENT_MIN_STRUCTURES: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettlementSystem {
    pub sites: Vec<SiteBeing>,
    pub next_id: u64,
}

impl SettlementSystem {
    pub fn new() -> Self {
        Self {
            sites: Vec::new(),
            next_id: 1,
        }
    }

    /// Reconcile sites with the built environment and the people in it.
    ///
    /// Every cell holding at least one constructed structure is a site — a
    /// [`SiteKind::Shelter`], or a [`SiteKind::Settlement`] once it has
    /// [`SETTLEMENT_MIN_STRUCTURES`] structures. Human presence alone never
    /// creates a site. Occupants are the *living* humans standing on the
    /// site; a site whose people have all left stays (its buildings do) but
    /// is marked inactive.
    pub fn sync_humans(
        &mut self,
        humans: &HumanSystem,
        structures: &[crate::resource_economy::ConstructedStructure],
    ) {
        let mut structures_per_cell: std::collections::BTreeMap<(i32, i32), usize> =
            std::collections::BTreeMap::new();
        for structure in structures {
            *structures_per_cell
                .entry((structure.position.row, structure.position.col))
                .or_insert(0) += 1;
        }
        for (&(row, col), &count) in &structures_per_cell {
            let position = GridPosition::new(row, col);
            let kind = if count >= SETTLEMENT_MIN_STRUCTURES {
                SiteKind::Settlement
            } else {
                SiteKind::Shelter
            };
            match self.sites.iter_mut().find(|site| site.position == position) {
                Some(site) => site.kind = kind,
                None => {
                    let id = self.next_id.max(1);
                    self.next_id = id + 1;
                    self.sites.push(SiteBeing {
                        id,
                        kind,
                        position,
                        occupants: Vec::new(),
                        active: false,
                    });
                }
            }
        }

        for site in &mut self.sites {
            site.occupants.clear();
        }
        for human in humans.registry.get_all_humans() {
            if !matches!(human.profile.status, crate::humans::HumanStatus::Alive) {
                continue;
            }
            if let Some(site) = self
                .sites
                .iter_mut()
                .find(|site| site.position == human.position)
            {
                site.occupants.push(human.profile.human_id.0);
            }
        }
        for site in &mut self.sites {
            site.active = !site.occupants.is_empty();
        }
    }

    pub fn build_site(&mut self, kind: SiteKind, position: GridPosition, builder_id: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.sites.push(SiteBeing {
            id,
            kind,
            position,
            occupants: vec![builder_id],
            active: true,
        });
        id
    }
}
