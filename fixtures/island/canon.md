# Island canon (`canon.json`)

Earth-like Marr'Kena, loaded with `CanonLocked::load` and checked with
`mk_core::canon::validator::validate_physical_consistency`. Upstream's
`CanonLocked::default()` and its identity check `validate_canon` are unchanged.

| Field | Value | Source |
|---|---|---|
| planet_radius_m, rotation_period_s, obliquity_deg, orbital_eccentricity | 1.9113e7 m, 129,600 s, 27°, 0.031 | kept from Marr'Kena canon |
| star_mass_kg, star_luminosity_w, star_radius_m | 2.2270864e30 kg, 5.1678e26 W, 7.51356e8 m | kept from Marr'Kena canon |
| surface_gravity_m_s2 | 9.80665 | Earth standard gravity (owner decision) |
| planet_mass_kg | 5.367508e25 (8.99 M⊕) | g·R²/G with G = 6.67430e-11 (CODATA 2018) |
| semi_major_axis_m | 1.738276e11 (1.162 AU) | √(L / 4πS) for S = 1,361 W/m², refined by Kepler from the whole-second period |
| orbital_period_s | 37,349,219 (288.19 local days) | Kepler's third law, G(M★ + M) |
| solar_constant_w_m2 | 1,361.0 | Earth (Kopp & Lean 2011) |
| McKenz | 30 local days; a = 1.1156e9 m | period kept; a from Kepler with upstream mass ratio 0.0123 |
| Hahn | 48 local days; a = 1.5225e9 m | shortened from 90 local days to stay inside 0.49 R_Hill (Domingos et al. 2006) with margin and away from 2:1 / 3:2 resonance |
| sea_level_pressure_pa | 101,325 | Earth standard atmosphere |
| ref_temp_k, albedo_baseline | 288 K, 0.306 | Earth global mean surface temperature and Bond albedo |
| ocean_fraction, mean_salinity_psu | 0.708, 35.0 | Earth |
| all other fields | unchanged | Marr'Kena canon (biosphere reference stocks are planetary and unused at island scale) |

**Declared exception:** bulk density 1,835 kg/m³, below the rocky minimum
(~3,300 kg/m³). Earth gravity at three times Earth's radius requires it; the
crust, mantle and tectonics are modelled with Earth-like rock properties. The
validator reports it with severity `Declared`.
