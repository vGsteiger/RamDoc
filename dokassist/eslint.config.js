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

  // Svelte files (and rune modules, *.svelte.ts): tell the svelte parser to use the TS parser
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
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

  // UI primitives render whatever href their caller passes, so they cannot
  // wrap it in resolve() themselves — resolution belongs at the call site.
  {
    files: ['src/lib/components/ui/**/*.svelte'],
    rules: {
      'svelte/no-navigation-without-resolve': ['error', { ignoreLinks: true }],
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
