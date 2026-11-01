// Display only values actually returned by /api/cell. These are annual
// climatology and production, not live weather or species observations.
export function cellEnvironmentDetails(cell) {
  const details = [];
  if (typeof cell.biome === 'string' && cell.biome.trim()) {
    details.push(`Biome: ${cell.biome}`);
  }
  if (Number.isFinite(cell.mean_temperature_c)) {
    details.push(`Mean annual air temperature: ${cell.mean_temperature_c.toFixed(1)} °C`);
  }
  if (Number.isFinite(cell.annual_rain_mm) && cell.annual_rain_mm >= 0) {
    details.push(`Annual precipitation: ${cell.annual_rain_mm.toFixed(0)} mm/year`);
  }
  // The island's producer NPP field is terrestrial. Sea cells report zero
  // because marine production is not represented, not because it is absent.
  if (cell.land && Number.isFinite(cell.npp_kgc_m2_yr) && cell.npp_kgc_m2_yr >= 0) {
    details.push(`Land net primary production: ${cell.npp_kgc_m2_yr.toFixed(2)} kg C/m²/year`);
  }
  return details;
}
