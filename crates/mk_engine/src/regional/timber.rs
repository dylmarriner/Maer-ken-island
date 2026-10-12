//! Felled timber: the log on the ground, and getting it to the store.
//!
//! A felled stem does not go anywhere by itself. Its usable wood comes
//! down as a log lying from the stump in the direction it fell, green:
//! wood at felling holds 0.82 kg of water per kg of oven-dry wood in
//! hardwoods and 1.06 in softwoods (Wood Handbook 2010 Table 4-1, as
//! averaged by Penn State Extension; `fixtures/reference/labour/timber.json`).
//! Its size follows from its mass and the wood's basic density: a cylinder
//! of the stem's diameter, no longer than the tree was tall.
//!
//! Someone then has to move it. A founder who works the log chooses the
//! way that costs them the least time of those the estate allows:
//!
//! - **the excavator** (the estate's CAT 330): walk to the garage, track
//!   out to the log at its 5.3 km/h, pick it up in one loader cycle,
//!   track to the store carrying it, set it down, track back. Only a log
//!   within its rated lift (5,800 kg over the side at 6 m) and with fuel
//!   in the tank.
//! - **the ute** (the Ford Raptor): cross-cut the log with the axe into
//!   pieces a person can carry (no heavier than the sustainable load for
//!   their body, no longer than the ute's bed), walk to the garage, drive
//!   to the log, carry the pieces to the ute one by one, drive to the
//!   store, carry them in one by one, as many trips as the ute's payload
//!   needs, and drive the ute home.
//! - **on foot**: cross-cut it the same way and carry every piece to the
//!   store.
//!
//! Every leg is timed from the packs (walking and loaded walking, the
//! felling pack's cut time for a cross-cut through the same section,
//! vehicle speeds, the CAT's travel speed, loader-crane cycle times) and
//! costs its Compendium MET. Fuel burns as the vehicles move. A log in the
//! store keeps its water: nothing here dries it (deviation D40).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::labour::{LabourBody, LabourError, LabourTable, LabourTool};
use crate::humans::lifecycle::WALKING_MET;

/// Where a log is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogPlace {
    /// Lying where it fell or was put down.
    Ground,
    /// On, or held by, the vehicle with this item id.
    Vehicle(u64),
    /// In the estate's timber store.
    Stored,
}

/// A log or a cross-cut piece of one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub id: u64,
    /// Centre (domain metres).
    pub position_m: (f64, f64),
    /// Direction from the butt end to the top end (rad, from +x).
    pub heading_rad: f64,
    pub length_m: f64,
    pub diameter_m: f64,
    /// Oven-dry wood (kg).
    pub dry_kg: f64,
    pub carbon_kgc: f64,
    /// Water held in the wood (kg).
    pub water_kg: f64,
    pub place: LogPlace,
}

impl Log {
    /// Mass as it is (kg): wood and water.
    pub fn green_kg(&self) -> f64 {
        self.dry_kg + self.water_kg
    }

    /// Water per kg of oven-dry wood.
    pub fn moisture_content(&self) -> f64 {
        if self.dry_kg > 0.0 {
            self.water_kg / self.dry_kg
        } else {
            0.0
        }
    }

    /// The butt end (domain metres).
    pub fn butt_m(&self) -> (f64, f64) {
        let half = 0.5 * self.length_m;
        (
            self.position_m.0 - half * self.heading_rad.cos(),
            self.position_m.1 - half * self.heading_rad.sin(),
        )
    }

    /// Cross-cut into `n` equal pieces along the log, numbered from the
    /// butt, with ids `first_id..first_id + n`.
    pub fn cross_cut(&self, n: u32, first_id: u64) -> Vec<Log> {
        let n = n.max(1);
        let k = f64::from(n);
        let piece_len = self.length_m / k;
        let butt = self.butt_m();
        (0..n)
            .map(|i| {
                let along = (f64::from(i) + 0.5) * piece_len;
                Log {
                    id: first_id + u64::from(i),
                    position_m: (
                        butt.0 + along * self.heading_rad.cos(),
                        butt.1 + along * self.heading_rad.sin(),
                    ),
                    heading_rad: self.heading_rad,
                    length_m: piece_len,
                    diameter_m: self.diameter_m,
                    dry_kg: self.dry_kg / k,
                    carbon_kgc: self.carbon_kgc / k,
                    water_kg: self.water_kg / k,
                    place: self.place,
                }
            })
            .collect()
    }
}

