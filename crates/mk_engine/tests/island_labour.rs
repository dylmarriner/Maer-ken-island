//! Phase 3 Task 3b: timed, energy-costed human work.

use mk_core::human::BiologicalSex;
use mk_engine::humans::needs::EffortFocus;
use mk_engine::humans::{AgentWorldObservation, HumanBeing};
use mk_engine::regional::labour::{
    day_energy_kcal, effort_for_met, ActiveTask, DigMaterial, LabourBody, LabourError, LabourTable,
    LabourTool, BASELINE_MET,
};
use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};

fn table() -> LabourTable {
    LabourTable::load_default().expect("reference packs")
}

fn library() -> ReferenceLibrary {
    ReferenceLibrary::load(&default_reference_dir()).unwrap()
}

fn founder() -> LabourBody {
    LabourBody::from_human(&HumanBeing::gem_d_founder())
}

#[test]
fn felling_takes_the_reference_time_for_the_tool() {
    let (t, lib) = (table(), library());
    let pack = lib
        .item(ReferenceDomain::Labour, "felling_time_30cm_tree")
        .unwrap()
        .table()
        .unwrap();
    for (tool, row) in [
        (LabourTool::StoneAxe, "stone_axe"),
        (LabourTool::SteelAxe, "steel_axe"),
        (LabourTool::CrosscutSaw, "two_person_crosscut_saw"),
        (LabourTool::Chainsaw, "chainsaw"),
    ] {
        let spec = t.fell_tree(tool, 30.0).unwrap();
        let minutes = spec.duration_s / 60.0;
        let (lo, hi) = (pack.get(row, "min").unwrap(), pack.get(row, "max").unwrap());
        assert!(
            (lo..=hi).contains(&minutes),
            "{row}: {minutes} min outside {lo}-{hi}"
        );
        assert_eq!((spec.output, spec.output_unit), (1.0, "tree"));
    }
    // Stone axes take 3-4 times as long as steel (Saraydar & Shimada 1971).
    let stone = t.fell_tree(LabourTool::StoneAxe, 30.0).unwrap().duration_s;
    let steel = t.fell_tree(LabourTool::SteelAxe, 30.0).unwrap().duration_s;
    assert!(
        (2.0..=4.5).contains(&(stone / steel)),
        "stone/steel {}",
        stone / steel
    );
    // A thicker trunk takes longer, in proportion to its cross-section.
    let big = t.fell_tree(LabourTool::SteelAxe, 60.0).unwrap().duration_s;
    assert!((big / steel - 4.0).abs() < 1e-9);
    // A chainsaw fells ~10 times faster than a steel axe (the packs'
    // midpoints: 2.5 against 25 minutes).
    let ratio = t
        .fell_tree(LabourTool::Chainsaw, 30.0)
        .unwrap()
        .output_per_hour()
        / t.fell_tree(LabourTool::SteelAxe, 30.0)
            .unwrap()
            .output_per_hour();
    assert!((9.0..=11.0).contains(&ratio), "chainsaw/axe {ratio}");
    // Only felling tools fell.
    assert!(t.fell_tree(LabourTool::GoldPan, 30.0).is_err());
}

#[test]
fn a_day_of_heavy_labour_costs_the_reference_energy() {
    let (t, lib) = (table(), library());
    let body = founder();
    let bmr = t.bmr_kcal_per_day(&body).unwrap();
    // The pack's own worked example: a 70 kg, 175 cm, 30 y man is 1,648.75 kcal/day.
    let example = LabourBody {
        sex: BiologicalSex::Male,
        age_years: 30.0,
        height_cm: 175.0,
        weight_kg: 70.0,
    };
    assert!((t.bmr_kcal_per_day(&example).unwrap() - 1_648.75).abs() < 1e-9);
    assert!((1_200.0..2_400.0).contains(&bmr), "founder BMR {bmr}");

    // A labour day with a realistic duty cycle: 3 h of felling at the
    // hand-axe MET, 2 h walking between trees, 3 h of light work (standing,
    // limbing, sharpening), 8 h of rest and 8 h of sleep.
    let felling = t.fell_tree(LabourTool::SteelAxe, 30.0).unwrap().met;
    let day = day_energy_kcal(
        &t,
        &body,
        &[
            (felling, 3.0),
            (t.met("walking_4_8_kmh_level").unwrap(), 2.0),
            (t.met("standing_light_work").unwrap(), 3.0),
            (1.15, 8.0),
            (t.met("sleeping").unwrap(), 8.0),
        ],
    )
    .unwrap();
    let pal = day / bmr;
    let pack = lib
        .item(ReferenceDomain::Humans, "physical_activity_level")
        .unwrap()
        .table()
        .unwrap();
    let (lo, hi) = (
        pack.get("vigorous_or_heavy_labour", "min").unwrap(),
        pack.get("vigorous_or_heavy_labour", "max").unwrap(),
    );
    assert!(
        (lo..=hi).contains(&pal),
        "heavy labour day is {pal:.2} x BMR; reference {lo}-{hi}"
    );
    // And under the sustained ceiling.
    let ceiling = lib
        .item(ReferenceDomain::Humans, "sustained_energy_ceiling")
        .unwrap()
        .range()
        .unwrap();
    assert!(pal <= ceiling.1);
    // A rest day costs well under a labour day.
    let rest = day_energy_kcal(&t, &body, &[(1.15, 16.0), (0.95, 8.0)]).unwrap();
    assert!(rest / bmr < 1.4 && rest < day);
}

