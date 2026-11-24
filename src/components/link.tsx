"use client";

import NextLink, { useLinkStatus } from "next/link";
import { useState, type ComponentProps } from "react";

// No prefetch on sight: each one is a Worker run, and bursts of them (every nav
// link at once) hit the free plan's 10 ms CPU limit, which kills real page loads
// and saves. Hovering or focusing a link is intent, so that one link prefetches
// then (and on touch, which fires the same mouseenter).
export default function Link({ prefetch, onMouseEnter, onFocus, children, ...props }: ComponentProps<typeof NextLink>) {
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
    >
      {children}
      <Pending />
    </NextLink>
  );
}

/**
 * Without prefetch the old page sits there until the Worker renders the new
 * one, which reads as a hung app. This marker shows the loading bar in
 * loading.css while the clicked link's page is on its way.
 */
function Pending() {
  return useLinkStatus().pending ? <span className="nav-pending" hidden /> : null;
}
