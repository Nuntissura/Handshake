module.exports = {
  forbidden: [
    {
      name: "no-lib-imports-components",
      from: { path: "^src/lib" },
      to: { path: "^src/components" },
    },
    {
      name: "no-lib-imports-state",
      from: { path: "^src/lib" },
      to: { path: "^src/state" },
    },
    {
      name: "no-state-imports-components",
      from: { path: "^src/state" },
      to: { path: "^src/components" },
    },
  ],
  options: {
    doNotFollow: {
      path: "node_modules",
    },
    exclude: {
      path: "dist",
    },
    tsPreCompilationDeps: true,
    tsConfig: {
      fileName: "tsconfig.json",
    },
    enhancedResolveOptions: {
      extensions: [".ts", ".tsx", ".js", ".jsx", ".json"],
    },
  },
};
