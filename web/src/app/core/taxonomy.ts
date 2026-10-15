import { I18nKey } from './i18n';
import { IconName } from '../shared/icons';

// Groups the 100 register domains into areas an agricultural researcher or data
// engineer recognises, each with an icon and a colour tone. scripts/check-ui.mjs
// fails if a domain in register/catalog.json is missing here or listed twice.

export type Tone = 'leaf' | 'soil' | 'wheat' | 'sky' | 'clay' | 'berry' | 'slate';

export interface DomainGroup {
  id: string;
  label: I18nKey;
  icon: IconName;
  tone: Tone;
  domains: string[];
}

export const GROUPS: DomainGroup[] = [
  {
    id: 'field',
    label: 'group.field',
    icon: 'wheat',
    tone: 'leaf',
    domains: [
      'field-and-farm-model',
      'crop-and-cultivar-catalogue',
      'crop-calendars-and-phenology',
      'crop-yield-and-production',
      'pest-and-disease-intelligence',
      'plant-phenotyping',
    ],
  },
  {
    id: 'soilWater',
    label: 'group.soilWater',
    icon: 'soil',
    tone: 'soil',
    domains: [
      'soil-profile-and-classification',
      'soil-chemistry-and-fertility',
      'soil-physics-and-mechanics',
      'water-resources-and-hydrology',
      'irrigation-engineering',
    ],
  },
  {
    id: 'climate',
    label: 'group.climate',
    icon: 'sun',
    tone: 'wheat',
    domains: ['weather-observations', 'weather-forecasts-and-climate', 'environmental-monitoring', 'sustainability-and-carbon', 'risk-and-resilience-analytics'],
  },
  {
    id: 'geo',
    label: 'group.geo',
    icon: 'satellite',
    tone: 'sky',
    domains: ['geospatial-foundations', 'raster-and-satellite-imagery', 'remote-sensing-indices', 'gis-map-workspace', 'satellite-and-agency-integrations'],
  },
  {
    id: 'sensors',
    label: 'group.sensors',
    icon: 'tractor',
    tone: 'clay',
    domains: ['sensor-registry-and-telemetry', 'iot-event-processing', 'agricultural-equipment-data', 'edge-and-offline-operations'],
  },
  {
    id: 'life',
    label: 'group.life',
    icon: 'dna',
    tone: 'berry',
    domains: [
      'laboratory-and-sample-management',
      'genomics-and-sequence-metadata',
      'transcriptomics-and-gene-expression',
      'proteomics-and-metabolomics',
      'biological-pathways-and-ontology',
      'experimental-design',
    ],
  },
  {
    id: 'analytics',
    label: 'group.analytics',
    icon: 'chart',
    tone: 'sky',
    domains: [
      'statistics-and-inference',
      'causal-and-comparative-analysis',
      'machine-learning-lifecycle',
      'forecasting-and-prediction',
      'computer-vision',
      'anomaly-and-fault-detection',
      'decision-support-and-recommendations',
      'optimization-and-simulation',
      'agricultural-decision-provenance',
      'natural-language-assistance',
      'knowledge-graph-and-semantics',
      'search-and-discovery',
    ],
  },
  {
    id: 'research',
    label: 'group.research',
    icon: 'microscope',
    tone: 'leaf',
    domains: [
      'scientific-workflow-engine',
      'research-notebooks-and-projects',
      'visualization-and-dashboards',
      'reporting-and-publishing',
      'open-science-and-reproducibility',
      'scientific-method-governance',
      'public-agricultural-datasets',
    ],
  },
  {
    id: 'data',
    label: 'group.data',
    icon: 'layers',
    tone: 'sky',
    domains: [
      'agricultural-data-catalogue',
      'data-ingestion',
      'data-validation-and-quality',
      'data-transformation',
      'data-lineage-and-provenance',
      'data-versioning-and-reproducibility',
      'data-access-and-governance',
      'data-interoperability-standards',
    ],
  },
  {
    id: 'storage',
    label: 'group.storage',
    icon: 'database',
    tone: 'slate',
    domains: ['relational-data-services', 'object-and-file-storage', 'columnar-and-analytical-storage', 'time-series-data'],
  },
  {
    id: 'economics',
    label: 'group.economics',
    icon: 'coins',
    tone: 'wheat',
    domains: ['economic-and-farm-finance-analytics', 'agricultural-economics-and-markets', 'supply-chain-and-traceability'],
  },
  {
    id: 'platform',
    label: 'group.platform',
    icon: 'shield',
    tone: 'clay',
    domains: [
      'platform-governance-and-configuration',
      'identity-and-access-management',
      'organization-and-tenancy',
      'authentication',
      'authorization-and-policy',
      'audit-and-compliance',
      'security-operations',
      'privacy-and-data-rights',
      'multi-tenant-billing-and-quotas',
      'administration-and-support',
    ],
  },
  {
    id: 'engineering',
    label: 'group.engineering',
    icon: 'sliders',
    tone: 'slate',
    domains: [
      'api-gateway-and-contracts',
      'realtime-communications',
      'background-job-execution',
      'scheduling-and-resource-management',
      'notifications-and-subscriptions',
      'external-data-connectors',
      'testing-and-quality-engineering',
      'performance-engineering',
      'reliability-and-resilience',
      'deployment-and-infrastructure',
      'developer-experience-and-sdks',
      'cli-and-automation',
      'plugin-and-extension-system',
      'documentation-and-knowledge',
      'licensing-and-intellectual-property',
      'quality-assurance-and-release-governance',
      'platform-cost-and-capacity-analytics',
      'multilingual-and-localization',
      'accessibility-and-usability',
    ],
  },
  {
    id: 'aspf',
    label: 'group.aspf',
    icon: 'network',
    tone: 'berry',
    domains: [
      'future-agriql-integration-contract',
      'future-kutajkutambi-ai-integration-contract',
      'future-agricuda-integration-contract',
      'future-apsnetwork-integration-contract',
      'future-nityaga-rtos-integration-contract',
      'hardware-and-fpga-research-interfaces',
    ],
  },
];

