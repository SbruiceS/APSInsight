import { Component, input } from '@angular/core';

/**
 * The APSInsight logo as a resolution-independent SVG: a leaf-cut "A", the
 * "PS" letterforms and "Insight", ringed by a growth chart, an orbit arc and
 * a field-wave swoosh.
 *
 * With `animated`, the parts assemble once: letters rise in, the leaf
 * unfurls, bars grow, the trend line draws and the arcs sweep round. Then a
 * gentle loop runs (the leaf sways, the newest data point pulses, a sheen
 * crosses the wordmark). Default styles are the finished logo, so
 * reduced-motion users and static renders see the complete mark. Styles live
 * in styles/_logo.scss.
 */
@Component({
  selector: 'apsi-logo',
  host: { class: 'apsi-logo', '[class.animated]': 'animated()' },
  template: `
    <svg viewBox="40 100 1520 580" role="img" [attr.aria-label]="label()" focusable="false">
      <defs>
        <linearGradient id="lg-word" gradientUnits="userSpaceOnUse" x1="60" y1="0" x2="905" y2="0">
          <stop offset="0" class="lg-stop-leaf" />
          <stop offset="0.38" class="lg-stop-green" />
          <stop offset="0.68" class="lg-stop-teal" />
          <stop offset="1" class="lg-stop-navy" />
        </linearGradient>
        <linearGradient id="lg-a-shade" gradientUnits="userSpaceOnUse" x1="0" y1="275" x2="0" y2="530">
          <stop offset="0" class="lg-stop-leaf" />
          <stop offset="1" class="lg-stop-forest" />
        </linearGradient>
        <linearGradient id="lg-leaf" gradientUnits="userSpaceOnUse" x1="120" y1="0" x2="390" y2="0">
          <stop offset="0" class="lg-stop-forest" />
          <stop offset="1" class="lg-stop-leaf" />
        </linearGradient>
        <linearGradient id="lg-i" gradientUnits="userSpaceOnUse" x1="0" y1="350" x2="0" y2="470">
          <stop offset="0" class="lg-stop-leaf" />
          <stop offset="0.22" class="lg-stop-leaf" />
          <stop offset="0.24" class="lg-stop-ink" />
          <stop offset="1" class="lg-stop-ink" />
        </linearGradient>
        <linearGradient id="lg-chart" gradientUnits="userSpaceOnUse" x1="0" y1="330" x2="0" y2="200">
          <stop offset="0" class="lg-stop-navy" />
          <stop offset="1" class="lg-stop-teal" />
        </linearGradient>
        <linearGradient id="lg-orbit" gradientUnits="userSpaceOnUse" x1="1300" y1="180" x2="1300" y2="650">
          <stop offset="0" class="lg-stop-teal" />
          <stop offset="1" class="lg-stop-navy" />
        </linearGradient>
        <linearGradient id="lg-wave" gradientUnits="userSpaceOnUse" x1="905" y1="0" x2="1390" y2="0">
          <stop offset="0" class="lg-stop-green" />
          <stop offset="0.55" class="lg-stop-teal" />
          <stop offset="1" class="lg-stop-navy" />
        </linearGradient>
        <linearGradient id="lg-sheen" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stop-color="#fff" stop-opacity="0" />
          <stop offset="0.5" stop-color="#fff" stop-opacity="0.55" />
          <stop offset="1" stop-color="#fff" stop-opacity="0" />
        </linearGradient>
        <clipPath id="lg-letters">
          <path [attr.d]="A" />
          <path [attr.d]="P" />
          <path [attr.d]="S" />
        </clipPath>
      </defs>

      <!-- orbit: thin green arc over the top, broad blue crescent round the right -->
      <path class="lg-arc-green" pathLength="1" d="M1020 346C1036 236 1150 124 1326 127" />
      <path
        class="lg-arc-blue"
        fill="url(#lg-orbit)"
        d="M1410 183C1532 262 1562 436 1472 548C1412 622 1322 652 1238 642C1322 628 1402 590 1450 528C1522 432 1500 272 1410 183Z"
      />

      <!-- field waves -->
      <g class="lg-waves">
        <path
          class="lg-wave lg-wave-1"
          fill="url(#lg-wave)"
          d="M905 545C1000 506 1120 514 1200 544C1272 572 1332 576 1386 562C1322 592 1252 592 1186 566C1102 533 1002 526 905 545Z"
        />
        <path
          class="lg-wave lg-wave-2"
          fill="url(#lg-wave)"
          d="M960 556C1050 550 1130 574 1200 600C1272 626 1332 626 1382 610C1322 642 1242 642 1180 616C1102 586 1032 566 960 556Z"
        />
      </g>

      <!-- growth chart -->
      <g class="lg-bars" fill="url(#lg-chart)">
        <rect class="lg-bar" style="--i: 0" x="1160" y="314" width="20" height="16" rx="3" />
        <rect class="lg-bar" style="--i: 1" x="1210" y="298" width="20" height="32" rx="3" />
        <rect class="lg-bar" style="--i: 2" x="1258" y="266" width="21" height="64" rx="3" />
        <rect class="lg-bar" style="--i: 3" x="1308" y="254" width="21" height="76" rx="3" />
        <rect class="lg-bar" style="--i: 4" x="1360" y="216" width="22" height="114" rx="3" />
      </g>
      <polyline class="lg-trend" pathLength="1" points="1125,293 1170,258 1220,258 1268,213 1318,213 1373,160" />
      <g class="lg-points">
        <circle class="lg-point lg-point-green" style="--i: 0" cx="1125" cy="293" r="11" />
        <circle class="lg-point lg-point-green" style="--i: 1" cx="1170" cy="258" r="11" />
        <circle class="lg-point lg-point-green" style="--i: 2" cx="1220" cy="258" r="11" />
        <circle class="lg-point" style="--i: 3" cx="1268" cy="213" r="11" />
        <circle class="lg-point" style="--i: 4" cx="1318" cy="213" r="11" />
        <circle class="lg-point lg-point-last" style="--i: 5" cx="1373" cy="160" r="14" />
      </g>

      <!-- wordmark -->
      <g class="lg-word">
        <path class="lg-letter lg-a" style="--i: 0" fill="url(#lg-a-shade)" [attr.d]="A" />
        <g class="lg-leaf-group">
          <path class="lg-leaf" fill="url(#lg-leaf)" d="M118 514C186 440 300 420 390 458C302 470 204 482 118 514Z" />
          <path class="lg-leaf lg-leaf-under" fill="url(#lg-leaf)" d="M146 522C226 474 318 470 382 463C322 502 232 522 146 522Z" />
          <path class="lg-leaf-rib" d="M162 494C232 462 302 454 364 458" />
        </g>
        <path class="lg-letter lg-p" style="--i: 1" fill="url(#lg-word)" [attr.d]="P" />
        <path class="lg-letter lg-s" style="--i: 2" fill="url(#lg-word)" [attr.d]="S" />
        <g clip-path="url(#lg-letters)">
          <rect class="lg-sheen" fill="url(#lg-sheen)" x="-160" y="260" width="160" height="290" />
        </g>
      </g>
      <text class="lg-insight" x="944" y="470" textLength="512" lengthAdjust="spacingAndGlyphs">
        <tspan fill="url(#lg-i)">I</tspan><tspan class="lg-ink">nsight</tspan>
      </text>
    </svg>
  `,
})
export class Logo {
  readonly animated = input(false);
  readonly label = input('APSInsight');

  /** Letter outlines, shared by the visible letters and the sheen clip. */
  protected readonly A = 'M222 274Q240 262 258 274L410 500L340 500L240 337L122 530L58 530Z';
  protected readonly P =
    'M386 278L610 278C662 278 690 310 690 349C690 388 662 420 610 420L466 420L466 500L418 500L418 372L604 372C622 372 632 362 632 349C632 336 622 326 604 326L418 326Z';
  protected readonly S =
    'M770 278L905 278L862 326L772 326C754 326 744 336 744 348C744 360 754 370 772 370L820 370C872 370 900 398 900 435C900 474 872 500 820 500L640 500L682 452L820 452C836 452 844 445 844 435C844 425 836 418 820 418L772 418C726 418 698 390 698 348C698 306 726 278 770 278Z';
}
