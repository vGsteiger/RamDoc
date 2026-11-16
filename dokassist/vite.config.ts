import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    tailwindcss(),
    // SvelteKit 3 no longer reads svelte.config.js; options go to the plugin.
    sveltekit({
      adapter: adapter({
        fallback: "index.html",
      }),
      // Kit 3 replaces `$lib` with the `#lib` subpath import. Keep the alias
      // until the imports are migrated in a dedicated change.
      alias: { $lib: "src/lib" },
    }),
    svelteTesting(),
  ],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/tests/setup.ts"],
    include: ["src/tests/**/*.test.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/lib/**"],
      thresholds: {
        lines: 40,
        functions: 40,
        branches: 30,
      },
    },
  },
});
