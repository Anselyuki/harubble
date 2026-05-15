import js from '@eslint/js';
import globals from 'globals';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import sveltePlugin from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import prettierConfig from 'eslint-config-prettier';

const browserGlobals = { ...globals.browser, ...globals.es2025 };
const nodeGlobals = { ...globals.node, ...globals.es2025 };

const typeAwareParserOptions = {
  projectService: {
    allowDefaultProject: [
      'eslint.config.js',
      'vite.config.ts',
      'svelte.config.js',
      'prettier.config.js',
    ],
  },
  tsconfigRootDir: import.meta.dirname,
};

const coreRules = {
  eqeqeq: ['error', 'always', { null: 'ignore' }],
  'no-console': 'error',
  'no-var': 'error',
  'prefer-const': 'error',
  'prefer-template': 'error',
  'object-shorthand': 'error',
  'no-useless-rename': 'error',
  'no-useless-computed-key': 'error',
  'no-useless-concat': 'error',
  'no-lonely-if': 'error',
  'prefer-arrow-callback': 'error',
};

const tsRules = {
  ...js.configs.recommended.rules,
  ...tseslint.configs.recommended.rules,
  ...coreRules,
  'no-throw-literal': 'off',
  'no-undef': 'off',
  '@typescript-eslint/no-unused-vars': [
    'error',
    { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
  ],
  '@typescript-eslint/no-non-null-asserted-nullish-coalescing': 'error',
  '@typescript-eslint/no-useless-constructor': 'error',
  '@typescript-eslint/consistent-type-assertions': 'error',
  '@typescript-eslint/consistent-type-imports': [
    'error',
    { prefer: 'type-imports', fixStyle: 'separate-type-imports' },
  ],
  '@typescript-eslint/consistent-generic-constructors': 'error',
  '@typescript-eslint/no-inferrable-types': 'error',
  '@typescript-eslint/prefer-for-of': 'error',
  '@typescript-eslint/array-type': ['error', { default: 'array' }],
  '@typescript-eslint/no-floating-promises': 'error',
  '@typescript-eslint/no-misused-promises': [
    'error',
    { checksVoidReturn: { arguments: false } },
  ],
  '@typescript-eslint/await-thenable': 'error',
  '@typescript-eslint/no-unnecessary-condition': 'error',
  '@typescript-eslint/only-throw-error': 'error',
  '@typescript-eslint/use-unknown-in-catch-callback-variable': 'error',
};

export default [
  {
    ignores: [
      'node_modules/**',
      'dist/**',
      'target/**',
      'src-tauri/target/**',
      '.svelte-kit/**',
      'build/**',
      '.claude/**',
      'src/lib/paraglide/**',
      'src/paraglide/**',
    ],
  },
  {
    files: ['**/*.{js,mjs,cjs}'],
    ...js.configs.recommended,
    languageOptions: {
      ...js.configs.recommended.languageOptions,
      globals: nodeGlobals,
    },
    rules: {
      ...js.configs.recommended.rules,
      ...coreRules,
      'no-array-constructor': 'error',
      'no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
  {
    files: ['scripts/**/*.{js,mjs,cjs}'],
    rules: {
      'no-console': 'off',
      'prefer-template': 'off',
      'no-useless-escape': 'warn',
    },
  },
  {
    files: ['**/*.ts'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ...typeAwareParserOptions,
      },
      globals: browserGlobals,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: tsRules,
  },
  {
    files: ['vite.config.ts'],
    languageOptions: {
      globals: nodeGlobals,
    },
  },
  ...sveltePlugin.configs['flat/recommended'],
  ...sveltePlugin.configs['flat/prettier'],
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tsparser,
        extraFileExtensions: ['.svelte'],
        ...typeAwareParserOptions,
      },
      globals: browserGlobals,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tsRules,
      'prefer-const': 'off',
      'svelte/prefer-const': 'error',
      'no-useless-assignment': 'error',
      'svelte/no-at-debug-tags': 'error',
      'svelte/no-inspect': 'error',
      'svelte/button-has-type': 'error',
      'svelte/no-target-blank': 'error',
      'svelte/spaced-html-comment': 'error',
      'svelte/block-lang': ['error', { script: 'ts' }],
    },
  },
  {
    files: ['src/lib/components/ui/**/*.svelte'],
    rules: {
      '@typescript-eslint/no-unnecessary-condition': 'off',
    },
  },
  {
    files: ['**/*.svelte.ts'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
        ...typeAwareParserOptions,
      },
      globals: browserGlobals,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tsRules,
      'prefer-const': 'off',
      'svelte/prefer-const': [
        'error',
        { excludedRunes: ['$state', '$derived'] },
      ],
      'no-useless-assignment': 'error',
    },
  },
  prettierConfig,
];