/// The log a felled stem leaves: `dry_kg` of usable wood holding
/// `carbon_kgc`, at `moisture_content` (water per kg dry) and
/// `basic_specific_gravity` (oven-dry mass per green volume, against
/// water), lying from the stump at `stump_m` toward `heading_rad`. A
/// cylinder of the stem's diameter, shortened to the tree's height with
/// the diameter widened to keep its volume if it would be longer.
#[allow(clippy::too_many_arguments)]
pub fn felled_log(
    id: u64,
    stump_m: (f64, f64),
    heading_rad: f64,
    stem_diameter_m: f64,
    tree_height_m: f64,
    dry_kg: f64,
    carbon_kgc: f64,
    moisture_content: f64,
    basic_specific_gravity: f64,
) -> Log {
    let volume_m3 = dry_kg.max(0.0) / (basic_specific_gravity.max(1e-6) * 1_000.0);
    let mut diameter = stem_diameter_m.max(1e-3);
    let mut length = volume_m3 / (std::f64::consts::PI * diameter * diameter / 4.0);
    if length > tree_height_m && tree_height_m > 0.0 {
        length = tree_height_m;
        diameter = (4.0 * volume_m3 / (std::f64::consts::PI * length)).sqrt();
    }
    Log {
        id,
        position_m: (
            stump_m.0 + 0.5 * length * heading_rad.cos(),
            stump_m.1 + 0.5 * length * heading_rad.sin(),
        ),
        heading_rad,
        length_m: length,
        diameter_m: diameter,
        dry_kg: dry_kg.max(0.0),
        carbon_kgc: carbon_kgc.max(0.0),
        water_kg: dry_kg.max(0.0) * moisture_content.max(0.0),
        place: LogPlace::Ground,
    }
}

/// The direction a felled tree falls (rad): drawn from the tree's own id,
/// so the same tree always falls the same way.
pub fn fall_heading_rad(tree_id: u64, seed: &[u8; 32]) -> f64 {
    let mut h = blake3::Hasher::new();
    h.update(b"fall-heading");
    h.update(seed);
    h.update(&tree_id.to_le_bytes());
    let bytes = h.finalize();
    let u = u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("8 bytes")) as f64
        / u64::MAX as f64;
    u * std::f64::consts::TAU
}

/// How a founder moves a log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HaulMethod {
    Excavator,
    Ute,
    OnFoot,
}

/// One timed piece of a haul and what it does when it is done.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HaulAction {
    /// The founder walks somewhere.
    Walk,
    /// Cross-cut log `log_id` into `pieces`, with ids from `first_id`.
    CrossCut {
        log_id: u64,
        pieces: u32,
        first_id: u64,
    },
    /// Drive a road vehicle `km`, carrying `logs`.
    Drive {
        vehicle_id: u64,
        km: f64,
        logs: Vec<u64>,
    },
    /// Carry `logs` one by one onto the vehicle.
    Load { vehicle_id: u64, logs: Vec<u64> },
    /// Carry `logs` one by one off the vehicle into the store.
    Unload { logs: Vec<u64> },
    /// Carry one piece on foot from where it lies into the store.
    CarryToStore { log_id: u64 },
    /// Run a tracked machine for `hours`, carrying `log`.
    Track {
        vehicle_id: u64,
        hours: f64,
        log: Option<u64>,
    },
    /// The machine picks the log up.
    Lift { vehicle_id: u64, log_id: u64 },
    /// The machine sets the log down in the store.
    SetDown { log_id: u64 },
}

/// A timed task of a haul.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaulTask {
    pub action: HaulAction,
    pub duration_s: f64,
    pub met: f64,
    /// Where the founder is when it is done (domain metres).
    pub ends_at_m: (f64, f64),
}

/// A founder's haul of one log: the tasks still to do, the first under
/// way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HaulJob {
    pub log_id: u64,
    pub method: HaulMethod,
    pub tasks: VecDeque<HaulTask>,
    /// Seconds into the first task.
    pub worked_s: f64,
}

