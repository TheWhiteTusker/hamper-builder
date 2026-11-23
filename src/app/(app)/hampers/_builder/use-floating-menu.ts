import { useEffectEvent, useLayoutEffect, useState, type RefObject } from "react";

export type MenuCoords = { top: number; left: number; width: number; transform: string };

/**
 * Where a fixed-position menu sits: under its input, or above when there's
 * no room below. Follows scrolling and resizing while open, and calls
 * `onOutside` for a click outside both the input's box and the menu.
 * A layout effect, so the menu is placed before it's first painted.
 */
export function useFloatingMenu(
  isOpen: boolean,
  refs: {
    input: RefObject<HTMLInputElement | null>;
    container: RefObject<HTMLDivElement | null>;
    menu: RefObject<HTMLDivElement | null>;
  },
  onOutside: () => void,
) {
  const [coords, setCoords] = useState<MenuCoords>({ top: 0, left: 0, width: 0, transform: "none" });
  const { input, container, menu } = refs;
  const outside = useEffectEvent(onOutside);

  useLayoutEffect(() => {
    if (!isOpen) return;

    const place = () => {
      if (!input.current) return;
      const rect = input.current.getBoundingClientRect();
      const dropdownMaxHeight = 250;
      const spaceBelow = window.innerHeight - rect.bottom;
      const showAbove = spaceBelow < dropdownMaxHeight && rect.top > dropdownMaxHeight;
      const width = Math.max(rect.width, 280);
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
      const top = showAbove ? rect.top - 4 : rect.bottom + 4;
      setCoords({ top, left, width, transform: showAbove ? "translateY(-100%)" : "none" });
    };
    const handleScroll = (e: Event) => {
      if (menu.current?.contains(e.target as Node)) return;
      place();
    };
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as Node;
      if (container.current && !container.current.contains(target) && menu.current && !menu.current.contains(target)) {
        outside();
      }
    };

    place();
    window.addEventListener("scroll", handleScroll, true);
    window.addEventListener("resize", place);
    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      window.removeEventListener("scroll", handleScroll, true);
      window.removeEventListener("resize", place);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen, input, container, menu]);

  return coords;
}
