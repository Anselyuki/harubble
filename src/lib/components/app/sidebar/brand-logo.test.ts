import { beforeAll, describe, expect, it } from 'vitest';

let source = '';
let sidebarSource = '';
let animatorSource = '';
let expandAnimatorSource = '';
let coreAnimatorSource = '';
let collapseAnimatorSource = '';

beforeAll(async () => {
  // @ts-expect-error Vitest runs in Node and reads local source files.
  const { readFileSync } = await import('node:fs');
  source = readFileSync(
    'src/lib/components/app/sidebar/BrandLogo.svelte',
    'utf8'
  );
  sidebarSource = readFileSync(
    'src/lib/components/app/sidebar/AppSidebar.svelte',
    'utf8'
  );
  animatorSource = [
    'src/lib/design/sidebar-animator.ts',
    'src/lib/design/sidebar-animator-collapse.ts',
    'src/lib/design/sidebar-animator-expand.ts',
  ]
    .map((path) => readFileSync(path, 'utf8'))
    .join('\n');
  expandAnimatorSource = readFileSync(
    'src/lib/design/sidebar-animator-expand.ts',
    'utf8'
  );
  collapseAnimatorSource = readFileSync(
    'src/lib/design/sidebar-animator-collapse.ts',
    'utf8'
  );
  coreAnimatorSource = readFileSync(
    'src/lib/design/sidebar-animator.ts',
    'utf8'
  );
});

