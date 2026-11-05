/** Join a device directory and an entry name with exactly one separator. */
export function joinPath(base: string, name: string): string {
  return base.endsWith("/") ? `${base}${name}` : `${base}/${name}`;
}

/** The parent of a device directory, never climbing above `/`. */
export function parentPath(path: string): string {
  const trimmed = path.replace(/\/+$/, "");
  const index = trimmed.lastIndexOf("/");
  return index <= 0 ? "/" : trimmed.slice(0, index);
}

/**
 * The parent of `path`, clamped to `root`. Browsing never rises above the root
 * (the device root is not listable), so going up from the root stays put.
 */
export function parentWithin(path: string, root: string): string {
  const parent = parentPath(path);
  return parent.startsWith(root) ? parent : root;
}

export interface Crumb {
  label: string;
  path: string;
}

/** Breadcrumb segments for a device path (`/sdcard/Download` → `sdcard › Download`). */
export function breadcrumbs(path: string): Crumb[] {
  const trimmed = path.replace(/\/+$/, "");
  if (trimmed === "" || trimmed === "/") return [];

  const crumbs: Crumb[] = [];
  let current = "";
  for (const part of trimmed.split("/").filter(Boolean)) {
    current += `/${part}`;
    crumbs.push({ label: part, path: current });
  }
  return crumbs;
}

/** Breadcrumb segments below `root`; the root is rendered separately, so crumbs at or above it are dropped. */
export function breadcrumbsWithin(path: string, root: string): Crumb[] {
  const base = root.replace(/\/+$/, "");
  return breadcrumbs(path).filter(
    (crumb) => crumb.path !== base && !base.startsWith(`${crumb.path}/`),
  );
}

/** The final segment of a host path, used to name an upload destination. */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}
