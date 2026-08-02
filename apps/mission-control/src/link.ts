/**
 * Routing via an in-memory "link" — pure browser navigation without react-router.
 *
 * Each route is just a string id. We expose `Link.go(name)` and `Link.use()`
 * so any component can navigate. Keeping routing inside the store means we
 * don't need a router dependency and the UI renders in tests without setup.
 */

const listeners = new Set<(name: string) => void>();
let current: string = (() => {
  if (typeof window === "undefined") return "dashboard";
  const h = window.location.hash.replace(/^#/, "");
  if (!h) return "dashboard";
  return h;
})();

export const Link = {
  current(): string { return current; },
  go(name: string): void {
    current = name;
    if (typeof window !== "undefined") {
      window.location.hash = "#" + name;
    }
    listeners.forEach((l) => l(name));
  },
  subscribe(l: (name: string) => void): () => void { listeners.add(l); return () => listeners.delete(l); },
};

export function useRoute(): string {
  const [route] = useStateLinking();
  return route;
}

import { useEffect, useState } from "react";
function useStateLinking(): [string, (v: string) => void] {
  const [s, set] = useState(current);
  useEffect(() => Link.subscribe(set), []);
  if (typeof window !== "undefined") {
    window.addEventListener("hashchange", () => set(window.location.hash.replace(/^#/, "") || "dashboard"));
  }
  return [s, set];
}
