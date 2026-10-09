//! How big the estate's things actually are, in metres.
//!
//! Until this existed, a renderer drawing the estate had to invent a size
//! for every object: the island recorded that Gem-D owns a Ford Raptor
//! and where it stands, not that it is 5.38 m long. The invention was
//! confined and labelled, but it was still invention, and a renderer is
//! the wrong place for a fact about the world.
//!
//! So the facts live here, and the renderer reads them. Three rules,
//! which are the whole of the policy:
//!
//! 1. **Every figure carries its source**, in the value itself rather
//!    than in a comment, so a reader can check one without reading the
//!    other. `REALISM.md`'s rule is that a number with an invented
//!    provenance is worse than a number with none, because it looks like
//!    sourced data and is not. [`Dimensions::source`] is therefore not
//!    optional and not decorative: it is checked by a test.
//! 2. **Named products get the manufacturer's figures.** This estate owns
//!    a Ford Ranger Raptor, two Fendt Varios and a Polaris RZR Pro R.
//!    Those are real machines with published dimensions, and guessing at
//!    the size of something whose spec sheet exists would be inexcusable.
//! 3. **Derived, never stored.** Nothing here enters `WorldState`, so
//!    nothing here enters the canonical state digest. A size is a fact
//!    *about* an item, not a thing the simulation evolves; storing it
//!    would move every recorded digest for no gain.
//!
//! Where a figure is a convention rather than a measurement — a hand tool
//! kept in a drawer, say — the source says so in those words. The point
//! is never to make everything look sourced; it is that a reader can tell
//! at a glance which is which.

use super::property::PropertyItemKind;

/// How big a thing is, and where that came from.
///
/// Metres, and oriented the way the thing is: `length_m` is its long
/// axis, `width_m` across, `height_m` up. A renderer that wants a box
/// gets one; a renderer that wants to know whether it fits through a door
/// can ask too.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dimensions {
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    /// Where the figure came from, in words a reader can check. Either a
    /// citation, or a plain statement that it is a convention.
    pub source: &'static str,
}

impl Dimensions {
    const fn new(length_m: f64, width_m: f64, height_m: f64, source: &'static str) -> Self {
        Self {
            length_m,
            width_m,
            height_m,
            source,
        }
    }

    /// The largest of the three, which is what a doorway or a shed cares
    /// about.
    pub fn longest_m(&self) -> f64 {
        self.length_m.max(self.width_m).max(self.height_m)
    }

    /// Footprint on the ground, m².
    pub fn footprint_m2(&self) -> f64 {
        self.length_m * self.width_m
    }
}

/// A television's screen, from its advertised diagonal.
///
/// Not a lookup but arithmetic, because a "75in" television has a
/// *defined* size: the diagonal is 75 inches and the panel is 16:9, so
/// the width is 75 × 16/√(16² + 9²) inches and the height 75 × 9/√(…).
/// That is geometry rather than a measurement, and it is better than any
/// retailer's figure because it does not depend on the bezel.
///
/// The depth is the convention: a modern flat panel on a stand is about
/// 60 mm deep at the thickest, and that part is stated as a convention
/// rather than dressed up as derived.
fn television(diagonal_inches: f64) -> (f64, f64, f64) {
    const INCH_M: f64 = 0.0254;
    let diagonal_m = diagonal_inches * INCH_M;
    let ratio = (16.0_f64 * 16.0 + 9.0 * 9.0).sqrt();
    (diagonal_m * 16.0 / ratio, 0.06, diagonal_m * 9.0 / ratio)
}

/// How big this thing is.
///
/// Matched on the name first, because the name is where the island keeps
/// the specification -- "Large TV (75in 8K, soundbar)" and "Ford Raptor
/// 4x4 Ute" each say exactly what they are -- and on the kind only as a
/// fallback for a thing the table has not met.
pub fn of(kind: PropertyItemKind, name: &str) -> Dimensions {
    if let Some(known) = by_name(name) {
        return known;
    }
    by_kind(kind)
}

