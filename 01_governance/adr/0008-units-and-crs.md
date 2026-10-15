# ADR-0008: Quantities carry units and dimensions; geometries carry a CRS

- Status: Accepted
- Date: 2026-10-15

## Decision

- Units are case-sensitive UCUM-style codes. The catalogue lives in `apsi_science::units` and is published as [docs/UNITS.md](../../docs/UNITS.md).
- `Quantity` stores a value in coherent SI units together with its dimension. Adding, subtracting or converting across dimensions is rejected. Absolute temperatures cannot be added, multiplied, divided or scaled; differences can.
- Currencies are not units. Converting between them needs a dated exchange rate.
- The contracts require a unit on every quantity. Dimensionless values are written `1` and never left empty. A missing value is `null` with quality `missing`, never a sentinel number.
- Geometry is stored and exchanged in WGS 84 (`EPSG:4326`, lon/lat order). Any other CRS must be declared explicitly. See the [CRS policy](../../03_domain_model/crs-policy.md).

## Consequences

- Schema migrations that change units use the `scale` and `set_unit` operations together, so the history records the conversion.
- Neither free-text units nor CRS names are accepted at contract boundaries.
