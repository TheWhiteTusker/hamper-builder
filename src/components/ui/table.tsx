import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// shadcn/ui's Table (https://ui.shadcn.com/docs/components/table), restyled to
// the app's palette. No scroll wrapper: pages already put tables in a card with
// overflow-x-auto. Add `num` to a head or cell for right-aligned figures.

export function Table({ className, ...props }: ComponentProps<"table">) {
  return <table data-slot="table" className={cn("w-full border-collapse text-sm", className)} {...props} />;
}

export function TableHeader(props: ComponentProps<"thead">) {
  return <thead data-slot="table-header" {...props} />;
}

export function TableBody(props: ComponentProps<"tbody">) {
  return <tbody data-slot="table-body" {...props} />;
}

export function TableRow(props: ComponentProps<"tr">) {
  return <tr data-slot="table-row" {...props} />;
}

export function TableHead({ className, ...props }: ComponentProps<"th">) {
  return (
    <th
      data-slot="table-head"
      className={cn(
        "border-b border-line px-2.5 py-2 text-left text-[11px] font-semibold uppercase tracking-wide text-muted [&.num]:text-right [&.num]:tabular-nums",
        className,
      )}
      {...props}
    />
  );
}

export function TableCell({ className, ...props }: ComponentProps<"td">) {
  return (
    <td
      data-slot="table-cell"
      className={cn("border-b border-line px-2.5 py-1.5 align-middle [&.num]:text-right [&.num]:tabular-nums", className)}
      {...props}
    />
  );
}