/// Named things, with the figures their makers publish.
fn by_name(name: &str) -> Option<Dimensions> {
    // Vehicles the estate actually owns. Each of these is a real machine
    // and each figure is from the maker or from a spec table citing it.
    if name.contains("Ford Raptor") {
        // Ford Ranger Raptor, 2023 onward: 5,381 mm long, 2,028 mm wide
        // (body, excluding mirrors), 1,922 mm high.
        return Some(Dimensions::new(
            5.381,
            2.028,
            1.922,
            "Ford Ranger Raptor (2023-), 5381 x 2028 x 1922 mm, body width excluding mirrors \
             (carexpert.co.nz and vehiclescore.co.uk spec tables)",
        ));
    }
    if name.contains("Fendt 1000") {
        // Fendt's own 1000 Vario brochure: 6,350 mm overall length,
        // 2,750 mm overall width on standard tyres, cab height 3,506 to
        // 3,606 mm depending on variant. The taller figure is taken
        // because it is the one that has to fit through a door.
        return Some(Dimensions::new(
            6.350,
            2.750,
            3.606,
            "Fendt 1000 Vario official brochure (fendt.com 700251-fendt1000vario): 6350 mm \
             overall length, 2750 mm overall width on standard tyres, 3606 mm cab height",
        ));
    }
    if name.contains("Fendt 900") {
        // The Gen7 900's overall dimensions are not in Fendt's public
        // spec tables; the 939 Vario of the preceding series is, and it
        // is the same tractor class. Said plainly rather than passed off
        // as the Gen7's own figure.
        return Some(Dimensions::new(
            5.655,
            2.750,
            3.372,
            "Fendt 939 Vario (the 900 series preceding Gen7), 5655 x 2750 x 3372 mm \
             (Traktorenlexikon). Fendt publishes no overall dimensions for the Gen7 900, so \
             this is the nearest model rather than the named one",
        ));
    }
    if name.contains("Polaris RZR 2000 PRO R") || name.contains("PRO R") {
        // Polaris's own spec page for the 2-seat Pro R.
        return Some(Dimensions::new(
            3.470,
            1.880,
            1.850,
            "Polaris RZR Pro R (2-seat) official specification: 136.5 x 74 x 72.8 in \
             (3470 x 1880 x 1850 mm), polaris.com",
        ));
    }

    // Things whose size is fixed by a standard rather than by a maker.
    if name.contains("Network Equipment Rack") {
        // EIA-310 fixes 1U at 44.45 mm and the mounting width at 19 in;
        // the cabinet around it is the manufacturer's. A 42U enclosure
        // is the common full-height one.
        return Some(Dimensions::new(
            1.000,
            0.600,
            1.991,
            "42U enclosure for EIA-310 19-inch equipment: 1U is 44.45 mm and the mounting \
             width 482.6 mm by the standard; the 600 x 1000 x 1991 mm cabinet around it is \
             APC's Easy Rack 42U, a common full-height example",
        ));
    }
    if name.contains("King Size Bed") {
        // NZ king: 1,670 x 2,030 mm. The island's canon is New Zealand,
        // and a NZ king is narrower than an Australian one -- which is
        // exactly the sort of difference that makes "king size" a bad
        // thing to guess at.
        return Some(Dimensions::new(
            2.030,
            1.670,
            0.600,
            "New Zealand king mattress, 1670 x 2030 mm (bedpost.co.nz, derlook.co.nz; \
             Wikipedia gives 1650 mm). Height is the convention for a mattress on a base",
        ));
    }
    if name.contains("75in") {
        let (w, d, h) = television(75.0);
        return Some(Dimensions::new(
            w,
            d,
            h,
            "75-inch 16:9 panel, from the diagonal by geometry rather than a retailer's \
             figure. Depth is a convention for a flat panel on a stand",
        ));
    }
    if name.contains("55in") {
        let (w, d, h) = television(55.0);
        return Some(Dimensions::new(
            w,
            d,
            h,
            "55-inch 16:9 panel, from the diagonal by geometry rather than a retailer's \
             figure. Depth is a convention for a flat panel on a stand",
        ));
    }
    None
}

