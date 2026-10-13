"use client";

import { TableCell, TableRow } from "@/components/ui/table";
import { useActionState, useState } from "react";
import type { ActionState } from "@/lib/forms";
import type { Profile, UserRole } from "@/lib/types";
import { setUserRole } from "../category-actions";
import { Status, keepValues } from "./status";

const ROLE_HINT: Record<UserRole, string> = {
  admin: "Everything, including products, settings and users",
  manager: "Hampers, all quotations, and quote status",
  sales: "Their own quotations",
};

export function UserRoleForm({ profile, isSelf }: { profile: Profile; isSelf: boolean }) {
  // The hint follows the saved role; nothing re-renders the page after a save.
  const [role, setRole] = useState(profile.role);
  const [state, action, pending] = useActionState(async (prev: ActionState, formData: FormData) => {
    const res = await setUserRole(prev, formData);
    if (res.ok) setRole(formData.get("role") as UserRole);
    return res;
  }, {});

  return (
    <TableRow>
      <TableCell>
        {profile.full_name || "—"}
        {isSelf && <span className="badge ml-2">you</span>}
      </TableCell>
      <TableCell className="text-[var(--color-muted)]">{ROLE_HINT[role]}</TableCell>
      <TableCell>
        <form onSubmit={keepValues(action)} className="flex items-center gap-2">
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
