import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  {
    ignores: [
      '**/dist/**',
      '**/.vitepress/cache/**',
      '**/.vitepress/dist/**',
      '**/node_modules/**',
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx,mts,cts,js,mjs,cjs}'],
    languageOptions: {
      parserOptions: {
        onUnsupportedTypeScriptVersion: 'error',
      },
    },
  },
);