/** Domains whose own icon says more than their group's icon. */
const DOMAIN_ICONS: Record<string, IconName> = {
  'field-and-farm-model': 'field',
  'crop-and-cultivar-catalogue': 'seed',
  'crop-calendars-and-phenology': 'sprout',
  'pest-and-disease-intelligence': 'bug',
  'plant-phenotyping': 'leaf',
  'soil-chemistry-and-fertility': 'flask',
  'water-resources-and-hydrology': 'drop',
  'irrigation-engineering': 'sprinkler',
  'weather-observations': 'thermometer',
  'weather-forecasts-and-climate': 'cloudRain',
  'environmental-monitoring': 'wind',
  'sustainability-and-carbon': 'tree',
  'risk-and-resilience-analytics': 'alert',
  'geospatial-foundations': 'map',
  'gis-map-workspace': 'pin',
  'remote-sensing-indices': 'leaf',
  'agricultural-equipment-data': 'tractor',
  'sensor-registry-and-telemetry': 'thermometer',
  'iot-event-processing': 'network',
  'laboratory-and-sample-management': 'flask',
  'experimental-design': 'field',
  'machine-learning-lifecycle': 'sparkle',
  'natural-language-assistance': 'sparkle',
  'computer-vision': 'search',
  'forecasting-and-prediction': 'cloudRain',
  'decision-support-and-recommendations': 'check',
  'research-notebooks-and-projects': 'book',
  'reporting-and-publishing': 'scroll',
  'public-agricultural-datasets': 'globe',
  'time-series-data': 'clock',
  'supply-chain-and-traceability': 'truck',
  'agricultural-economics-and-markets': 'chart',
  'audit-and-compliance': 'scroll',
  'identity-and-access-management': 'users',
  'authentication': 'lock',
  'organization-and-tenancy': 'building',
  'notifications-and-subscriptions': 'bell',
  'external-data-connectors': 'plug',
  'documentation-and-knowledge': 'book',
  'multilingual-and-localization': 'globe',
  'future-agriql-integration-contract': 'database',
  'future-kutajkutambi-ai-integration-contract': 'sparkle',
  'future-agricuda-integration-contract': 'chip',
  'future-nityaga-rtos-integration-contract': 'chip',
  'hardware-and-fpga-research-interfaces': 'chip',
};

const BY_DOMAIN = new Map(GROUPS.flatMap((g) => g.domains.map((d) => [d, g] as const)));

const FALLBACK: DomainGroup = { id: 'other', label: 'group.other', icon: 'leaf', tone: 'leaf', domains: [] };

export function groupOf(domain: string): DomainGroup {
  return BY_DOMAIN.get(domain) ?? FALLBACK;
}

export function iconOf(domain: string): IconName {
  return DOMAIN_ICONS[domain] ?? groupOf(domain).icon;
}

export function groupById(id: string): DomainGroup | undefined {
  return GROUPS.find((g) => g.id === id);
}