#[test]
fn heavy_labour_drains_glucose_and_water_faster_than_rest() {
    let t = table();
    let human = HumanBeing::gem_d_founder();
    let observation = AgentWorldObservation {
        caloric_access: 0.0,
        hydration_access: 0.0,
        ambient_temperature_c: 15.0,
        ..AgentWorldObservation::default()
    };
    let dt_years = 1.0 / (365.25 * 24.0);
    let rest = human
        .needs
        .step(&observation, 1.0, EffortFocus::none(), dt_years);
    let work_met = t.fell_tree(LabourTool::SteelAxe, 30.0).unwrap().met;
    let work = human
        .needs
        .step(&observation, 1.0, effort_for_met(work_met), dt_years);
    let (rest_drop, work_drop) = (
        human.needs.glucose - rest.glucose,
        human.needs.glucose - work.glucose,
    );
    let multiple = effort_for_met(work_met).activity;
    assert!(multiple > 3.0 && multiple < 6.0, "{multiple}");
    assert!(
        (work_drop / rest_drop - multiple).abs() < 1e-6,
        "glucose drain ratio {}",
        work_drop / rest_drop
    );
    assert!(human.needs.hydration - work.hydration > human.needs.hydration - rest.hydration);
    // The old behaviour is unchanged: the default effort (activity 0) and
    // an explicit baseline of 1.0 give identical needs.
    let none = human
        .needs
        .step(&observation, 1.0, EffortFocus::default(), dt_years);
    assert_eq!(none.glucose, rest.glucose);
    assert_eq!(effort_for_met(BASELINE_MET).activity, 1.0);
    assert_eq!(
        effort_for_met(0.9).activity,
        1.0,
        "rest never drains less than baseline"
    );
}

#[test]
fn panning_yields_grams_per_day_inside_the_reference_range() {
    let (t, lib) = (table(), library());
    let rate = lib
        .item(ReferenceDomain::Labour, "gold_panning_throughput")
        .unwrap()
        .table()
        .unwrap();
    let recovery = lib
        .item(ReferenceDomain::Labour, "placer_gold_recovery")
        .unwrap()
        .table()
        .unwrap();
    let grade = 1.0; // g of gold per m3 in place
    for (tool, rate_row, recovery_row, coarse) in [
        (LabourTool::GoldPan, "gold_pan", "pan_coarse_gold", true),
        (LabourTool::GoldPan, "gold_pan", "pan_fine_gold", false),
        (
            LabourTool::SluiceBox,
            "sluice_box",
            "sluice_coarse_gold",
            true,
        ),
    ] {
        let spec = t.pan_gravel(tool, grade, coarse).unwrap();
        let (v_lo, v_hi) = (
            rate.get(rate_row, "min").unwrap(),
            rate.get(rate_row, "max").unwrap(),
        );
        let (r_lo, r_hi) = (
            recovery.get(recovery_row, "min").unwrap(),
            recovery.get(recovery_row, "max").unwrap(),
        );
        let grams_per_day = spec.output;
        assert!(
            (grade * v_lo * r_lo..=grade * v_hi * r_hi).contains(&grams_per_day),
            "{rate_row}/{recovery_row}: {grams_per_day} g/day"
        );
        assert_eq!(spec.duration_s, 8.0 * 3_600.0);
        // Yield is proportional to grade, and barren gravel yields nothing.
        let rich = t.pan_gravel(tool, 4.0, coarse).unwrap();
        assert!((rich.output - 4.0 * grams_per_day).abs() < 1e-12);
        assert_eq!(t.pan_gravel(tool, 0.0, coarse).unwrap().output, 0.0);
    }
    // A sluice processes more gravel than a pan.
    assert!(
        t.pan_gravel(LabourTool::SluiceBox, 1.0, true)
            .unwrap()
            .output
            > t.pan_gravel(LabourTool::GoldPan, 1.0, true).unwrap().output
    );
}

