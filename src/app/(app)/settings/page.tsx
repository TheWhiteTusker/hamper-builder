import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { PageHeader } from "@/components/ui";
import { CompanyForm } from "./_forms/company-form";
import { TermsForm } from "./_forms/setting-forms";
import { CategoryList } from "./_forms/category-list";
import { UserRoleForm } from "./_forms/user-role-form";
import { UpdateAppButton } from "./_forms/update-app-button";
import { PickListManager } from "./pick-list-manager";
import type { Category, Profile } from "@/lib/types";
import { APP_VERSION } from "@/lib/version";

// The tab lives in the URL (?tab=users), so it survives a refresh and can be linked to.
const TABS = [
  { key: "general", label: "General" },
  { key: "categories", label: "Categories" },
  { key: "picklists", label: "Pick-lists" },
  { key: "users", label: "Users" },
  { key: "tools", label: "Tools" },
] as const;
type Tab = (typeof TABS)[number]["key"];

const TOOLS = [
  { href: "/admin/import", title: "Import spreadsheet", hint: "Bring across products, saved hampers and past quotations from the old workbook." },
  { href: "/admin/refresh-prices", title: "Refresh prices", hint: "Update saved hampers with current product prices." },
  { href: "/cost-calculator/master", title: "Rates & Hierarchy Master", hint: "Costing types, categories, varieties and their rates." },
  { href: "/bin", title: "Recycle Bin", hint: "Restore deleted hampers, products and photos. Items are kept for 30 days." },
];

export default async function SettingsPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { supabase, profile } = await requireRole("admin");
  const requested = (await searchParams).tab;
  const tab: Tab = TABS.find((t) => t.key === requested)?.key ?? "general";
  const desktop = process.env.LATTICE_DESKTOP === "1"; // set by electron/main.cjs

  return (
    <>
      <PageHeader title="Settings" subtitle="Company details, categories, pick-lists, users and tools" />

      <nav aria-label="Settings sections" className="mb-4 flex flex-wrap gap-1 rounded-lg bg-slate-100 p-1">
        {TABS.map((t) => (
          <Link
            key={t.key}
            href={t.key === "general" ? "/settings" : `/settings?tab=${t.key}`}
            aria-current={t.key === tab ? "page" : undefined}
            className={`rounded-md px-3 py-1.5 text-sm font-semibold transition-all ${
              t.key === tab
                ? "bg-white text-[var(--color-brand-dark)] shadow-sm"
                : "text-[var(--color-muted)] hover:text-[var(--color-ink)]"
            }`}
          >
            {t.label}
          </Link>
        ))}
      </nav>

      {tab === "general" && <GeneralTab supabase={supabase} />}
      {tab === "categories" && <CategoriesTab supabase={supabase} />}
      {tab === "picklists" && <PickListsTab supabase={supabase} />}
      {tab === "users" && <UsersTab supabase={supabase} selfId={profile.id} />}
      {tab === "tools" && (
        <section className="card divide-y divide-[var(--color-line)]">
          {TOOLS.map((t) => (
            <div key={t.href} className="flex flex-wrap items-center justify-between gap-3 p-4">
              <div>
                <h2 className="text-sm font-semibold">{t.title}</h2>
                <p className="mt-0.5 text-xs text-(--color-muted)">{t.hint}</p>
              </div>
              <Link href={t.href} className="btn-secondary">
                Open →
              </Link>
            </div>
          ))}
          <div className="flex flex-wrap items-center justify-between gap-3 p-4">
            <div>
              <h2 className="text-sm font-semibold">App</h2>
              {/* Only the desktop app has a real version: CI bumps package.json for
                  the desktop build alone, so the website would always read 0.2.4. */}
              <p className="mt-0.5 text-xs text-(--color-muted)">
                Lattice Lane Hamper Builder
                {desktop && (
                  <>
                    {" "}· Version <span className="font-mono font-medium text-(--color-brand)">v{APP_VERSION}</span>
                  </>
                )}
              </p>
            </div>
            <UpdateAppButton desktop={desktop} />
          </div>
        </section>
      )}
    </>
  );
}

type Supabase = Awaited<ReturnType<typeof requireRole>>["supabase"];

async function GeneralTab({ supabase }: { supabase: Supabase }) {
  const settings = await loadSettings(supabase);
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <CompanyForm company={settings.company} gstRate={settings.gst_rate} />
      <div className="grid gap-4">
        <TermsForm settingKey="quote_terms" title="Default quotation terms" terms={settings.quote_terms} />
        <TermsForm settingKey="pi_terms" title="Default proforma invoice terms" terms={settings.pi_terms} />
      </div>
    </div>
  );
}

async function CategoriesTab({ supabase }: { supabase: Supabase }) {
  const { data: categories } = await supabase
    .from("categories")
    .select("*")
    .order("sort_order")
    .order("name")
    .returns<Category[]>();
  return (
    <section className="card overflow-hidden">
      <div className="border-b border-line px-4 py-3">
        <h2 className="text-sm font-semibold">Categories</h2>
        <p className="mt-0.5 text-xs text-(--color-muted)">
          &ldquo;Counts as an item&rdquo; decides what goes into a hamper&rsquo;s item count. Turn it
          off for boxes, filler and ribbon: they add cost, but a client does not count the box as
          one of the gifts.
        </p>
      </div>
      <CategoryList categories={categories ?? []} />
    </section>
  );
}

async function PickListsTab({ supabase }: { supabase: Supabase }) {
  const settings = await loadSettings(supabase);
  return (
    <section className="card p-4">
      <h2 className="text-sm font-semibold">Pick-lists</h2>
      <p className="mt-0.5 text-xs text-(--color-muted)">
        Manage the standard options that fill dropdowns across the app.
      </p>
      <div className="mt-4">
        <PickListManager settings={settings} />
      </div>
    </section>
  );
}

async function UsersTab({ supabase, selfId }: { supabase: Supabase; selfId: string }) {
  const { data: profiles } = await supabase.from("profiles").select("*").order("created_at").returns<Profile[]>();
  return (
    <section className="card overflow-hidden">
      <div className="border-b border-line px-4 py-3">
        <h2 className="text-sm font-semibold">Users</h2>
        <p className="mt-0.5 text-xs text-(--color-muted)">
          People sign up themselves and start as Sales. Once your team is set up, turn off new
          sign-ups in the Supabase dashboard under Authentication → Providers.
        </p>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>Can access</TableHead>
            <TableHead>Role</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {(profiles ?? []).map((p) => (
            <UserRoleForm key={p.id} profile={p} isSelf={p.id === selfId} />
          ))}
        </TableBody>
      </Table>
    </section>
  );
}