impl HaulJob {
    /// Seconds of work the whole haul takes.
    pub fn total_s(&self) -> f64 {
        self.tasks.iter().map(|t| t.duration_s).sum()
    }
}

/// The estate's places a haul runs between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HaulSite {
    /// The garage's door (domain metres): where the vehicles are.
    pub garage_m: (f64, f64),
    /// The store's door: where a vehicle stops to unload.
    pub store_m: (f64, f64),
    /// How far a piece is carried from the door to the stack (m).
    pub store_carry_m: f64,
}

/// A road vehicle: item id, fuel in its tank (L), and burn (L/km).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadVehicle {
    pub item_id: u64,
    pub litres: f64,
    pub litres_per_km: f64,
}

/// A tracked machine: item id, fuel (L), and burn (L/h).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackedMachine {
    pub item_id: u64,
    pub litres: f64,
    pub litres_per_hour: f64,
}

/// What the estate has to move logs with.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Fleet {
    pub ute: Option<RoadVehicle>,
    pub excavator: Option<TrackedMachine>,
}

fn distance_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Every way of moving `log` the estate allows, with its tasks, for a
/// founder with `body` standing at `founder_m`. Cross-cut pieces take ids
/// from `first_piece_id`.
pub fn haul_plans(
    log: &Log,
    founder_m: (f64, f64),
    body: &LabourBody,
    labour: &LabourTable,
    site: &HaulSite,
    fleet: &Fleet,
    first_piece_id: u64,
) -> Result<Vec<HaulJob>, LabourError> {
    let walk = |d: f64| labour.walking_time_s(body, d, 0.0, 0.0);
    let carry_met = labour.met("carrying_stacking_logs")?;
    let mut plans = Vec::new();
    let butt = log.butt_m();
    let to_butt = walk(distance_m(founder_m, butt))?;

    // Cross-cut pieces: none heavier than the body's sustainable load.
    let sustainable = labour.sustainable_load_kg(body)?;
    let by_mass = (log.green_kg() / sustainable.max(1e-6)).ceil().max(1.0) as u32;
    let cut = labour.fell_tree(LabourTool::SteelAxe, log.diameter_m * 100.0)?;
    let cross_cut = |pieces: u32| HaulTask {
        action: HaulAction::CrossCut {
            log_id: log.id,
            pieces,
            first_id: first_piece_id,
        },
        // Each cut through the log is a cut through the same section the
        // felling cut went through.
        duration_s: to_butt + f64::from(pieces - 1) * cut.duration_s,
        met: if pieces > 1 { cut.met } else { WALKING_MET },
        ends_at_m: butt,
    };
    let piece_at =
        |pieces: u32, i: u32| log.cross_cut(pieces, first_piece_id)[i as usize].position_m;

    // The ute.
    if let Some(ute) = fleet.ute {
        let bed = labour.ute_bed_length_m()?;
        let payload = labour.ute_payload_kg()?;
        let pieces = by_mass.max((log.length_m / bed).ceil() as u32).max(1);
        let piece_kg = log.green_kg() / f64::from(pieces);
        let per_trip = ((payload / piece_kg).floor() as u32).max(1);
        let speed = labour.vehicle_speed_m_s("farm_track")?;
        let drive_met = labour.met("driving_light_truck")?;
        let mut tasks = VecDeque::new();
        tasks.push_back(cross_cut(pieces));
        tasks.push_back(HaulTask {
            action: HaulAction::Walk,
            duration_s: walk(distance_m(butt, site.garage_m))?,
            met: WALKING_MET,
            ends_at_m: site.garage_m,
        });
        let drive = |from: (f64, f64), to: (f64, f64), logs: Vec<u64>| HaulTask {
            action: HaulAction::Drive {
                vehicle_id: ute.item_id,
                km: distance_m(from, to) / 1_000.0,
                logs,
            },
            duration_s: distance_m(from, to) / speed,
            met: drive_met,
            ends_at_m: to,
        };
        tasks.push_back(drive(site.garage_m, butt, Vec::new()));
        let ids: Vec<u64> = (0..pieces).map(|i| first_piece_id + u64::from(i)).collect();
        let mut km = distance_m(site.garage_m, butt) / 1_000.0;
        for (t, trip) in ids.chunks(per_trip as usize).enumerate() {
            let (mut carry_s, mut back_s) = (0.0, 0.0);
            for &id in trip {
                let d = distance_m(piece_at(pieces, (id - first_piece_id) as u32), butt);
                carry_s += labour.walking_time_s(body, d, 0.0, piece_kg)?;
                back_s += walk(d)?;
            }
            tasks.push_back(HaulTask {
                action: HaulAction::Load {
                    vehicle_id: ute.item_id,
                    logs: trip.to_vec(),
                },
                duration_s: carry_s + back_s,
                met: (carry_s * carry_met + back_s * WALKING_MET) / (carry_s + back_s).max(1e-9),
                ends_at_m: butt,
            });
            tasks.push_back(drive(butt, site.store_m, trip.to_vec()));
            let n = trip.len() as f64;
            let carry_s = n * labour.walking_time_s(body, site.store_carry_m, 0.0, piece_kg)?;
            let back_s = n * walk(site.store_carry_m)?;
            tasks.push_back(HaulTask {
                action: HaulAction::Unload {
                    logs: trip.to_vec(),
                },
                duration_s: carry_s + back_s,
                met: (carry_s * carry_met + back_s * WALKING_MET) / (carry_s + back_s).max(1e-9),
                ends_at_m: site.store_m,
            });
            km += distance_m(butt, site.store_m) / 1_000.0;
            if (t + 1) * (per_trip as usize) < ids.len() {
                tasks.push_back(drive(site.store_m, butt, Vec::new()));
                km += distance_m(site.store_m, butt) / 1_000.0;
            }
        }
        tasks.push_back(drive(site.store_m, site.garage_m, Vec::new()));
        km += distance_m(site.store_m, site.garage_m) / 1_000.0;
        if ute.litres >= km * ute.litres_per_km {
            plans.push(HaulJob {
                log_id: log.id,
                method: HaulMethod::Ute,
                tasks,
                worked_s: 0.0,
            });
        }
    }

    // On foot: every piece carried to the store.
    {
        let pieces = by_mass;
        let piece_kg = log.green_kg() / f64::from(pieces);
        let mut tasks = VecDeque::new();
        tasks.push_back(cross_cut(pieces));
        let mut at = butt;
        for i in 0..pieces {
            let id = first_piece_id + u64::from(i);
            let p = piece_at(pieces, i);
            let to_piece = walk(distance_m(at, p))?;
            let carry_s = labour.walking_time_s(
                body,
                distance_m(p, site.store_m) + site.store_carry_m,
                0.0,
                piece_kg,
            )?;
            let back_s = walk(site.store_carry_m)?;
            let walk_s = to_piece + back_s;
            tasks.push_back(HaulTask {
                action: HaulAction::CarryToStore { log_id: id },
                duration_s: walk_s + carry_s,
                met: (carry_s * carry_met + walk_s * WALKING_MET) / (carry_s + walk_s).max(1e-9),
                ends_at_m: site.store_m,
            });
            at = site.store_m;
        }
        plans.push(HaulJob {
            log_id: log.id,
            method: HaulMethod::OnFoot,
            tasks,
            worked_s: 0.0,
        });
    }

    // The excavator, for a log within its rated lift.
    if let Some(ex) = fleet.excavator {
        if log.green_kg() <= labour.excavator_lift_kg()? {
            let speed = labour.excavator_travel_m_s()?;
            let op_met = labour.met("operating_heavy_equipment")?;
            let track = |from: (f64, f64), to: (f64, f64), log_id: Option<u64>| {
                let s = distance_m(from, to) / speed;
                HaulTask {
                    action: HaulAction::Track {
                        vehicle_id: ex.item_id,
                        hours: s / 3_600.0,
                        log: log_id,
                    },
                    duration_s: s,
                    met: op_met,
                    ends_at_m: to,
                }
            };
            let lift_s = labour.crane_cycle_s("loading")?;
            let set_s = labour.crane_cycle_s("unloading")?;
            let tasks: VecDeque<HaulTask> = [
                HaulTask {
                    action: HaulAction::Walk,
                    duration_s: walk(distance_m(founder_m, site.garage_m))?,
                    met: WALKING_MET,
                    ends_at_m: site.garage_m,
                },
                track(site.garage_m, log.position_m, None),
                HaulTask {
                    action: HaulAction::Lift {
                        vehicle_id: ex.item_id,
                        log_id: log.id,
                    },
                    duration_s: lift_s,
                    met: op_met,
                    ends_at_m: log.position_m,
                },
                track(log.position_m, site.store_m, Some(log.id)),
                HaulTask {
                    action: HaulAction::SetDown { log_id: log.id },
                    duration_s: set_s,
                    met: op_met,
                    ends_at_m: site.store_m,
                },
                track(site.store_m, site.garage_m, None),
            ]
            .into_iter()
            .collect();
            let hours: f64 = tasks
                .iter()
                .map(|t| match t.action {
                    HaulAction::Track { hours, .. } => hours,
                    HaulAction::Lift { .. } | HaulAction::SetDown { .. } => t.duration_s / 3_600.0,
                    _ => 0.0,
                })
                .sum();
            if ex.litres >= hours * ex.litres_per_hour {
                plans.push(HaulJob {
                    log_id: log.id,
                    method: HaulMethod::Excavator,
                    tasks,
                    worked_s: 0.0,
                });
            }
        }
    }
    Ok(plans)
}

