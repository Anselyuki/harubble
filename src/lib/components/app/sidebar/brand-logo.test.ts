import { beforeAll, describe, expect, it } from 'vitest';

let source = '';
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
  it('renders the logo on an accent slab flush with the page edge', () => {
    expect(source).toContain('class="brand-logo-slab"');
    expect(source).toContain('padding: 20px 8px 12px 10px;');
    expect(source).toContain('--brand-logo-slab-left: 0px;');
    expect(source).toContain('--brand-logo-slab-right: 8px;');
    expect(source).toContain('position: absolute;');
    expect(source).toContain('left: var(--brand-logo-slab-left);');
    expect(source).toContain('right: var(--brand-logo-slab-right);');
    expect(source).toContain('bottom: 12px;');
    expect(source).toContain('background: var(--accent);');
    expect(source).toContain('border-radius: 0 8px 8px 0;');
    expect(source).not.toContain('padding: 20px 8px 12px 24px;');
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
    expect(animatorSource).toContain('animateLogoSlabInsetVars');
    expect(animatorSource).toContain("left: '0px'");
    expect(animatorSource).toContain('--brand-logo-slab-left');
    expect(animatorSource).toContain('--brand-logo-slab-right');
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

  it('measures collapsed logo targets in the sidebar content box', () => {
    const layoutWidthIndex = collapseAnimatorSource.indexOf(
      'getCollapsedLogoLayoutWidth'
    );
    const measureIndex = collapseAnimatorSource.indexOf(
      'measureCollapsedTargets('
    );
    const targetWidthIndex = collapseAnimatorSource.indexOf(
      'collapsedLogoLayoutWidth'
    );

    expect(layoutWidthIndex).toBeGreaterThan(-1);
    expect(measureIndex).toBeGreaterThan(-1);
    expect(targetWidthIndex).toBeGreaterThan(-1);
    expect(collapseAnimatorSource).toContain(
      'clone.style.left = `${logoContainerEl.offsetLeft}px`;'
    );
    expect(collapseAnimatorSource).toContain(
      'clone.style.top = `${logoContainerEl.offsetTop}px`;'
    );
    expect(collapseAnimatorSource).toContain(
      [
        'const measured = measureCollapsedTargets(',
        '    config.logoContainerEl,',
        '    config.logoCharEls,',
        '    collapsedLogoLayoutWidth',
        '  );',
      ].join('\n')
    );
  });

  it('keeps collapsed logo letters locked through final width cleanup', () => {
    const lockRectsIndex = collapseAnimatorSource.indexOf(
      'const lockedCharRects = config.logoCharEls.map((el) =>'
    );
    const lockFunctionIndex = collapseAnimatorSource.indexOf(
      'const lockLogoCharCenters = () =>'
    );
    const widthTweenIndex = collapseAnimatorSource.indexOf(
      "config.shellEl,\n    {\n      '--sidebar-width': ctx.constants.COLLAPSED_WIDTH,"
    );
    const updateIndex = collapseAnimatorSource.indexOf(
      'onUpdate: lockLogoCharCenters'
    );
    const completeIndex = collapseAnimatorSource.indexOf(
      'onComplete: lockLogoCharCenters'
    );
    const layoutSwitchIndex = collapseAnimatorSource.indexOf(
      'config.onLayoutSwitch(true);'
    );
    const flushIndex = collapseAnimatorSource.indexOf('flushSync();');
    const clearIndex = collapseAnimatorSource.indexOf(
      "clearProps: 'x,y,transform'"
    );

    expect(collapseAnimatorSource).toContain('getCenterLockTransform');
    expect(collapseAnimatorSource).toContain('flushSync');
    expect(lockRectsIndex).toBeGreaterThan(-1);
    expect(lockFunctionIndex).toBeGreaterThan(lockRectsIndex);
    expect(widthTweenIndex).toBeGreaterThan(lockFunctionIndex);
    expect(updateIndex).toBeGreaterThan(widthTweenIndex);
    expect(completeIndex).toBeGreaterThan(widthTweenIndex);
    expect(layoutSwitchIndex).toBeGreaterThan(completeIndex);
    expect(flushIndex).toBeGreaterThan(layoutSwitchIndex);
    expect(clearIndex).toBeGreaterThan(flushIndex);
  });
});
