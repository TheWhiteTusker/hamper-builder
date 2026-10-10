"use client";

import { TableCell, TableRow } from "@/components/ui/table";
import { useActionState } from "react";
import type { Profile, UserRole } from "@/lib/types";
import { setUserRole } from "../category-actions";
import { Status } from "./status";

const ROLE_HINT: Record<UserRole, string> = {
  admin: "Everything, including products, settings and users",
  manager: "Hampers, all quotations, and quote status",
  sales: "Their own quotations",
};

export function UserRoleForm({ profile, isSelf }: { profile: Profile; isSelf: boolean }) {
  const [state, action, pending] = useActionState(setUserRole, {});

  return (
    <TableRow>
      <TableCell>
        {profile.full_name || "—"}
        {isSelf && <span className="badge ml-2">you</span>}
      </TableCell>
      <TableCell className="text-[var(--color-muted)]">{ROLE_HINT[profile.role]}</TableCell>
      <TableCell>
        <form action={action} className="flex items-center gap-2">
          <input type="hidden" name="id" value={profile.id} />
          <select name="role" defaultValue={profile.role} className="select max-w-[140px]">
            <option value="sales">Sales</option>
            <option value="manager">Manager</option>
            <option value="admin">Admin</option>
          </select>
          <button type="submit" className="btn-secondary" disabled={pending}>
            {pending ? "Saving…" : "Save"}
          </button>
          <Status state={state} />
        </form>
      </TableCell>
    </TableRow>
  );
}