/// The plan that costs the founder the least time.
pub fn quickest(plans: Vec<HaulJob>) -> Option<HaulJob> {
    plans
        .into_iter()
        .min_by(|a, b| a.total_s().total_cmp(&b.total_s()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> Log {
        felled_log(1, (0.0, 0.0), 0.0, 0.30, 20.0, 400.0, 190.0, 0.82, 0.52)
    }

    #[test]
    fn a_felled_log_is_green_and_lies_from_the_stump() {
        let l = log();
        assert!((l.water_kg - 0.82 * 400.0).abs() < 1e-9);
        assert!((l.green_kg() - 728.0).abs() < 1e-9);
        // 400 kg dry at 520 kg/m^3: 0.769 m^3, a 0.30 m cylinder 10.9 m long.
        let v = 400.0 / 520.0;
        assert!((l.length_m - v / (std::f64::consts::PI * 0.0225)).abs() < 1e-9);
        assert!(l.butt_m().0.abs() < 1e-9 && l.butt_m().1.abs() < 1e-9);
        assert_eq!(l.place, LogPlace::Ground);
        // Too long for the tree: shortened to its height, volume kept.
        let short = felled_log(2, (0.0, 0.0), 0.0, 0.10, 5.0, 400.0, 190.0, 0.82, 0.52);
        assert_eq!(short.length_m, 5.0);
        let vol = std::f64::consts::PI * short.diameter_m * short.diameter_m / 4.0 * 5.0;
        assert!((vol - v).abs() < 1e-9);
    }

    #[test]
    fn cross_cutting_keeps_every_kilogram_and_lays_the_pieces_end_to_end() {
        let l = log();
        let pieces = l.cross_cut(4, 10);
        assert_eq!(pieces.len(), 4);
        let sum = |f: fn(&Log) -> f64| pieces.iter().map(f).sum::<f64>();
        assert!((sum(|p| p.dry_kg) - l.dry_kg).abs() < 1e-9);
        assert!((sum(|p| p.water_kg) - l.water_kg).abs() < 1e-9);
        assert!((sum(|p| p.carbon_kgc) - l.carbon_kgc).abs() < 1e-9);
        assert!((sum(|p| p.length_m) - l.length_m).abs() < 1e-9);
        assert_eq!(pieces[0].id, 10);
        assert!(pieces
            .windows(2)
            .all(|w| w[1].position_m.0 > w[0].position_m.0));
    }

    #[test]
    fn a_tree_falls_the_same_way_every_time() {
        let seed = [7u8; 32];
        assert_eq!(fall_heading_rad(5, &seed), fall_heading_rad(5, &seed));
        assert_ne!(fall_heading_rad(5, &seed), fall_heading_rad(6, &seed));
        let h = fall_heading_rad(5, &seed);
        assert!((0.0..std::f64::consts::TAU).contains(&h));
    }

    fn labour() -> LabourTable {
        LabourTable::load_default().expect("reference packs load")
    }

    fn body() -> LabourBody {
        LabourBody {
            sex: crate::humans::BiologicalSex::Male,
            age_years: 30.0,
            height_cm: 178.0,
            weight_kg: 80.0,
        }
    }

    fn site() -> HaulSite {
        HaulSite {
            garage_m: (100.0, 0.0),
            store_m: (120.0, 0.0),
            store_carry_m: 4.0,
        }
    }

    fn fleet(ute_litres: f64, ex_litres: f64) -> Fleet {
        Fleet {
            ute: Some(RoadVehicle {
                item_id: 1,
                litres: ute_litres,
                litres_per_km: 0.27,
            }),
            excavator: Some(TrackedMachine {
                item_id: 2,
                litres: ex_litres,
                litres_per_hour: 32.4,
            }),
        }
    }

    #[test]
    fn by_hand_a_log_is_cut_into_pieces_a_body_can_carry_and_the_ute_takes_its_payload() {
        let (labour, body) = (labour(), body());
        let l = log();
        let plans = haul_plans(
            &l,
            (0.0, 0.0),
            &body,
            &labour,
            &site(),
            &fleet(100.0, 0.0),
            100,
        )
        .unwrap();
        let ute = plans
            .iter()
            .find(|p| p.method == HaulMethod::Ute)
            .expect("a ute plan");
        let pieces = match &ute.tasks[0].action {
            HaulAction::CrossCut { pieces, .. } => *pieces,
            other => panic!("starts with {other:?}"),
        };
        let piece_kg = l.green_kg() / f64::from(pieces);
        assert!(piece_kg <= labour.sustainable_load_kg(&body).unwrap() + 1e-9);
        assert!(l.length_m / f64::from(pieces) <= labour.ute_bed_length_m().unwrap() + 1e-9);
        let payload = labour.ute_payload_kg().unwrap();
        let mut loaded = 0;
        for t in &ute.tasks {
            if let HaulAction::Load { logs, .. } = &t.action {
                assert!(
                    logs.len() as f64 * piece_kg <= payload + 1e-9,
                    "over the payload"
                );
                loaded += logs.len();
            }
        }
        assert_eq!(loaded, pieces as usize, "every piece goes on the ute");
        // Cutting takes the felling pack's cut time per cut.
        let cut = labour
            .fell_tree(LabourTool::SteelAxe, l.diameter_m * 100.0)
            .unwrap()
            .duration_s;
        assert!(ute.tasks[0].duration_s >= f64::from(pieces - 1) * cut);
        // With no fuel in the excavator, the quickest is a hand method.
        let best = quickest(plans).unwrap();
        assert_ne!(best.method, HaulMethod::Excavator);
    }

    #[test]
    fn the_excavator_lifts_only_what_it_is_rated_for_and_only_with_fuel() {
        let (labour, body) = (labour(), body());
        let l = log();
        let plans = haul_plans(
            &l,
            (0.0, 0.0),
            &body,
            &labour,
            &site(),
            &fleet(100.0, 474.0),
            100,
        )
        .unwrap();
        let ex = plans
            .iter()
            .find(|p| p.method == HaulMethod::Excavator)
            .expect("an excavator plan");
        // Track out at the CAT's 5.3 km/h.
        let track = ex
            .tasks
            .iter()
            .find(|t| matches!(t.action, HaulAction::Track { .. }))
            .unwrap();
        let d = (l.position_m.0 - 100.0).hypot(l.position_m.1);
        assert!((track.duration_s - d / (5.3 / 3.6)).abs() < 1e-9);
        // Too heavy to lift: no excavator plan.
        let heavy = felled_log(3, (0.0, 0.0), 0.0, 1.2, 40.0, 6_000.0, 2_850.0, 0.82, 0.52);
        assert!(heavy.green_kg() > labour.excavator_lift_kg().unwrap());
        let plans = haul_plans(
            &heavy,
            (0.0, 0.0),
            &body,
            &labour,
            &site(),
            &fleet(100.0, 474.0),
            100,
        )
        .unwrap();
        assert!(plans.iter().all(|p| p.method != HaulMethod::Excavator));
        // An empty ute leaves only the walk.
        let plans = haul_plans(
            &l,
            (0.0, 0.0),
            &body,
            &labour,
            &site(),
            &fleet(0.0, 0.0),
            100,
        )
        .unwrap();
        assert_eq!(quickest(plans).unwrap().method, HaulMethod::OnFoot);
    }
}
