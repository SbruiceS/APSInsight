// APSInsight agricultural icon set. Every icon is drawn on a 24×24 grid with
// round strokes in `currentColor`, so it inherits the surrounding text colour
// and theme. Each icon is a list of parts; a part's `motion` names the CSS
// animation applied to it (see src/styles/_icons.scss), e.g. leaves sway,
// sun rays turn, drops fall, wheels roll. Motion runs on hover/focus of the
// enclosing control, or always when the icon is `live`, and is disabled
// entirely under `prefers-reduced-motion`.

export type Motion =
  | 'sway'
  | 'grow'
  | 'spin'
  | 'drip'
  | 'rain'
  | 'roll'
  | 'puff'
  | 'pulse'
  | 'bob'
  | 'orbit'
  | 'wave'
  | 'flow'
  | 'wiggle'
  | 'twinkle'
  | 'draw'
  | 'ring'
  | 'bubble'
  | 'twist'
  | 'rise'
  | 'needle'
  | 'spray'
  | 'flip';

export interface IconPart {
  d: string[];
  motion?: Motion;
  fill?: boolean;
}

/** Path for a circle, so every shape stays a `<path>`. */
const c = (cx: number, cy: number, r: number): string => `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;

export const ICONS = {
  sprout: [
    { d: ['M5 21h14'] },
    { d: ['M12 21v-9'], motion: 'grow' },
    { d: ['M12 13c-4 0-6.5-2.5-6.5-6.5 4 0 6.5 2.5 6.5 6.5z', 'M12 11c0-4 2.5-6.5 6.5-6.5 0 4-2.5 6.5-6.5 6.5z'], motion: 'sway' },
  ],
  leaf: [{ d: ['M5 19c0-8 6-14 14-14 0 8-6 14-14 14z', 'M5 19l9-9'], motion: 'sway' }],
  wheat: [
    { d: ['M12 22V7'] },
    {
      d: [
        'M12 8c-2.2-.8-3.4-2.6-3.4-5 2.2.8 3.4 2.6 3.4 5z',
        'M12 8c2.2-.8 3.4-2.6 3.4-5-2.2.8-3.4 2.6-3.4 5z',
        'M12 13c-2.2-.8-3.4-2.6-3.4-5 2.2.8 3.4 2.6 3.4 5z',
        'M12 13c2.2-.8 3.4-2.6 3.4-5-2.2.8-3.4 2.6-3.4 5z',
        'M12 18c-2.2-.8-3.4-2.6-3.4-5 2.2.8 3.4 2.6 3.4 5z',
        'M12 18c2.2-.8 3.4-2.6 3.4-5-2.2.8-3.4 2.6-3.4 5z',
      ],
      motion: 'sway',
    },
  ],
  tractor: [
    { d: ['M3 15V9h7l2 4h7v3', 'M6 9V5h4v4', 'M10 15h4'] },
    { d: [c(7, 17, 3), 'M7 15.5v3'], motion: 'roll' },
    { d: [c(18, 18, 2)], motion: 'roll' },
    { d: [c(15.5, 4.5, 1)], motion: 'puff' },
  ],
  drop: [{ d: ['M12 3c3.5 4.5 6 7.5 6 11a6 6 0 0 1-12 0c0-3.5 2.5-6.5 6-11z', 'M9.5 14.5a3 3 0 0 0 2 2.5'], motion: 'drip' }],
  sun: [
    { d: [c(12, 12, 4)], motion: 'pulse' },
    {
      d: ['M12 2v2', 'M12 20v2', 'M2 12h2', 'M20 12h2', 'M4.9 4.9l1.4 1.4', 'M17.7 17.7l1.4 1.4', 'M4.9 19.1l1.4-1.4', 'M17.7 6.3l1.4-1.4'],
      motion: 'spin',
    },
  ],
  cloudRain: [
    { d: ['M7 15h10a4 4 0 0 0 0-8 5.5 5.5 0 0 0-10.5 1.5A3.3 3.3 0 0 0 7 15z'] },
    { d: ['M8 18l-1 2.5', 'M12 18l-1 2.5', 'M16 18l-1 2.5'], motion: 'rain' },
  ],
  soil: [
    { d: ['M3 8c3-2 6 2 9 0s6 2 9 0', 'M3 13h18', 'M3 18h18'] },
    { d: [c(7, 15.5, 0.8), c(14, 15.5, 0.8), c(10, 20.5, 0.8), c(18, 20.5, 0.8)], fill: true, motion: 'pulse' },
    { d: ['M12 7V3.5', 'M12 5c-1.5 0-2.5-1-2.5-2.5 1.5 0 2.5 1 2.5 2.5z'], motion: 'sway' },
  ],
  field: [
    { d: ['M3 7h18', 'M2 21L9 7', 'M8 21l2.5-14', 'M16 21l-2.5-14', 'M22 21L15 7'] },
    { d: [c(18, 3.5, 1.5)], motion: 'pulse' },
  ],
  barn: [
    { d: ['M3 21V10l9-6 9 6v11z', 'M9 21v-6h6v6', 'M9 15l6 6', 'M15 15l-6 6', 'M10.5 10.5h3'], motion: 'bob' },
  ],
  seed: [{ d: ['M12 4c4 3 6 7 6 10a6 6 0 0 1-12 0c0-3 2-7 6-10z', 'M12 10v8'], motion: 'bob' }],
  sprinkler: [
    { d: ['M12 21v-8', 'M9 21h6', 'M10 13h4'] },
    { d: ['M12 11c-3-1-6-3-8-6', 'M12 11c3-1 6-3 8-6', 'M12 11V4'], motion: 'spray' },
  ],
  flask: [
    { d: ['M9 3h6', 'M10 3v6L5 19a2 2 0 0 0 1.7 3h10.6a2 2 0 0 0 1.7-3l-5-10V3', 'M7.5 15h9'] },
    { d: [c(11, 18, 0.8), c(13.6, 17, 0.6)], fill: true, motion: 'bubble' },
  ],
  microscope: [
    { d: ['M6 21h12', 'M9 21v-3', 'M8 18h8', 'M14 18a5 5 0 0 0-2-9', 'M7 14h6'] },
    { d: ['M9 3l4 2-3 6-4-2z'], motion: 'bob' },
  ],
  dna: [
    {
      d: ['M7 3c0 5 10 7 10 12 0 3-2 5-2 6', 'M17 3c0 5-10 7-10 12 0 3 2 5 2 6', 'M8.5 6h7', 'M9.5 12h5', 'M8 18h8'],
      motion: 'twist',
    },
  ],
  satellite: [
    { d: ['M2 9h5v4H2z', 'M17 9h5v4h-5z', 'M9 8h6v6H9z', 'M7 11h2', 'M15 11h2', 'M12 14v3', 'M9.5 19.5a2.5 2.5 0 0 1 5 0'], motion: 'orbit' },
    { d: ['M17 17a4 4 0 0 1 2 3', 'M19 14.5a7 7 0 0 1 3 5.5'], motion: 'wave' },
  ],
  chart: [
    { d: ['M4 20V4', 'M4 20h16'] },
    { d: ['M8 20v-5', 'M12 20v-8', 'M16 20v-3'], motion: 'grow' },
    { d: ['M7 10l4-4 3 3 5-5'], motion: 'draw' },
  ],
  map: [{ d: ['M9 4L3 6v14l6-2 6 2 6-2V4l-6 2-6-2z', 'M9 4v14', 'M15 6v14'] }, { d: [c(18, 9, 1)], fill: true, motion: 'bob' }],
  pin: [{ d: ['M12 21s-6-6.5-6-11a6 6 0 0 1 12 0c0 4.5-6 11-6 11z', c(12, 10, 2)], motion: 'bob' }],
  thermometer: [{ d: ['M10 14V5a2 2 0 0 1 4 0v9a4 4 0 1 1-4 0z'] }, { d: ['M12 17v-7'], motion: 'rise' }],
  wind: [{ d: ['M3 9h11a3 3 0 1 0-3-3', 'M3 15h15a3 3 0 1 1-3 3', 'M3 12h8'], motion: 'flow' }],
  bug: [
    { d: ['M12 7a4 4 0 0 1 4 4v4a4 4 0 0 1-8 0v-4a4 4 0 0 1 4-4z', 'M10 7a2 2 0 0 1 4 0', 'M12 9v10'] },
    { d: ['M8 11H5', 'M8 15H5', 'M16 11h3', 'M16 15h3', 'M9 19l-2 2', 'M15 19l2 2'], motion: 'wiggle' },
    { d: ['M10.5 5L9 3', 'M13.5 5L15 3'], motion: 'wiggle' },
  ],
  tree: [
    { d: ['M12 22v-6', 'M9 22h6'] },
    { d: ['M12 16c-4 0-7-2.5-7-6a5 5 0 0 1 3-4.5A4 4 0 0 1 16 5.5 5 5 0 0 1 19 10c0 3.5-3 6-7 6z'], motion: 'sway' },
  ],
  coins: [{ d: [c(9, 9, 5), 'M9 6.5v5', 'M7.5 8h3'], motion: 'flip' }, { d: ['M15.5 7.5a5 5 0 1 1-6 7.6'] }],
  truck: [{ d: ['M2 7h11v9H2z', 'M13 10h4l3 3v3h-7'] }, { d: [c(6, 18, 2)], motion: 'roll' }, { d: [c(17, 18, 2)], motion: 'roll' }],
  network: [
    { d: ['M12 7v5', 'M12 12l-5.5 4.5', 'M12 12l5.5 4.5'] },
    { d: [c(12, 5, 2), c(5, 18, 2), c(19, 18, 2)], motion: 'pulse' },
  ],
  chip: [
    { d: ['M7 7h10v10H7z', 'M9 3v4', 'M15 3v4', 'M9 17v4', 'M15 17v4', 'M3 9h4', 'M3 15h4', 'M17 9h4', 'M17 15h4'] },
    { d: ['M10 10h4v4h-4z'], motion: 'pulse' },
  ],
  sparkle: [
    { d: ['M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z'], motion: 'twinkle' },
    { d: ['M19 15l.7 2 2 .7-2 .7-.7 2-.7-2-2-.7 2-.7z'], motion: 'twinkle' },
  ],
  shield: [{ d: ['M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z'] }, { d: ['M8.5 12l2.5 2.5 4.5-5'], motion: 'draw' }],
  lock: [{ d: ['M6 11h12v10H6z', 'M8 11V7a4 4 0 0 1 8 0v4'] }, { d: [c(12, 16, 1)], fill: true, motion: 'pulse' }],
  users: [{ d: [c(9, 8, 3), 'M3 20a6 6 0 0 1 12 0', c(17, 9, 2.5), 'M15.5 15.2A5 5 0 0 1 22 20'], motion: 'bob' }],
  building: [{ d: ['M4 21V5l8-3v19', 'M12 9h8v12', 'M3 21h18', 'M7 8h2', 'M7 12h2', 'M7 16h2', 'M15 13h2', 'M15 17h2'] }],
  bell: [{ d: ['M6 16v-5a6 6 0 0 1 12 0v5l2 2H4z', 'M10 20a2 2 0 0 0 4 0'], motion: 'ring' }],
  scroll: [{ d: ['M7 3h10a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z'] }, { d: ['M9 8h6', 'M9 12h6', 'M9 16h4'], motion: 'draw' }],
  book: [{ d: ['M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z', 'M4 19a2 2 0 0 1 2-2h13', 'M9 7h6', 'M9 10h6'], motion: 'bob' }],
  plug: [{ d: ['M9 2v5', 'M15 2v5', 'M7 7h10v4a5 5 0 0 1-10 0z', 'M12 16v6'] }, { d: ['M19 14l2-1', 'M19 18l2 1'], motion: 'twinkle' }],
  gauge: [{ d: ['M4 18a8 8 0 1 1 16 0', 'M12 6v1.5', 'M6.3 8.3l1 1', 'M17.7 8.3l-1 1'] }, { d: ['M12 18l4-6'], motion: 'needle' }],
  database: [
    { d: ['M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z', 'M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6', 'M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3'] },
    { d: [c(17, 12.8, 0.6), c(17, 18.6, 0.6)], fill: true, motion: 'pulse' },
  ],
  layers: [{ d: ['M12 3l9 5-9 5-9-5z'], motion: 'bob' }, { d: ['M3 13l9 5 9-5', 'M3 17.5l9 5 9-5'] }],
  ruler: [{ d: ['M3 17L17 3l4 4L7 21z', 'M7 13l2 2', 'M10 10l2 2', 'M13 7l2 2'], motion: 'wiggle' }],
  clock: [{ d: [c(12, 12, 9)] }, { d: ['M12 7v5l3 2'], motion: 'needle' }],
  search: [{ d: [c(11, 11, 6), 'M20 20l-4.5-4.5'], motion: 'bob' }],
  home: [{ d: ['M3 11l9-7 9 7', 'M5 10v10h14V10', 'M10 20v-5h4v5'], motion: 'bob' }],
  logout: [{ d: ['M15 4h4v16h-4'] }, { d: ['M10 8l-4 4 4 4', 'M6 12h10'], motion: 'flow' }],
  globe: [{ d: [c(12, 12, 9), 'M3 12h18', 'M12 3c3 3 3 15 0 18', 'M12 3c-3 3-3 15 0 18'], motion: 'spin' }],
  moon: [{ d: ['M20 14A8 8 0 1 1 10 4a6.5 6.5 0 0 0 10 10z'], motion: 'bob' }],
  menu: [{ d: ['M4 6h16', 'M4 12h16', 'M4 18h16'] }],
  close: [{ d: ['M6 6l12 12', 'M18 6L6 18'] }],
  plus: [{ d: ['M12 5v14', 'M5 12h14'], motion: 'spin' }],
  download: [{ d: ['M5 20h14'] }, { d: ['M12 4v11', 'M7 10l5 5 5-5'], motion: 'drip' }],
  upload: [{ d: ['M5 20h14'] }, { d: ['M12 15V4', 'M7 9l5-5 5 5'], motion: 'rise' }],
  history: [{ d: ['M3 12a9 9 0 1 0 3-6.7', 'M3 4v5h5'] }, { d: ['M12 7v5l3 2'], motion: 'needle' }],
  check: [{ d: ['M5 12.5l4.5 4.5L19 7.5'], motion: 'draw' }],
  alert: [{ d: ['M12 3l10 18H2z'] }, { d: ['M12 9v5', 'M12 17v.5'], motion: 'pulse' }],
  arrowRight: [{ d: ['M5 12h14', 'M13 6l6 6-6 6'], motion: 'flow' }],
  filter: [{ d: ['M4 5h16l-6 7v6l-4 2v-8z'], motion: 'drip' }],
  sliders: [
    { d: ['M4 6h10', 'M18 6h2', 'M4 12h4', 'M12 12h8', 'M4 18h12', 'M20 18h0'] },
    { d: [c(16, 6, 2), c(10, 12, 2), c(18, 18, 2)], motion: 'flow' },
  ],
} satisfies Record<string, IconPart[]>;

export type IconName = keyof typeof ICONS;

export const ICON_NAMES = Object.keys(ICONS) as IconName[];
