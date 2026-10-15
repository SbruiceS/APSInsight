import { IconName } from '../shared/icons';

/**
 * Worked example inputs for every function in the science compute registry
 * (`GET /api/v1/compute`). Values are synthetic and illustrative only.
 */
export const LAB_EXAMPLES: Record<string, unknown> = {
  'indices.registry': {},
  'indices.compute': { index: 'ndvi', pixel: { red: 0.08, nir: 0.42 } },
  'indices.summarize': {
    index: 'ndvi',
    nodata: -9999,
    pixels: [
      { red: 0.08, nir: 0.42 },
      { red: 0.1, nir: 0.38 },
      { red: -9999, nir: 0.4 },
      { red: 0.12, nir: 0.3 },
    ],
  },
  'geo.distance': { a: { lon: 77.209, lat: 28.6139 }, b: { lon: 72.8777, lat: 19.076 } },
  'geo.area': { ring: square() },
  'geo.contains': { ring: square(), point: { lon: 77.2005, lat: 28.6005 } },
  'geo.bbox': { ring: square() },
  'geo.web_mercator': { lon: 77.209, lat: 28.6139 },
  'stats.mean': [4.1, 4.6, 5.2, 4.8, 5.0],
  'stats.describe': [4.1, 4.6, 5.2, 4.8, 5.0, 3.9, 4.4],
  'stats.mean_ci': { values: [4.1, 4.6, 5.2, 4.8, 5.0], confidence: 0.95 },
  'stats.welch_t_test': { a: [5.1, 5.4, 4.9, 5.6, 5.2], b: [4.2, 4.6, 4.4, 4.9, 4.1] },
  'stats.benjamini_hochberg': { p_values: [0.001, 0.008, 0.039, 0.041, 0.27] },
  'stats.bonferroni': { p_values: [0.001, 0.008, 0.039, 0.041, 0.27] },
  'stats.linear_regression': { x: [0, 40, 80, 120, 160], y: [2.1, 3.4, 4.6, 5.3, 5.6] },
  'stats.forecast_errors': { actual: [12, 15, 14, 18], predicted: [11, 16, 13, 17] },
  'ts.gaps': {
    interval: 900,
    tolerance: 1.5,
    observations: [
      { t: 0, v: 21.2 },
      { t: 900, v: 21.6 },
      { t: 3600, v: 22.9 },
      { t: 4500, v: 23.1 },
    ],
  },
  'ts.late_arrivals': {
    observations: [
      { t: 0, v: 1 },
      { t: 900, v: 2 },
      { t: 600, v: 3 },
      { t: 1800, v: 4 },
    ],
  },
  'ts.downsample': {
    width: 3600,
    agg: 'mean',
    observations: [
      { t: 0, v: 20.5 },
      { t: 1800, v: 21.5 },
      { t: 3600, v: 24 },
      { t: 5400, v: 26 },
    ],
  },
  'agro.gdd': { tmin: 12, tmax: 31, base: 10, cap: 30 },
  'agro.et0': { tmin_c: 18, tmax_c: 32, rh_mean: 55, wind_2m: 2.1, net_radiation: 14.5, elevation_m: 216 },
  'agro.water_balance': {
    capacity_mm: 120,
    initial_mm: 80,
    days: [
      { rain_mm: 0, irrigation_mm: 0, etc_mm: 5.2 },
      { rain_mm: 12, irrigation_mm: 0, etc_mm: 4.1 },
      { rain_mm: 0, irrigation_mm: 25, etc_mm: 5.6 },
    ],
  },
  'agro.usda_texture': { sand: 40, silt: 40, clay: 20 },
  'agro.moisture_correction': { yield: 9.2, moisture_pct: 21, standard_pct: 15.5 },
  'agro.convert': { value: 150, from: 'lb/ac', to: 'kg/ha' },
  'units.convert': { value: 4.2, from: 't/ha', to: 'kg/ha' },
  'agro.bushels_to_t_ha': { bu_ac: 180, lb_per_bushel: 56 },
  'econ.margin': {
    area_ha: 40,
    yield_per_ha: 5.5,
    price_per_unit: 210,
    costs: [
      { item: 'seed', per_ha: 120 },
      { item: 'fertiliser', per_ha: 260 },
      { item: 'machinery', per_ha: 150, fixed: true },
    ],
  },
  'econ.emissions': {
    activities: [
      { name: 'diesel (L)', amount: 1200, factor: 2.68, factor_uncertainty: 0.05 },
      { name: 'urea N (kg)', amount: 3000, factor: 5.9, factor_uncertainty: 0.3 },
    ],
  },
};

function square() {
  return [
    { lon: 77.2, lat: 28.6 },
    { lon: 77.201, lat: 28.6 },
    { lon: 77.201, lat: 28.601 },
    { lon: 77.2, lat: 28.601 },
    { lon: 77.2, lat: 28.6 },
  ];
}

export interface LabFamily {
  prefix: string;
  icon: IconName;
  label: 'lab.indices' | 'lab.geo' | 'lab.stats' | 'lab.ts' | 'lab.agro' | 'lab.units' | 'lab.econ' | 'lab.other';
}

export const LAB_FAMILIES: LabFamily[] = [
  { prefix: 'indices', icon: 'leaf', label: 'lab.indices' },
  { prefix: 'agro', icon: 'sprout', label: 'lab.agro' },
  { prefix: 'units', icon: 'ruler', label: 'lab.units' },
  { prefix: 'geo', icon: 'map', label: 'lab.geo' },
  { prefix: 'stats', icon: 'chart', label: 'lab.stats' },
  { prefix: 'ts', icon: 'clock', label: 'lab.ts' },
  { prefix: 'econ', icon: 'coins', label: 'lab.econ' },
];

export function familyOf(fn: string): LabFamily {
  const prefix = fn.split('.')[0];
  return LAB_FAMILIES.find((f) => f.prefix === prefix) ?? { prefix, icon: 'flask', label: 'lab.other' };
}
