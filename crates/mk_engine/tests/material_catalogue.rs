//! Phase 3 Task 2: the material catalogue -- every resource and material
//! the island can yield, with a cited density and hardness.

use mk_engine::materials::catalogue::{
    CatalogueItem, DensityBasis, MaterialCatalogue, MaterialCategory, ALL_ITEMS,
};
use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};

fn catalogue() -> MaterialCatalogue {
    MaterialCatalogue::load_default().expect("the catalogue loads")
}

#[test]
fn every_material_has_its_row_and_every_row_a_material() {
    let lib = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    let cat = catalogue();
    // Things with no single density of their own, and why: composites,
    // and materials whose density is another's.
    let no_density = [
        CatalogueItem::Herbs,
        CatalogueItem::Rope,
        CatalogueItem::CutGem,
        CatalogueItem::PolishedStone,
        CatalogueItem::Tools,
        CatalogueItem::Weapons,
        CatalogueItem::Structures,
    ];
    for item in ALL_ITEMS {
        let density = cat.density_kg_m3(item);
        if no_density.contains(&item) {
            assert_eq!(density, None, "{item:?} should have no single density");
        } else {
            let rho = density.unwrap_or_else(|| panic!("{item:?} has no density row"));
            assert!(rho > 0.0 && rho.is_finite(), "{item:?}: {rho}");
        }
    }
    // And the other direction: a row the code does not know is a material
    // the pack describes and the island cannot hold.
    for (key, basis) in [
        ("material_density_intrinsic", DensityBasis::Intrinsic),
        ("material_density_bulk", DensityBasis::Bulk),
        ("mohs_hardness", DensityBasis::Intrinsic),
    ] {
        let table = lib
            .item(ReferenceDomain::Geology, key)
            .unwrap()
            .table()
            .unwrap();
        let column = if key == "mohs_hardness" {
            "mohs"
        } else {
            "density_kg_m3"
        };
        for (row, _) in table.column(column).unwrap() {
            let item = CatalogueItem::from_key(row)
                .unwrap_or_else(|| panic!("{key} row {row} is not a catalogue material"));
            if key != "mohs_hardness" {
                assert_eq!(item.density_basis(), basis, "{row} is in the wrong table");
            }
        }
    }
    let keys: std::collections::BTreeSet<&str> = ALL_ITEMS.iter().map(|i| i.key()).collect();
    assert_eq!(keys.len(), ALL_ITEMS.len(), "two materials share a key");
}

#[test]
fn the_plan_s_required_materials_are_all_here() {
    // `docs/superpowers/plans/2026-10-01-island-life-property-humans.md`,
    // Task 2's table, category by category.
    use CatalogueItem::*;
    use MaterialCategory as C;
    let required: [(C, &[CatalogueItem]); 14] = [
        (
            C::PreciousMetalOre,
            &[GoldOre, PlacerGold, SilverOre, PlatinumGroupConcentrate],
        ),
        (
            C::BaseMetalOre,
            &[
                IronOre,
                MagnetiteOre,
                Ironsand,
                CopperOre,
                TinOre,
                LeadOre,
                ZincOre,
                NickelOre,
                Chromite,
                TungstenOre,
                Molybdenite,
                Cinnabar,
                Uraninite,
                LithiumOre,
                Bauxite,
            ],
        ),
        (
            C::Gem,
            &[
                Diamond, Ruby, Sapphire, Emerald, Topaz, Tourmaline, Garnet, Opal, Nephrite,
            ],
        ),
        (
            C::Crystal,
            &[QuartzCrystal, Amethyst, Agate, Chalcedony, Obsidian],
        ),
        (
            C::Stone,
            &[
                Granite, Basalt, Limestone, Marble, Sandstone, Slate, Flint, Pumice, Stone,
            ],
        ),
        (C::Sediment, &[Clay, Kaolin, Sand, SilicaSand, Gravel, Peat]),
        (
            C::IndustrialMineral,
            &[
                Sulfur,
                Gypsum,
                PhosphateRock,
                Feldspar,
                Mica,
                Graphite,
                Zeolite,
                Serpentine,
                Kyanite,
                Pyrite,
            ],
        ),
        (C::FossilFuel, &[Coal, CrudeOil, NaturalGas]),
        (C::Salt, &[RockSalt, SeaSalt]),
        (
            C::Plant,
            &[
                Wood, Resin, Fibre, Herbs, Fruit, Nuts, Fungi, PlantFood, Honey, Beeswax, Seaweed,
            ],
        ),
        (C::Animal, &[Meat, Hide, Bone, Feather]),
        (C::Marine, &[Fish, Shellfish]),
        (C::Water, &[Water]),
        (
            C::Processed,
            &[
                Planks,
                Masonry,
                Rope,
                Fuel,
                Charcoal,
                Quicklime,
                Glass,
                GoldIngot,
                SilverIngot,
                CopperIngot,
                TinIngot,
                LeadIngot,
                ZincIngot,
                IronIngot,
                Steel,
                Bronze,
                Brass,
                CutGem,
                PolishedStone,
                Tools,
                Weapons,
                Structures,
            ],
        ),
    ];
    for (category, items) in required {
        for item in items {
            assert_eq!(item.category(), category, "{item:?}");
        }
    }
    let listed: usize = required.iter().map(|(_, items)| items.len()).sum();
    assert_eq!(
        listed,
        ALL_ITEMS.len(),
        "a material outside the plan's table"
    );
}