describe('BrandLogo styling', () => {
  it('renders the accent slab as grid-overlapping sibling via BrandSlab component', () => {
    expect(source).not.toContain('class="brand-slab"');
    expect(source).not.toContain('brand-logo-slab');
    expect(sidebarSource).toContain('BrandSlab');
    expect(sidebarSource).toContain('bind:slabEl={logoSlabEl}');
    expect(sidebarSource).toContain('class="brand-region"');
    expect(sidebarSource).toContain('display: grid;');
    expect(sidebarSource).toContain('grid-template: 1fr / 1fr;');
    expect(source).toContain('grid-area: 1 / 1;');
    expect(source).toContain('z-index: 1;');
    expect(source).toContain('padding: 20px 8px 12px 10px;');
  });

  it('renders filled and outline logo glyphs with font text', () => {
    expect(source).toContain('data-logo-glyph');
    expect(source).toContain('{letter.char}');
    expect(source).toContain('class:outline={letter.outline}');
    expect(source).toContain('.brand-char.outline');
    expect(source).toContain('font-family: var(--font-wide);');
    expect(source).toContain('font-weight: 700;');
    expect(source).toContain('color: var(--theme-text-primary);');
    expect(source).toContain('color: transparent;');
    expect(source).toContain('-webkit-text-stroke: 1.5px var(--theme-tint);');
    expect(source).not.toContain('<svg');
    expect(source).not.toContain('const GLYPH_PATHS');
    expect(source).not.toContain('const GLYPH_NORMALIZE_TRANSFORMS');
    expect(source).not.toContain('<mask');
    expect(source).not.toContain('<filter');
    expect(source).not.toContain('feMorphology');
    expect(source).not.toContain('brand-glyph');
  });

  it('keeps font logo glyphs compact with a little more breathing room', () => {
    expect(source).toContain('--brand-logo-glyph-size: 22px;');
    expect(source).toContain('--brand-logo-char-gap: 0px;');
    expect(source).toContain('font-size: var(--brand-logo-glyph-size);');
    expect(source).toContain('gap: var(--brand-logo-char-gap);');
    expect(source).toContain('row-gap: var(--brand-logo-char-gap);');
    expect(source).not.toContain('--brand-logo-glyph-size: 18px;');
    expect(source).not.toContain('--brand-logo-glyph-size: 19px;');
    expect(source).not.toContain('--brand-logo-glyph-size: 20px;');
    expect(source).not.toContain('--brand-logo-char-gap: 2px;');
  });

  it('keeps BrandLogo animation under the GSAP sidebar animator', () => {
    expect(source).not.toMatch(/\btransition\s*:/);
    expect(source).not.toMatch(/\banimation\s*:/);
    expect(animatorSource).toContain('animateLogoSlabInsets');
    expect(animatorSource).toContain("marginLeft: '0px'");
    expect(animatorSource).toContain('logoSlabEl');
  });

  it('centers the vertical logo letters within the collapsed slab width', () => {
    expect(source).toContain('--brand-logo-mark-offset-x: 0px;');
    expect(source).toContain('--brand-logo-mark-offset-x: calc(');
    expect(source).toContain('var(--brand-logo-collapsed-slab-right) * -0.5');
    expect(source).toContain(
      'transform: translateX(var(--brand-logo-mark-offset-x));'
    );
  });

  it('tightens the vertical logo character spacing by 2px', () => {
    expect(source).toContain(
      '--brand-logo-collapsed-char-line-height: calc(0.88em - 2px);'
    );
    expect(source).toContain(
      'line-height: var(--brand-logo-collapsed-char-line-height);'
    );
  });

  it('does not fade logo characters during orientation animation', () => {
    expect(animatorSource).not.toContain('opacity: 0.35');
    expect(animatorSource).not.toContain('opacity: 0.6');
    expect(animatorSource).not.toContain('config.logoCharEls,\n    {');
    expect(animatorSource).not.toContain("clearProps: 'x,y,opacity,transform'");
  });

  it('expands the slab right after rotation and before logo movement on expand', () => {
    const rotationCompleteIndex = expandAnimatorSource.indexOf(
      'await chainTimelineComplete(phase1)'
    );
    const slabExpansionIndex = expandAnimatorSource.indexOf(
      'ctx.expandLogoSlabRight'
    );
    const flipIndex = expandAnimatorSource.indexOf('ctx.flipPhase');

    expect(rotationCompleteIndex).toBeGreaterThan(-1);
    expect(slabExpansionIndex).toBeGreaterThan(-1);
    expect(flipIndex).toBeGreaterThan(-1);
    expect(rotationCompleteIndex).toBeLessThan(slabExpansionIndex);
    expect(slabExpansionIndex).toBeLessThan(flipIndex);
    expect(coreAnimatorSource).not.toContain('expandLogoSlabLeft');
    expect(coreAnimatorSource).not.toContain(
      "toCollapsed ? 'collapsed' : 'expanded'"
    );
  });

  it('measures collapsed logo targets in the final collapsed sidebar frame', () => {
    const frameIndex = collapseAnimatorSource.indexOf(
      "document.createElement('aside')"
    );
    const measureIndex = collapseAnimatorSource.indexOf(
      'measureCollapsedTargets('
    );
    const sidebarClassIndex = collapseAnimatorSource.indexOf(
      'measurementSidebar.className = sidebarEl.className;'
    );
    const collapsedClassIndex = collapseAnimatorSource.indexOf(
      "measurementSidebar.classList.add('collapsed');"
    );
    const sidebarWidthIndex = collapseAnimatorSource.indexOf(
      'measurementSidebar.style.width = collapsedSidebarWidth;'
    );
    const frameAppendIndex = collapseAnimatorSource.indexOf(
      'measurementSidebar.appendChild(clone);'
    );
    const shellAppendIndex = collapseAnimatorSource.indexOf(
      'sidebarEl.parentElement!.appendChild(measurementSidebar);'
    );

    expect(frameIndex).toBeGreaterThan(-1);
    expect(measureIndex).toBeGreaterThan(-1);
    expect(sidebarClassIndex).toBeGreaterThan(frameIndex);
    expect(collapsedClassIndex).toBeGreaterThan(sidebarClassIndex);
    expect(sidebarWidthIndex).toBeGreaterThan(collapsedClassIndex);
    expect(frameAppendIndex).toBeGreaterThan(sidebarWidthIndex);
    expect(shellAppendIndex).toBeGreaterThan(frameAppendIndex);
    expect(collapseAnimatorSource).toContain(
      [
        'const measured = measureCollapsedTargets(',
        '    config.sidebarEl,',
        '    config.logoContainerEl,',
        '    config.logoCharEls,',
        '    ctx.constants.COLLAPSED_WIDTH',
        '  );',
      ].join('\n')
    );
  });

  it('keeps collapsed logo letters locked through final width cleanup', () => {
    const lockRectsIndex = collapseAnimatorSource.indexOf(
      'const lockedCharRects = config.logoCharEls.map((el) =>'
    );
    const lockFunctionIndex = collapseAnimatorSource.indexOf(
      'const lockLogoCharTopLeft = () =>'
    );
    const widthTweenIndex = collapseAnimatorSource.indexOf(
      "config.shellEl,\n    {\n      '--sidebar-width': ctx.constants.COLLAPSED_WIDTH,"
    );
    const updateIndex = collapseAnimatorSource.indexOf(
      'onUpdate: lockLogoCharTopLeft'
    );
    const completeIndex = collapseAnimatorSource.indexOf(
      'onComplete: lockLogoCharTopLeft'
    );
    const clearIndex = collapseAnimatorSource.lastIndexOf(
      "clearProps: 'x,y,transform'"
    );

    expect(collapseAnimatorSource).toContain('getTopLeftLockTransform');
    expect(collapseAnimatorSource).toContain('flushSync');
    expect(lockRectsIndex).toBeGreaterThan(-1);
    expect(lockFunctionIndex).toBeGreaterThan(lockRectsIndex);
    expect(widthTweenIndex).toBeGreaterThan(lockFunctionIndex);
    expect(updateIndex).toBeGreaterThan(widthTweenIndex);
    expect(completeIndex).toBeGreaterThan(widthTweenIndex);
    expect(clearIndex).toBeGreaterThan(completeIndex);
  });

  it('completes collapse with direct layout switch (no intermediate measurement)', () => {
    const layoutSwitchIndex = collapseAnimatorSource.indexOf(
      'config.onLayoutSwitch(true);'
    );
    const flushIndex = collapseAnimatorSource.indexOf('flushSync();');
    const clearPropsIndex = collapseAnimatorSource.lastIndexOf(
      "clearProps: 'x,y,transform'"
    );

    expect(layoutSwitchIndex).toBeGreaterThan(-1);
    expect(flushIndex).toBeGreaterThan(layoutSwitchIndex);
    expect(clearPropsIndex).toBeGreaterThan(flushIndex);
    expect(collapseAnimatorSource).not.toContain(
      'config.onLayoutSwitch(false);'
    );
  });
});
