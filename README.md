# Lattice Lane

Hamper costing and client quotations. This replaces the Google Sheet and its
Apps Script (archived at [docs/legacy-apps-script.js](docs/legacy-apps-script.js)).

Next.js 16 + Supabase (Postgres, Auth, RLS). No API layer — the app talks to
Postgres directly, and all pricing lives in database views so it cannot drift.

---

## Setup

**1. Create a Supabase project** at [supabase.com](https://supabase.com).

**2. Run the migrations**, in order, in the SQL editor (Database → SQL Editor):

| File | What it creates |
| --- | --- |
| `supabase/migrations/0001_schema.sql` | Tables, views, document numbering |
| `supabase/migrations/0002_rls.sql` | Row level security and the role matrix |
| `supabase/migrations/0003_seed.sql` | Default settings and pick-lists |
| `supabase/migrations/0004_rpc.sql` | Transactional save / duplicate / refresh |
| `supabase/migrations/0005_rls_performance.sql` | Indexes and per-statement RLS checks |
| `supabase/migrations/0006_fix_new_user_role_cast.sql` | Cast the new-user role to its enum |
| `supabase/migrations/0007_guards_allow_direct_sql.sql` | Let direct SQL administer roles and status |

With the Supabase CLI instead: `supabase db push`.

**3. Configure the app:**

```bash
cp .env.local.example .env     # then fill in from Project Settings -> API
pnpm install
pnpm dev
```

Two env files:

| File | Committed | Holds |
| --- | :---: | --- |
| `.env.production` | **yes** | The two `NEXT_PUBLIC_` values, so every build has them |
| `.env` | no | Local overrides and tooling secrets (Cloudflare, R2, Supabase tokens) |

A local `cf:build` copies every value in `.env` into the Worker bundle, so don't
deploy from your machine: push to `main` and let Workers Builds deploy, since it
never sees `.env`.

**4. Create the first account.** Open <http://localhost:3015>, choose *Create an
account*. The first person to sign up becomes the **admin** — everyone after
starts as **sales**. Once the team is in, turn off open sign-ups in Supabase
under Authentication → Providers → Email.

**5. Import the spreadsheet.** Sign in, go to Settings → Import spreadsheet, and
follow the three steps there. Export each tab from Google Sheets as CSV first
(File → Download → Comma-separated values).

The importer reads columns **by position**, matching the indices the Apps Script
used, so renamed headers are fine but reordered columns are not. It does a dry
run before writing anything, matches on product code / hamper code / document
number (so re-running updates rather than duplicates), and afterwards sets
numbering to continue from your highest existing `H###`, `LLQT-###`, `LLPI-###`.

---

## Roles

Everyone signed in can read everything — the team needs to see costs to quote.
Writes are enforced by RLS policies in the database, not just hidden in the UI.

| | Sales | Manager | Admin |
| --- | :---: | :---: | :---: |
| Create quotations, edit their own | ● | ● | ● |
| Edit anyone's quotation, change status | | ● | ● |
| Build and edit hampers, approve final prices | | ● | ● |
| Product Master and cost prices | | | ● |
| Settings, users, import, price refresh | | | ● |

---

## How the spreadsheet maps across

| Apps Script | Now |
| --- | --- |
| `saveHamper`, `updateHamper`, `loadHamper`, `newHamper` | `/hampers/[code]` — one Save that creates or updates |
| `duplicateHamper` | Duplicate button — makes a real copy, not just a renamed builder |
| `onEdit` cascading dropdowns | Filtered in the browser, no server round-trip |
| `populateProductRow` | Selecting a product fills the row |
| `getNextHamperCode`, numbering in `saveQuote` | `next_doc_no()`, row-locked so concurrent saves cannot collide |
| `addToQuote`, `addSelectedHamperToQuote`, `removeHamperFromQuote` | Add / remove lines in the quote builder |
| `newQuote`, `saveQuote` | `/quotes/[docNo]` — and quotes stay **editable**, which the sheet never managed |
| `refreshHamperPrices` | `/admin/refresh-prices`, with a preview before applying |
| `categoryCountsAsItem` | Settings → Categories → "counts as an item" |
| Hamper Summary tab | `hamper_summary` view — always consistent with the lines |
| Detail Mode / Packaging Treatment | Finally used, on the printed quotation |
| 15 products per hamper, 10 hampers per quote | No limit |
| — | Printable / PDF quotation, proforma conversion, follow-up tracking |

### Pricing

Every number comes from `src/lib/pricing.ts` (live totals while typing) and the
matching SQL views (authoritative once saved):

```
total_cp        = Σ qty × unit_cp          gross_profit = final_catalogue_sp − total_cp
base_sp         = Σ qty × unit_sp          final_margin = gross_profit / final_catalogue_sp
sp_after_disc   = base_sp × (1 − discount) number_of_items = Σ qty where category counts

final_rate = catalogue_price × (1 − discount)      taxable = subtotal − discount + adj1 + adj2
amount     = qty × final_rate                      grand   = taxable × (1 + gst_rate)
```

Two behaviours carried over deliberately:

- **Final Catalogue SP is a manual override.** Refreshing prices never touches it.
- **Hamper lines are price snapshots.** A saved hamper does not move when a cost
  changes; it re-syncs only when someone runs Refresh Prices. Order totals are
  option-only for `Combined Order` structures, matching the sheet's
  `=IF(B7="Combined Order", …)` formulas.

---

## Development

```bash
npm run dev        # http://localhost:3015
npm test           # pricing and import mapping, via node --test
npm run typecheck
npm run lint
npm run build
```

Tests use Node's built-in runner — no framework. `src/lib/pricing.test.ts` checks
every formula against a hamper worked by hand; `src/lib/import.test.ts` checks
the column mapping, including reconciling quote totals that disagree with their
own line items.

## Deploying

Cloudflare Workers, via [OpenNext](https://opennext.js.org/cloudflare).

```bash
pnpm run cf:build     # next build + the Worker bundle in .open-next/
pnpm run cf:preview   # run it locally on workerd
npx wrangler deploy   # needs CLOUDFLARE_API_TOKEN in .env; ships .env too, prefer CI
```

Through Workers Builds instead, set the **build command** to `pnpm run cf:build`
— plain `next build` does not produce `.open-next/`. Nothing else needs
configuring: the `NEXT_PUBLIC_` values come from the committed
`.env.production`.

Do not move them into dashboard variables. They are compiled in when the build
runs, so a *runtime* variable arrives too late to have any effect, and a *build*
variable reintroduces a hand-typed name that fails silently — one missing letter
builds green and serves 503 on every page. If a deploy target genuinely needs
different values, a real environment variable still overrides the file.

`wrangler deploy` also replaces the Worker's configuration with `wrangler.jsonc`,
so any variable added in the dashboard is deleted on the next deploy.

Then add the deployed URL to Supabase under Authentication → URL Configuration.

On Windows, `cf:build` fails with `EPERM: symlink` because OpenNext symlinks
pnpm's nested `node_modules`. Add `nodeLinker: hoisted` to `pnpm-workspace.yaml`
and reinstall to build locally.