#[test]
fn walking_takes_real_time_and_heavy_loads_are_refused() {
    let t = table();
    let body = founder();
    // 10 km on the flat takes about 2 hours.
    let hours = t.walking_time_s(&body, 10_000.0, 0.0, 0.0).unwrap() / 3_600.0;
    assert!((1.8..=2.2).contains(&hours), "10 km took {hours:.2} h");
    // 600 m of ascent adds an hour (Naismith).
    let climb = t.walking_time_s(&body, 10_000.0, 600.0, 0.0).unwrap() / 3_600.0;
    assert!(
        (climb - hours - 1.0).abs() < 0.01,
        "climb added {:.2} h",
        climb - hours
    );
    // A load past the sustainable fraction slows the walk; within it does not.
    let sustainable = t.sustainable_load_kg(&body).unwrap();
    let light = t
        .walking_time_s(&body, 10_000.0, 0.0, 0.5 * sustainable)
        .unwrap();
    assert_eq!(light, hours * 3_600.0);
    let max = t.max_load_kg(&body).unwrap();
    let heavy = t.walking_time_s(&body, 10_000.0, 0.0, max).unwrap();
    assert!(heavy > light * 1.3, "heavy {heavy} vs light {light}");
    // Above the maximum the load is refused.
    assert!(matches!(
        t.walking_time_s(&body, 1_000.0, 0.0, max + 1.0),
        Err(LabourError::OverLoad { .. })
    ));
    assert!(t.check_carry(&body, max).is_ok());
    assert!(t.check_carry(&body, max + 0.1).is_err());
    assert!(max > sustainable && sustainable < body.weight_kg);
}

#[test]
fn work_takes_time_delivers_as_it_progresses_and_digging_follows_the_pack() {
    let (t, lib) = (table(), library());
    let spec = t.fell_tree(LabourTool::Chainsaw, 30.0).unwrap();
    let mut task = ActiveTask::new(spec);
    // Zero or negative time does nothing.
    assert_eq!(task.advance(0.0).delivered, 0.0);
    assert_eq!(task.advance(-5.0).worked_s, 0.0);
    let half = task.advance(spec.duration_s / 2.0);
    assert!((half.delivered - 0.5).abs() < 1e-12 && !half.finished);
    let rest = task.advance(10.0 * spec.duration_s);
    assert!(rest.finished && (half.delivered + rest.delivered - 1.0).abs() < 1e-12);
    assert!(
        (rest.worked_s - spec.duration_s / 2.0).abs() < 1e-9,
        "no work beyond the end"
    );
    assert_eq!(task.advance(60.0).delivered, 0.0);

    // Digging: a day's work moves the pack's volume.
    let dig = t.dig(DigMaterial::SoftSoil, 4.5).unwrap();
    let pack = lib
        .item(ReferenceDomain::Labour, "hand_excavation_rate")
        .unwrap()
        .table()
        .unwrap();
    let per_day = dig.output / (dig.duration_s / (8.0 * 3_600.0));
    assert!(
        (pack.get("soft_soil", "min").unwrap()..=pack.get("soft_soil", "max").unwrap())
            .contains(&per_day)
    );
    assert!(
        t.dig(DigMaterial::SoftRock, 1.0).unwrap().duration_s
            > t.dig(DigMaterial::SoftSoil, 1.0).unwrap().duration_s
    );
    // A timber-frame house of 100 m2 takes the pack's person-hours; hand tools double it.
    let power = t.build_timber_frame(100.0, true).unwrap();
    let hand = t.build_timber_frame(100.0, false).unwrap();
    assert!((power.duration_s / 3_600.0 / 100.0 - 22.0).abs() < 1e-9);
    assert!((hand.duration_s / power.duration_s - 2.0).abs() < 1e-12);
    // Hard-rock mining is slow: about half a tonne a day.
    let mine = t.mine_hard_rock(0.5).unwrap();
    assert!((mine.duration_s / 3_600.0 - 8.0).abs() < 1e-9);
}