/// Everything the table has not met by name, by what kind of thing it is.
///
/// These are conventions and say so. They are not measurements of any
/// particular object, and the source string on each says that in words so
/// nothing downstream can mistake one for a figure with a citation.
fn by_kind(kind: PropertyItemKind) -> Dimensions {
    match kind {
        PropertyItemKind::Vehicle => Dimensions::new(
            4.800,
            1.800,
            1.800,
            "convention: a light utility vehicle, for one the table does not know by name",
        ),
        PropertyItemKind::VehicleAttachment => Dimensions::new(
            2.500,
            2.000,
            1.200,
            "convention: a tractor-mounted implement, for one the table does not know by name",
        ),
        PropertyItemKind::ShedTool => Dimensions::new(
            1.200,
            0.300,
            0.200,
            "convention: a long-handled garden tool",
        ),
        PropertyItemKind::BuildingEquipment => Dimensions::new(
            1.200,
            0.800,
            1.000,
            "convention: a workshop machine on a bench or a stand",
        ),
        PropertyItemKind::ArmouryItem => Dimensions::new(
            0.900,
            0.500,
            1.800,
            "convention: a full-height secure locker",
        ),
        PropertyItemKind::Computer => {
            Dimensions::new(0.500, 0.250, 0.450, "convention: a desktop tower case")
        }
        PropertyItemKind::HouseholdItem => Dimensions::new(
            0.800,
            0.600,
            0.900,
            "convention: a piece of household furniture",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_figure_says_where_it_came_from() {
        // The rule `REALISM.md` exists to enforce: a number with no
        // provenance is a number a reader cannot check, and a number
        // with an invented provenance is worse. Neither is allowed to
        // leave this module.
        let kinds = [
            PropertyItemKind::Vehicle,
            PropertyItemKind::VehicleAttachment,
            PropertyItemKind::ShedTool,
            PropertyItemKind::BuildingEquipment,
            PropertyItemKind::ArmouryItem,
            PropertyItemKind::Computer,
            PropertyItemKind::HouseholdItem,
        ];
        let named = [
            "Ford Raptor 4x4 Ute",
            "Fendt 1000 Vario",
            "Fendt 900 Vario",
            "Polaris RZR 2000 PRO R",
            "Network Equipment Rack",
            "King Size Bed",
            "Large TV (75in 8K, soundbar)",
            "TV (55in 4K smart)",
        ];
        for kind in kinds {
            let d = of(kind, "something the table has never met");
            assert!(!d.source.is_empty(), "{kind:?} has a size with no source");
            assert!(
                d.source.starts_with("convention:"),
                "a fallback must say it is a convention, not imply a citation: {}",
                d.source
            );
        }
        for name in named {
            let d = of(PropertyItemKind::HouseholdItem, name);
            assert!(!d.source.is_empty(), "{name} has a size with no source");
            assert!(
                !d.source.starts_with("convention:"),
                "{name} is a named thing and should carry a real figure, not a convention"
            );
        }
    }

    #[test]
    fn nothing_is_a_zero_or_a_negative_size() {
        for name in ["Ford Raptor 4x4 Ute", "Shovel", "Gem-D's Computer"] {
            let d = of(PropertyItemKind::ShedTool, name);
            assert!(
                d.length_m > 0.0 && d.width_m > 0.0 && d.height_m > 0.0,
                "{name} has a size with no extent: {d:?}"
            );
        }
    }

    #[test]
    fn a_television_is_its_diagonal() {
        // 75 inches is 1.905 m, and a 16:9 panel of that diagonal is
        // 1.660 m by 0.934 m. Pythagoras, not a lookup -- which is the
        // point of deriving it.
        let tv = of(
            PropertyItemKind::HouseholdItem,
            "Large TV (75in 8K, soundbar)",
        );
        let diagonal = (tv.length_m.powi(2) + tv.height_m.powi(2)).sqrt();
        assert!(
            (diagonal - 1.905).abs() < 0.001,
            "a 75in panel has a 1.905 m diagonal, not {diagonal:.3} m"
        );
        assert!((tv.length_m - 1.660).abs() < 0.002, "{:?}", tv);
        assert!((tv.height_m - 0.934).abs() < 0.002, "{:?}", tv);
    }

    #[test]
    fn the_named_machines_are_the_sizes_their_makers_publish() {
        let raptor = of(PropertyItemKind::Vehicle, "Ford Raptor 4x4 Ute");
        assert_eq!(
            (raptor.length_m, raptor.width_m, raptor.height_m),
            (5.381, 2.028, 1.922)
        );

        // The tractor is the thing that decides how big a shed door has
        // to be, and it is 3.6 m tall -- taller than the 3 m this estate
        // draws its buildings at, which is worth knowing and is exactly
        // the sort of fact an invented size would have hidden.
        let fendt = of(PropertyItemKind::Vehicle, "Fendt 1000 Vario");
        assert!(
            fendt.height_m > 3.5,
            "a Fendt 1000 is over 3.5 m tall: {fendt:?}"
        );
        assert!(fendt.longest_m() > 6.0);
    }

    #[test]
    fn a_name_beats_a_kind() {
        // The Raptor is a Vehicle and so is the fallback; the point of
        // the table is that the named one does not get the fallback.
        let named = of(PropertyItemKind::Vehicle, "Ford Raptor 4x4 Ute");
        let anonymous = of(PropertyItemKind::Vehicle, "Some Other Truck");
        assert_ne!(named, anonymous);
        assert!(anonymous.source.starts_with("convention:"));
    }
}
