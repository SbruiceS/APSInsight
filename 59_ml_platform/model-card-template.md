# Model card: `<model name>` `<version>`

> Required for every model whose output reaches APSInsight, including outputs that arrive through the KutajKutambi AI contract. `ModelMeta.evaluation_ref` must point to a filled-in copy of this card. A model without one is not deployed.

## 1. Identity

| Field | Value |
|---|---|
| Name / version | |
| Owner (team, contact) | |
| Output kind | prediction / AI interpretation / recommendation (never an observation) |
| Licence of the model and its weights | |
| Training code commit / environment manifest | |

## 2. Intended use

- Decisions it supports:
- Users:
- **Out of scope:** uses it must not be relied on for.

## 3. Data

| Split | Source dataset ids and versions | Period | Region | Rows | Licence |
|---|---|---|---|---|---|
| train | | | | | |
| validation | | | | | |
| test (held out, never tuned on) | | | | | |

- Splitting method: temporal, spatial-block or random. Justify the choice; agricultural data is spatially and temporally autocorrelated, so random splits usually overstate skill.
- Known biases, gaps and synthetic data used (synthetic data must be labelled and never counted as observations).

## 4. Evaluation

| Metric | Value on test | Baseline (e.g. climatology or persistence) | Uncertainty (CI / bootstrap) |
|---|---|---|---|
| | | | |

- Performance by subgroup (crop, region, season, farm size):
- Calibration of the reported uncertainty (coverage of the prediction intervals):

## 5. Uncertainty and limitations

- How uncertainty is produced (`Uncertainty.method`):
- Conditions where the model is known to fail:

## 6. Human oversight

- Every recommendation needs human approval (`requires_human_approval: true`); the approval is recorded as a provenance event.
- Feedback and outcome tracking:

## 7. Changes

| Version | Date | Change | Re-evaluated? |
|---|---|---|---|
