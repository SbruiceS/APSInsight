# Coordinate reference system policy

1. **Storage and exchange CRS:** WGS 84 geographic coordinates, `EPSG:4326`, in **longitude, latitude** order (GeoJSON RFC 7946 order), in decimal degrees. The record attribute `location` is `{ "lon": .., "lat": .. }`; `apsi_science::geo::LonLat::check` rejects longitude outside ±180, latitude outside ±90 and non-finite values.
2. **Declared, never assumed:** any other CRS must be stated explicitly as `EPSG:<code>` or `OGC:CRS84` (the ASPF contracts enforce this with `check_crs`). Data whose CRS is unknown is rejected at the contract boundary rather than guessed.
3. **Implemented transformations:** WGS 84 ↔ spherical Web Mercator (`EPSG:3857`). Latitude is clamped to ±85.051129°, and the round trip is property-tested to 1e-9°. No other reprojection is implemented. Do not ingest projected data (UTM and similar) until a reviewed transformation library such as PROJ is integrated and its results are tested against reference points.
4. **Geometry validity:** rings must have at least four points, be closed, and contain only finite, in-range coordinates (`geo::check_ring`). Area uses a spherical-Earth approximation (R = 6,371,008.8 m). Its error grows with polygon size and latitude, and it is documented as an estimate.
5. **Resolution:** raster resolution and nodata values are attributes of raster metadata records and must be carried through every derived product. Raster processing is not implemented ([STATUS](../docs/STATUS.md)).
6. **Precision:** coordinates are not rounded on storage. Exports reproduce stored values exactly.
7. **Axis order in contracts:** `SpatialContext.bbox` is `[min_x, min_y, max_x, max_y]` in the axis order of the declared CRS.
