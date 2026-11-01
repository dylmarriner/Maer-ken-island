import test from 'node:test';
import assert from 'node:assert/strict';
import { cellEnvironmentDetails } from '../static/cell-details.mjs';

test('shows the real annual climatology and land productivity fields', () => {
  assert.deepEqual(cellEnvironmentDetails({
    land: true, biome: 'TemperateForest', mean_temperature_c: 11.26,
    annual_rain_mm: 9100.4, npp_kgc_m2_yr: 0.857,
  }), [
    'Biome: TemperateForest',
    'Mean annual air temperature: 11.3 °C',
    'Annual precipitation: 9100 mm/year',
    'Land net primary production: 0.86 kg C/m²/year',
  ]);
});

test('does not present unmodelled marine NPP as zero biological activity', () => {
  assert.deepEqual(cellEnvironmentDetails({
    land: false, biome: 'ShallowOcean', mean_temperature_c: 9.8,
    annual_rain_mm: 850, npp_kgc_m2_yr: 0,
  }), [
    'Biome: ShallowOcean',
    'Mean annual air temperature: 9.8 °C',
    'Annual precipitation: 850 mm/year',
  ]);
});

test('omits absent, malformed and non-finite measurements without inventing numbers', () => {
  assert.deepEqual(cellEnvironmentDetails({
    land: true, biome: '', mean_temperature_c: null,
    annual_rain_mm: Number.POSITIVE_INFINITY, npp_kgc_m2_yr: -1,
  }), []);
  assert.deepEqual(cellEnvironmentDetails({land: true}), []);
});
