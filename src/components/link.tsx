import NextLink from "next/link";
import type { ComponentProps } from "react";

// No prefetch by default: each one is a Worker run, and bursts of them hit the
// free plan's 10 ms CPU limit, which kills real page loads and saves.
export default function Link({ prefetch = false, ...props }: ComponentProps<typeof NextLink>) {
  return <NextLink prefetch={prefetch} {...props} />;
}
