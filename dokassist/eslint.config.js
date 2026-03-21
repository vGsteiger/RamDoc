import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  // Base JS recommended rules
  js.configs.recommended,

  // TypeScript recommended rules (non-type-aware — safe without project reference)
  ...tseslint.configs.recommended,

  // Svelte + TypeScript integration
  ...svelte.configs['flat/recommended'],

  // Global settings for browser + node environments
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },

  // Svelte files: tell the TS parser about the svelte parser
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },

  // Project-wide rule overrides
  {
    rules: {
      // typescript-eslint: relax rules that conflict with Svelte 5 rune patterns
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          destructuredArrayIgnorePattern: '^_',
        },
      ],
      // Allow 'any' in type positions — tighten later once types are fully annotated
      '@typescript-eslint/no-explicit-any': 'warn',
      // Svelte 5: $state/$derived/$effect are valid "unused" top-level expressions
      'no-unused-expressions': 'off',
    },
  },

  // Ignore generated/build output
  {
    ignores: [
      'node_modules/**',
      '.svelte-kit/**',
      'build/**',
      'dist/**',
      '*.config.js',
      '*.config.ts',
    ],
  }
);
