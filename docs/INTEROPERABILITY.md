# Interoperability

| Format / standard | Direction | Mapping | What is preserved |
|---|---|---|---|
| APSInsight JSON (`apsinsight.export` v1) | export and import | `{header, records[]}` | everything, including history-independent provenance |
| NDJSON | export and import | header line, then one record per line | everything |
| CSV, RFC 4180, UTF-8 | export and import | fixed columns; `attributes` and `provenance` as JSON cells; `tags` joined with `\|` | lossless round trip (tested with commas, quotes and newlines) |
| Plain CSV (third-party) | import | `name` required; recognised `tags`, `source_system`, `source_id`, `lifecycle`; every other column becomes an attribute (numeric-looking cells become numbers) | optional `FieldMapping` (`rename`, `defaults`, `drop`) |
| GeoJSON, RFC 7946 | export | `Feature` per record; `geometry` from the `location` attribute (a `Point` in lon/lat order, or `null`); every other attribute in `properties` | the APSInsight header under the `apsinsight` key |
| WGS 84 (EPSG:4326) ↔ Web Mercator (EPSG:3857) | compute `geo.web_mercator` | spherical Mercator, latitude limited to ±85.051129° | round trip to 1e-9° (property test) |
| Units | compute `agro.convert` | exact NIST SP 811 factors for lb/ac, kg/ha, t/ha, ac, ha, in, mm, °F and °C | conversions are exact rationals; unsupported pairs are errors, never guessed |
| Units with dimensional analysis | compute `units.convert` | any two units of the same dimension in [UNITS.md](UNITS.md); mismatched dimensions are rejected | exact factors; absolute temperatures and differences are distinguished |
| ASPF contracts v1.0 | `apsi-contracts` | JSON wire examples in `crates/apsi-contracts/contracts/v1/` | units, CRS, missingness, uncertainty and provenance are mandatory fields |
| CycloneDX 1.5 | `node tools/sbom.mjs` | SBOM per ecosystem (cargo, npm) | licence expressions as declared |
| Prometheus text format 0.0.4 | `/metrics` | counters and summaries | – |

## Semantic loss is never silent

- An unknown CSV column becomes an attribute rather than being dropped.
- A GeoJSON export of a record without a location has `geometry: null`; the record is still included.
- An import that cannot be represented, such as an archived lifecycle or an invalid row, is reported per row; it is never coerced.
