# AGENTS.md

## Commands
- `bun dev` - Run with hot reload
- `bun start` - Run the application
- No test/lint commands configured

## Code Style

### Imports
- Type imports: `import type { Foo } from "./types"`
- External packages first, then internal modules with `./` prefix

### Naming
- Components/Types: PascalCase (`FlakeList`, `StatusBarProps`)
- Files: PascalCase for components, camelCase for utilities
- Functions/variables: camelCase

### Types
- Explicit type annotations for function parameters and returns
- Union types for constrained values: `type View = "list" | "error"`
- Props interfaces: `interface FooProps { ... }`

### Error Handling
```typescript
try { ... } catch (err) {
  const msg = err instanceof Error ? err.message : String(err);
}
```

### Patterns
- SolidJS functional components with signals (`createSignal`, `createMemo`)
- Async/await with `Promise.all()` for parallel operations
- Bun shell: `await $\`nix flake update\`.text()`
