import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { Snippet } from "svelte";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// Type helpers used by shadcn-svelte components
export type WithElementRef<T, El extends HTMLElement = HTMLElement> = T & {
  ref?: El | null;
};

type ChildrenOrChild =
  | { children?: Snippet }
  | { child?: Snippet };

export type WithoutChildrenOrChild<T> = Omit<T, "children" | "child">;

export type WithoutChild<T> = Omit<T, "child">;
