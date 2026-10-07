# Theming — design-system contract

Zesp Connect is themed after **Android 17 / Material You** (the quick-settings
shade look: squircle tiles, pill buttons, thick sliders, soft elevated surfaces,
tactile press feedback). Read this before doing UI work.

## Rules (non-negotiable)

1. **Never hardcode a color.** Use semantic tokens only (`bg-primary`,
   `text-muted-foreground`, `bg-card`, `border`, `bg-success`, ...). The only
   allowed literal color is inside QR rendering (`WirelessPanel`). No
   `bg-blue-500`, `#fff`, `rgb(...)`, or `text-gray-*` in `src/`.
2. **One source of truth for palettes:** `src/app/theme/presets.ts`. Adding,
   editing, or removing a theme = editing that one file (`THEMES` /
   `defineTheme`). Never edit a component to change colors.
3. **`oklch()` only** for token values (needed so Tailwind's `/opacity`
   modifiers resolve via `color-mix`).
4. **Shape language:** cards & dialogs `rounded-[1.75rem]`; buttons / badges /
   pills `rounded-full`; quick tiles `rounded-[1.25rem]`; inputs & menus
   `rounded-2xl`/`rounded-xl`. Prefer the `--radius`-derived scale
   (`rounded-sm/md/lg/xl`) for semantic sizes.
5. **Motion language:** interactive elements use
   `transition-all duration-150 ease-emphasized` and `active:scale-[0.96]`–`[0.97]`
   press feedback. Focus ring = `ring-2 ring-ring ring-offset-2
   ring-offset-background`.
6. **Elevation over borders:** prefer `bg-card` + soft shadow + a subtle
   `border-border/60` over heavy hard borders.
7. **Reuse `QuickTile`** (`src/components/ui/quick-tile.tsx`) for tiles instead
   of re-implementing the squircle control.
8. **No token without a home.** If you add a `--color-*` entry to `@theme
   inline` in `src/styles/globals.css`, add the matching key to `ThemeTokens`
   (`types.ts`) and every preset — otherwise themes can't control it.
9. **Filled surfaces use the container pair.** Any filled / "on" control
   (button `default`, active tab, checked switch, `QuickTile`, badge `default`,
   active chip, toast action) uses `bg-primary-container
   text-on-primary-container` — never `bg-primary text-primary-foreground`. In
   dark mode `--primary` is a light pastel, so a bright fill with dark text
   looks wrong; `--primary-container` darkens to a muted accent so on-colors
   stay legible. Keep `--primary` for text/icons, sliders, rings and selection.

## How it works

```
store (themeId, themeMode)  ──▶  ThemeProvider  ──▶  writes CSS vars on <html>
                                   │                  + toggles .dark class
                                   └── reads palette from presets.ts
```

- Palettes: `presets.ts` (⭐ edit only this to add/change a theme).
- Types: `types.ts` (`ThemeTokens` keys are literal CSS custom-property names).
- Runtime: `provider.tsx` applies the active preset + resolves `system` mode via
  `prefers-color-scheme`, sets `color-scheme`, and toggles `.dark` (which powers
  the `dark:` variants defined by `@custom-variant dark` in `globals.css`).
- State: `themeId` / `themeMode` live in `src/app/store.ts`, persisted under the
  `zesp-connect-ui` localStorage key. `index.html` has a pre-paint guard that
  applies the persisted mode before React mounts to avoid a flash.
- UI: `AppearanceCard` (Settings) and `ThemeModeToggle` (header).

`src/styles/globals.css` keeps `:root` / `.dark` **only as a pre-hydration
fallback**; the real, switchable values come from `presets.ts`.
