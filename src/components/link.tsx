"use client";

import NextLink from "next/link";
import { useState, type ComponentProps } from "react";

// No prefetch on sight: each one is a Worker run, and bursts of them (every nav
// link at once) hit the free plan's 10 ms CPU limit, which kills real page loads
// and saves. Hovering or focusing a link is intent, so that one link prefetches
// then (and on touch, which fires the same mouseenter).
export default function Link({ prefetch, onMouseEnter, onFocus, ...props }: ComponentProps<typeof NextLink>) {
  const [wanted, setWanted] = useState(false);
  return (
    <NextLink
      prefetch={prefetch ?? (wanted ? null : false)}
      onMouseEnter={(e) => {
        setWanted(true);
        onMouseEnter?.(e);
      }}
      onFocus={(e) => {
        setWanted(true);
        onFocus?.(e);
      }}
      {...props}
    />
  );
}
