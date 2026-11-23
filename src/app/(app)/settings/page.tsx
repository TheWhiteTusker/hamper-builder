import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { PageHeader } from "@/components/ui";
import { CompanyForm } from "./_forms/company-form";
import { TermsForm } from "./_forms/setting-forms";
import { CategoryList } from "./_forms/category-list";
import { UserRoleForm } from "./_forms/user-role-form";
import { ToolsTab } from "./_forms/tools-tab";
import { KeyBindings } from "./_forms/key-bindings";
import { PickListManager } from "./pick-list-manager";
import { resolveColors } from "@/lib/product-code";
import { loadCostRows } from "../cost-calculator/load";
import { CostMasterView } from "../cost-calculator/master/master-view";
import type { Category, Profile } from "@/lib/types";
import { APP_VERSION, latestDesktopVersion } from "@/lib/version";

// Most-used first; the first tab is what /settings opens on. The tab lives in
// the URL (?tab=users), so it survives a refresh and can be linked to.
const TABS = [
  { key: "rates", label: "Rates", hint: "Material, hardware, finishing and machine rates used to cost products." },
  { key: "dropdowns", label: "Dropdowns", hint: "The options people pick from across the app: occasions, vendors, colours, statuses." },
  { key: "categories", label: "Categories", hint: "Product categories, and whether each counts towards a hamper's item count." },
  { key: "tools", label: "Tools", hint: "Recycle Bin, price refresh and app updates." },
  { key: "users", label: "Users", hint: "Who can sign in and what they can do." },
  { key: "company", label: "Company & terms", hint: "Your letterhead details, GST and the terms printed on quotes and invoices." },
  { key: "keys", label: "Key bindings", hint: "Shortcuts and mouse buttons for the app, like a game's controls." },
] as const;
type Tab = (typeof TABS)[number]["key"];

export default async function SettingsPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const { supabase, profile } = await requireRole("admin");
  const requested = (await searchParams).tab;
  const current = TABS.find((t) => t.key === requested) ?? TABS[0];
  const tab: Tab = current.key;
  const desktop = process.env.LATTICE_DESKTOP === "1"; // set by electron/main.cjs
  const version = tab !== "tools" ? null : desktop ? APP_VERSION : await latestDesktopVersion();

  return (
    <>
      <PageHeader title="Settings" />

      <nav aria-label="Settings sections" className="flex flex-wrap gap-1 rounded-lg bg-slate-100 p-1">
        {TABS.map((t) => (
          <Link
            key={t.key}
            href={t.key === TABS[0].key ? "/settings" : `/settings?tab=${t.key}`}
            aria-current={t.key === tab ? "page" : undefined}
            className={`rounded-md px-3 py-1.5 text-sm font-semibold transition-all ${
              t.key === tab
                ? "bg-white text-(--color-brand-dark) shadow-sm"
                : "text-(--color-muted) hover:text-(--color-ink)"
            }`}
          >
            {t.label}
          </Link>
        ))}
      </nav>
      <p className="mt-2 mb-4 text-sm text-(--color-muted)">{current.hint}</p>

      {tab === "dropdowns" && <PickListsTab supabase={supabase} />}
      {tab === "rates" && <RatesTab supabase={supabase} />}
      {tab === "categories" && <CategoriesTab supabase={supabase} />}
      {tab === "company" && <GeneralTab supabase={supabase} />}
      {tab === "users" && <UsersTab supabase={supabase} selfId={profile.id} />}
      {tab === "tools" && <ToolsTab desktop={desktop} version={version} />}
      {tab === "keys" && <KeyBindings />}
    </>
  );
}

type Supabase = Awaited<ReturnType<typeof requireRole>>["supabase"];

async function GeneralTab({ supabase }: { supabase: Supabase }) {
  const settings = await loadSettings(supabase);
  return (
    // Terms change more often than the letterhead, so they come first.
    <div className="grid gap-4 lg:grid-cols-2">
      <div className="grid content-start gap-4">
        <TermsForm settingKey="quote_terms" title="Quote terms" terms={settings.quote_terms} />
        <TermsForm settingKey="pi_terms" title="Proforma invoice terms" terms={settings.pi_terms} />
      </div>
      <CompanyForm company={settings.company} gstRate={settings.gst_rate} />
    </div>
  );
}

/** The same Rates screen as Cost Calculator → Rates; Settings is admin-only. */
async function RatesTab({ supabase }: { supabase: Supabase }) {
  const [rows, settings] = await Promise.all([loadCostRows(supabase), loadSettings(supabase)]);
  return <CostMasterView rows={rows} admin productColors={resolveColors(settings.product_colors, settings.color_hex)} />;
}

async function CategoriesTab({ supabase }: { supabase: Supabase }) {
  const { data: categories } = await supabase
    .from("categories")
    .select("*")
    .order("sort_order")
    .order("name")
    .overrideTypes<Category[], { merge: false }>();
  return (
    <section className="card overflow-hidden">
      <p className="border-b border-line px-4 py-3 text-xs text-(--color-muted)">
        Turn <strong>Counts as an item</strong> off for boxes, filler and ribbon: they add cost, but
        a client doesn&rsquo;t count the box as one of the gifts.
      </p>
      <CategoryList categories={categories ?? []} />
    </section>
  );
}

async function PickListsTab({ supabase }: { supabase: Supabase }) {
  const settings = await loadSettings(supabase);
  return (
    <section className="card p-4">
      <PickListManager settings={settings} />
    </section>
  );
}

async function UsersTab({ supabase, selfId }: { supabase: Supabase; selfId: string }) {
  const { data: profiles } = await supabase.from("profiles").select("*").order("created_at").overrideTypes<Profile[], { merge: false }>();
  return (
    <section className="card overflow-hidden">
      <p className="border-b border-line px-4 py-3 text-xs text-(--color-muted)">
        New people sign up themselves and start as Sales; change their role here. Once your team is
        set up, turn off new sign-ups in Supabase (Authentication → Providers).
      </p>
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