#[test]
fn the_numbers_behave_like_matter() {
    let cat = catalogue();
    let rho = |item| cat.density_kg_m3(item).unwrap();
    let mohs = |item| cat.mohs(item).unwrap();
    use CatalogueItem::*;
    // Diamond is the hardest thing on the scale, talc-soft molybdenite near
    // the bottom, quartz the 7 everything else is measured against.
    assert_eq!(mohs(Diamond), 10.0);
    assert!(ALL_ITEMS
        .iter()
        .filter_map(|i| cat.mohs(*i))
        .all(|h| (1.0..=10.0).contains(&h)));
    assert_eq!(mohs(QuartzCrystal), 7.0);
    assert!(mohs(Ruby) > mohs(Emerald) && mohs(Emerald) > mohs(QuartzCrystal));
    // A smelted metal is denser than the ore it came from, and gold is the
    // densest of the common ones short of platinum.
    for (ore, metal) in [
        (IronOre, IronIngot),
        (CopperOre, CopperIngot),
        (TinOre, TinIngot),
        (LeadOre, LeadIngot),
        (ZincOre, ZincIngot),
        (SilverOre, SilverIngot),
        (GoldOre, GoldIngot),
    ] {
        assert!(rho(metal) > rho(ore), "{metal:?} against {ore:?}");
    }
    assert!(rho(GoldIngot) > rho(LeadIngot) && rho(PlatinumGroupConcentrate) > rho(GoldIngot));
    // Wood and wax float; stone, salt and metal sink; natural gas is a gas.
    for floats in [Wood, Beeswax, Pumice, CrudeOil] {
        assert!(rho(floats) < rho(Water), "{floats:?}");
    }
    for sinks in [Granite, Basalt, RockSalt, Glass, Steel, Bone] {
        assert!(rho(sinks) > rho(Water), "{sinks:?}");
    }
    assert!(rho(NaturalGas) < 1.0);
    // Loose sediment holds less than the rock it came from.
    assert!(rho(Sand) < rho(Sandstone) && rho(SilicaSand) < rho(QuartzCrystal));
    // A litre of iron weighs 7.874 kg; a cubic metre of loose sand 1.6 t.
    assert!((cat.mass_kg(IronIngot, 1e-3).unwrap() - 7.874).abs() < 1e-9);
    assert!((cat.mass_kg(Sand, 1.0).unwrap() - 1_600.0).abs() < 1e-9);
}

#[test]
fn every_mineral_material_comes_out_of_the_ground_somewhere() {
    use mk_engine::regional::deposits::DepositKind;
    use CatalogueItem::*;
    use MaterialCategory as C;
    // Geological materials that come from somewhere other than a primary
    // deposit, and where: placers are secondary (eroded out of a primary
    // deposit and carried by rivers and surf), rubble is any rock, sea salt
    // is the sea's, and loose sediment -- clay, sand, gravel, peat -- is
    // dug from soils, river beds, beaches and bogs (kaolin, the one
    // sediment a deposit model yields, comes with building stone). Bauxite
    // forms only by tropical laterite weathering, which a mid-latitude
    // island's climate does not do.
    let elsewhere = [
        PlacerGold, Ironsand, Stone, SeaSalt, Bauxite, Clay, Sand, SilicaSand, Gravel, Peat,
    ];
    let yielded: std::collections::BTreeSet<CatalogueItem> = DepositKind::ALL
        .iter()
        .flat_map(|kind| kind.yields().iter().copied())
        .collect();
    for item in ALL_ITEMS {
        let geological = matches!(
            item.category(),
            C::PreciousMetalOre
                | C::BaseMetalOre
                | C::Gem
                | C::Crystal
                | C::Stone
                | C::Sediment
                | C::IndustrialMineral
                | C::FossilFuel
                | C::Salt
        );
        if !geological {
            assert!(
                !yielded.contains(&item),
                "{item:?} is not geological but a deposit yields it"
            );
            continue;
        }
        assert!(
            yielded.contains(&item) != elsewhere.contains(&item),
            "{item:?}: yielded {}, listed elsewhere {}",
            yielded.contains(&item),
            elsewhere.contains(&item)
        );
    }
    // Each deposit yields something, its ore first and no material twice.
    for kind in DepositKind::ALL {
        let y = kind.yields();
        assert!(!y.is_empty(), "{kind:?}");
        let unique: std::collections::BTreeSet<_> = y.iter().collect();
        assert_eq!(unique.len(), y.len(), "{kind:?} lists a material twice");
    }
}
