# Scientific validation plan

Scientific validity is a tested requirement. Each method in `apsi-science` has to show its reference, assumptions, a known-answer test, input rejection and its limitations. This table records the current state honestly.

| Method | Reference | Validation evidence today | Gap |
|---|---|---|---|
| NDVI, EVI, SAVI, NDWI, NDMI, GNDVI | Rouse et al. 1974; Huete 1988, 2002; Gao 1996; McFeeters 1996 | known-answer tests (`known_values`); rejection of reflectance outside [−0.5, 1.5] (tolerating atmospheric-correction overshoot) and of a zero denominator (`nodata_zero_denominator_and_missing_bands`) | no comparison against a reference raster product (e.g. a Sentinel-2 L2A scene) |
| FAO-56 Penman–Monteith ET₀ | Allen et al. 1998, FAO Irrigation and Drainage Paper 56 | order-of-magnitude test against Example 17 | the exact worked examples should be reproduced to the published precision |
| Growing degree days | standard averaging method with cap | unit tests including cap and base | – |
| USDA soil texture class | USDA texture triangle | class boundary tests | – |
| Yield moisture correction | standard dry-matter balance | unit tests | – |
| Welch t-test, CIs, Benjamini–Hochberg, Bonferroni, OLS | standard formulas | property tests (BH-adjusted p-values are ≥ the raw p and ≤ 1; the t CDF is monotone); known-answer tests against t tables | cross-check against R or SciPy outputs on fixed datasets |
| Geodesy (haversine, spherical area, Web Mercator) | spherical Earth, R = 6,371,008.8 m | round-trip property test to 1e-9° | area error versus ellipsoidal computation is not quantified |
| Unit conversion and dimensional analysis | NIST SP 811 (2008) | round-trip property test; cross-check with `agronomy::convert`; dimension mismatch rejected | – |
| AgriCUDA CPU fallback (NDVI) | same formula as above | used as the accelerator reference within the job tolerance | – |

## Procedure for adding or changing a method

1. Cite the primary reference in the function's doc comment, with units for every input and output.
2. Add a known-answer test from the reference (a worked example, a published table or an independent implementation), with the tolerance justified.
3. Add rejection tests for out-of-domain inputs. Return `ScienceError` rather than NaN, infinity or a clamped value.
4. Document assumptions and limitations in the doc comment and in this table.
5. Results must record their provenance: workflow runs store the definition hash and data fingerprint, and records store `transformation` provenance.
6. Use reference datasets only when their licence permits redistribution in tests. Otherwise store a checksum and a download instruction, never the data.

## Ground truth

Generated data, AI outputs and heuristic scores are never used as scientific ground truth. Validation datasets must be observational, with documented provenance and licence.
