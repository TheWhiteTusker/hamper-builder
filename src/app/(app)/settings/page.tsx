import { Table, TableBody, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireRole } from "@/lib/supabase/server";
import { loadSettings } from "@/lib/settings";
import { PageHeader } from "@/components/ui";
import { CompanyForm } from "./_forms/company-form";
import { TermsForm, DefaultForm } from "./_forms/setting-forms";
import { CategoryRow, AddCategoryForm } from "./_forms/category-forms";
import { UserRoleForm } from "./_forms/user-role-form";
import { UpdateAppButton } from "./_forms/update-app-button";
import { PickListManager } from "./pick-list-manager";
import type { Category, Profile } from "@/lib/types";
import { APP_VERSION } from "@/lib/version";

export default async function SettingsPage() {
  const { supabase, profile } = await requireRole("admin");

  const [settings, { data: categories }, { data: profiles }] = await Promise.all([
    loadSettings(supabase),
    supabase.from("categories").select("*").order("sort_order").order("name").returns<Category[]>(),
    supabase.from("profiles").select("*").order("created_at").returns<Profile[]>(),
  ]);

  const desktop = process.env.LATTICE_DESKTOP === "1";
  const nextSortOrder =(categories ?? []).reduce((max, c) => Math.max(max, c.sort_order), -1) + 1;

  return (
    <>
      <PageHeader title="Settings" subtitle="Company details, categories, pick-lists and users">
        <Link href="/admin/import" className="btn-secondary">
          Import spreadsheet
        </Link>
        <Link href="/admin/refresh-prices" className="btn-secondary">
          Refresh prices
        </Link>
        <Link href="/cost-calculator/master" className="btn-secondary">
          Rates & Hierarchy Master
        </Link>
        <Link href="/bin" className="btn-secondary">
          Recycle Bin
        </Link>
        {/* LATTICE_DESKTOP is set by electron/main.cjs. */}
        <UpdateAppButton desktop={desktop} />
      </PageHeader>

      <div className="grid gap-4 lg:grid-cols-2">
        <CompanyForm company={settings.company} gstRate={settings.gst_rate} />
        <div className="grid gap-4">
          <TermsForm
            settingKey="quote_terms"
            title="Default quotation terms"
            terms={settings.quote_terms}
          />
          <TermsForm
            settingKey="pi_terms"
            title="Default proforma invoice terms"
            terms={settings.pi_terms}
          />
        </div>
      </div>

      {/* ---------------- categories ---------------- */}
      <section className="card mt-4 overflow-hidden">
        <div className="border-b border-line px-4 py-3">
          <h2 className="text-sm font-semibold">Categories</h2>
          <p className="mt-0.5 text-xs text-(--color-muted)">
            &ldquo;Counts as an item&rdquo; decides what goes into a hamper&rsquo;s item count.
            Turn it off for boxes, filler and ribbon: they add cost, but a client does not
            count the box as one of the gifts.
          </p>
        </div>

        <Table>
          <TableBody>
            {(categories ?? []).map((category) => (
              <CategoryRow key={category.id} category={category} />
            ))}
          </TableBody>
        </Table>

        {(categories ?? []).length === 0 && (
          <p className="px-4 py-6 text-sm text-(--color-muted)">
            No categories yet. Import the Settings tab, or add them here.
          </p>
        )}

        <div className="border-t border-line">
          <AddCategoryForm nextSortOrder={nextSortOrder} />
        </div>
      </section>

      {/* ---------------- pick-lists ---------------- */}
      <section className="card mt-4 p-4">
        <h2 className="text-sm font-semibold">Pick-lists</h2>
        <p className="mt-0.5 text-xs text-(--color-muted)">
          Manage the standard options that fill dropdowns across the app.
        </p>

        <div className="mt-4">
          <PickListManager settings={settings} />
        </div>

        <div className="mt-6 border-t border-line pt-4">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-(--color-muted)">
            Default Selections
          </h3>
          <div className="mt-3 grid gap-3 md:grid-cols-3">
            <DefaultForm
              settingKey="default_detail_mode"
              label="Default contents setting"
              value={settings.default_detail_mode}
              options={settings.detail_modes}
            />
            <DefaultForm
              settingKey="default_packaging_treatment"
              label="Default packaging treatment"
              value={settings.default_packaging_treatment}
              options={settings.packaging_treatments}
            />
            <DefaultForm
              settingKey="default_validity"
              label="Default validity"
              value={settings.default_validity}
              options={settings.validity_options}
            />
          </div>
        </div>
      </section>

      {/* ---------------- users ---------------- */}
      <section className="card mt-4 overflow-hidden">
        <div className="border-b border-line px-4 py-3">
          <h2 className="text-sm font-semibold">Users</h2>
          <p className="mt-0.5 text-xs text-(--color-muted)">
            People sign up themselves and start as Sales. Once your team is set up, turn off
            new sign-ups in the Supabase dashboard under Authentication → Providers.
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
              <UserRoleForm key={p.id} profile={p} isSelf={p.id === profile.id} />
            ))}
          </TableBody>
        </Table>
      </section>

      {/* ---------------- recycle bin ---------------- */}
      <section className="card mt-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold">Recycle Bin</h2>
            <p className="mt-0.5 text-xs text-(--color-muted)">
              View and restore deleted hampers, products, and photos. Items are kept for 30 days before permanent deletion.
            </p>
          </div>
          <Link href="/bin" className="btn-secondary">
            Open Recycle Bin →
          </Link>
        </div>
      </section>

      {/* ---------------- app version footer ---------------- */}
      <footer className="mt-8 mb-4 border-t border-line pt-6 text-center text-xs text-(--color-muted)">
        <p className="font-semibold text-(--color-ink)">
          Lattice Lane Hamper Builder
        </p>
        {/* Only the desktop app has a real version: CI bumps package.json for
            the desktop build alone, so the website would always read 0.2.4. */}
        {desktop && (
          <p className="mt-1">
            Version <span className="font-mono font-medium text-(--color-brand)">v{APP_VERSION}</span>
          </p>
        )}
      </footer>
    </>
  );
}
