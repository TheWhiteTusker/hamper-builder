# Graph Report - Lattice Lane Hamper Builder  (2026-11-23)

## Corpus Check
- 487 files · ~170,631 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 8 file(s) not represented in the graph (top: .css 3, .onnx 2, .example 1)

## Summary
- 2082 nodes · 6868 edges · 100 communities (90 shown, 10 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 177 edges (avg confidence: 0.86)
- Token cost: 443,000 input · 0 output

## Community Hubs (Navigation)
- Presentation Deck Slides
- Quote Builder & Pricing
- Hamper Builder Editor
- Studio Stage Rendering
- Canvas Stage & Images
- Magic Layers (SAM Masks)
- Recycle Bin & Permissions
- Postgres Rule Sections
- Product Sheet XLSX Export
- Hamper Canvas Schema
- Tables & Money Formatting
- Studio Toolbar UI
- List Pages & Headers
- Settings & Client Forms
- Studio Editor Panels
- Cost Master Categories
- Product Codes & Colours
- Electron Auto-Updater
- Product Routes & Roles
- Shared Row Types & Live Merge
- Sheet Import Wizards
- Cost Tree & Rate Import
- Dev Dependencies
- Settings Page & Nav
- Feature Set PDF
- Product Details & Thumbs
- Cost Calculator View
- Costing Math & MRP
- Product Image Storage
- Cost Lines Hierarchy
- Cost Master View & Transfers
- Repricing Plan
- Keybindings
- Product Sheet Costing
- Desktop Release Workflow
- Login & Auth Card
- List Row Selection
- Canvas Arrange & Layers
- Migration & Build Scripts
- Cost Stage Tables
- Pick List Manager
- TypeScript Config
- Supabase Agent Skill
- Legacy Apps Script
- Bought-Out Master & Bin Data
- Cost Sheet Save Actions
- Live Refresh Realtime
- shadcn Components Config
- Runtime Dependencies
- Admin Import Wizard UI
- Settings Loader & Print
- Spreadsheet Import Parsing
- Electron Builder Config
- Bulk Costing Import
- Undo & Shortcuts
- README Architecture
- Image Edit Actions
- Supabase Security & Deploy
- Desktop Publish Button
- Product Sheet Formulas
- RLS & Least Privilege
- Signed Uploads & Canvas Save
- Landing Page
- Product Images Manager
- npm Scripts
- pnpm Build Allowlist
- Stage Context Menus
- Query Diagnosis (EXPLAIN/VACUUM)
- Supabase Env & Proxy
- Cost Calculator Tabs
- Root Layout & Service Worker
- Postgres Index Types
- Brand Asset Builder
- Combo Menu
- Product Photo Uploads
- Password Reset
- Amount In Words (INR)
- Composite & Covering Indexes
- Data Types & Constraints
- Primary Key Strategy
- OpenNext Cloudflare Files
- App Shell & Stale Pages
- Lattice Lane Brand Marks
- ESLint Config
- Price Summary & Breakdown
- Background Removal
- Foreign Key Indexes
- Cloudflare Worker Entry
- Lowercase Identifiers
- Electron Preload
- MCP Server Config
- Unsaved Changes Guard
- pg_stat_statements
- Partial Indexes
- React Doctor Config
- Template Icons
- PostCSS Config
- Next/Vercel Template Logos

## God Nodes (most connected - your core abstractions)
1. `react` - 114 edges
2. `Link()` - 93 edges
3. `describeError()` - 83 edges
4. `num()` - 76 edges
5. `createClient()` - 76 edges
6. `formatMoney()` - 67 edges
7. `requireUser` - 62 edges
8. `cx()` - 58 edges
9. `next` - 57 edges
10. `PageHeader()` - 45 edges

## Surprising Connections (you probably didn't know these)
- `Bulk spreadsheet import` --semantically_similar_to--> `Spreadsheet importer`  [INFERRED] [semantically similar]
  docs/Lattice-Lane-Hamper-Builder-Feature-Set.pdf → README.md
- `Roles and row-level security` --semantically_similar_to--> `Role matrix (Sales, Manager, Admin) enforced by RLS`  [INFERRED] [semantically similar]
  docs/Lattice-Lane-Hamper-Builder-Feature-Set.pdf → README.md
- `Collision-free document numbers (H###, LLQT-###, LLPI-###)` --semantically_similar_to--> `next_doc_no() row-locked document numbering`  [INFERRED] [semantically similar]
  docs/Lattice-Lane-Hamper-Builder-Feature-Set.pdf → README.md
- `One set of pricing formulas used everywhere` --semantically_similar_to--> `Hamper and quotation pricing formulas`  [INFERRED] [semantically similar]
  docs/Lattice-Lane-Hamper-Builder-Feature-Set.pdf → README.md
- `Order structure: Combined Order or Option Based` --semantically_similar_to--> `Combined Order structure (option-only totals)`  [INFERRED] [semantically similar]
  docs/Lattice-Lane-Hamper-Builder-Feature-Set.pdf → README.md

## Import Cycles
- 2-file cycle: `src/lib/canvas-render.ts -> src/lib/hamper-canvas.ts -> src/lib/canvas-render.ts`
- 2-file cycle: `src/lib/hamper-canvas-schema.ts -> src/lib/hamper-canvas.ts -> src/lib/hamper-canvas-schema.ts`

## Hyperedges (group relationships)
- **Connection pooling modes and their constraints** — _agents_skills_supabase_postgres_best_practices_references_conn_pooling_use_connection_pooling, _agents_skills_supabase_postgres_best_practices_references_conn_pooling_pgbouncer, _agents_skills_supabase_postgres_best_practices_references_conn_pooling_transaction_mode_pooling, _agents_skills_supabase_postgres_best_practices_references_conn_pooling_session_mode_pooling, _agents_skills_supabase_postgres_best_practices_references_conn_prepared_statements_prepared_statements_with_pooling [INFERRED 0.85]
- **Rule reference authoring contract** — _agents_skills_supabase_postgres_best_practices_references__template_rule_reference_template, _agents_skills_supabase_postgres_best_practices_references__contributing_error_first_structure, _agents_skills_supabase_postgres_best_practices_references__contributing_quantified_impact, _agents_skills_supabase_postgres_best_practices_references__contributing_impact_level_guidelines, _agents_skills_supabase_postgres_best_practices_references__contributing_review_checklist [INFERRED 0.85]
- **Timeout-based lock and connection slot reclamation** — _agents_skills_supabase_postgres_best_practices_references_conn_idle_timeout_configure_idle_connection_timeouts, _agents_skills_supabase_postgres_best_practices_references_conn_idle_timeout_idle_in_transaction_session_timeout, _agents_skills_supabase_postgres_best_practices_references_lock_short_transactions_keep_transactions_short, _agents_skills_supabase_postgres_best_practices_references_lock_short_transactions_statement_timeout [INFERRED 0.75]
- **Postgres index access methods** — _agents_skills_supabase_postgres_best_practices_references_query_index_types_btree_index, _agents_skills_supabase_postgres_best_practices_references_query_index_types_gin_index, _agents_skills_supabase_postgres_best_practices_references_query_index_types_gist_index, _agents_skills_supabase_postgres_best_practices_references_query_index_types_brin_index, _agents_skills_supabase_postgres_best_practices_references_query_index_types_hash_index [EXTRACTED 1.00]
- **Slow query diagnosis workflow** — _agents_skills_supabase_postgres_best_practices_references_monitor_pg_stat_statements_pg_stat_statements, _agents_skills_supabase_postgres_best_practices_references_monitor_explain_analyze_explain_analyze, _agents_skills_supabase_postgres_best_practices_references_monitor_explain_analyze_seq_scan, _agents_skills_supabase_postgres_best_practices_references_monitor_vacuum_analyze_analyze [INFERRED 0.85]
- **Performant RLS policy pattern** — _agents_skills_supabase_postgres_best_practices_references_security_rls_basics_row_level_security, _agents_skills_supabase_postgres_best_practices_references_security_rls_basics_auth_uid, _agents_skills_supabase_postgres_best_practices_references_security_rls_performance_select_wrapped_auth_uid, _agents_skills_supabase_postgres_best_practices_references_security_rls_performance_security_definer_function, _agents_skills_supabase_postgres_best_practices_references_security_rls_performance_rls_policy_column_index [INFERRED 0.85]
- **Products to costing to hampers to quotes to presentations flow** — docs_lattice_lane_hamper_builder_feature_set_product_master, docs_lattice_lane_hamper_builder_feature_set_cost_calculator, docs_lattice_lane_hamper_builder_feature_set_hamper_builder, docs_lattice_lane_hamper_builder_feature_set_quotations_proformas, docs_lattice_lane_hamper_builder_feature_set_presentation_studio [EXTRACTED 1.00]
- **RLS, views and privileged database code traps** — _agents_skills_supabase_skill_security_invoker_views, _agents_skills_supabase_skill_update_requires_select_policy, _agents_skills_supabase_skill_auth_role_deprecation, _agents_skills_supabase_skill_bola_ownership_predicate, _agents_skills_supabase_skill_update_with_check, _agents_skills_supabase_skill_security_definer_bypass [EXTRACTED 1.00]
- **Single-source pricing: browser formulas, SQL views and tests** — readme_pricing_formulas, src_lib_pricing, readme_hamper_summary_view, src_lib_pricing_test, docs_lattice_lane_hamper_builder_feature_set_pricing_formulas [INFERRED 0.85]
- **Lattice Lane bird-in-nest brand mark variants** — docs_brand_lattice_lane_logo_lattice_lane_logo, public_brand_deck_logo_deck_logo, public_lattice_lane_lockup_lattice_lane_lockup, public_lattice_lane_logo_black_lattice_lane_logo_black, public_lattice_lane_logo_lattice_lane_logo, src_app_icon_app_icon [INFERRED 0.85]
- **create-next-app Default Template Assets** — public_file_file_icon, public_globe_globe_icon, public_next_next_js_wordmark, public_vercel_vercel_logo, public_window_window_icon [INFERRED 0.95]
- **Starter Page Footer Link Icons (16x16 #666 line icons)** — public_file_file_icon, public_globe_globe_icon, public_window_window_icon [INFERRED 0.85]

## Communities (100 total, 10 thin omitted)

### Community 0 - "Presentation Deck Slides"
Cohesion: 0.05
Nodes (82): zod, getBrandLogo(), LOGO_SIZE, DeckData, HamperRow, LineRow, loadDeckData(), ProductRow (+74 more)

### Community 1 - "Quote Builder & Pricing"
Cohesion: 0.06
Nodes (69): lookupGstin(), fetchDetails(), convertToProforma(), deleteQuote(), LineSchema, nullableDate, nullableText, QuoteSchema (+61 more)

### Community 2 - "Hamper Builder Editor"
Cohesion: 0.06
Nodes (57): nextConfig, next, deleteHamper(), duplicateHamper(), HamperSchema, LineSchema, nullableText, saveHamper() (+49 more)

### Community 3 - "Studio Stage Rendering"
Cohesion: 0.08
Nodes (46): konva, react-konva, baseLayer(), cachedImage(), fontFaces(), FONTS, imageCache, loadFonts() (+38 more)

### Community 4 - "Canvas Stage & Images"
Cohesion: 0.10
Nodes (35): react, CanvasStage(), ActionResult, UploadTicket, BoardOverlays(), PageResizeHandles(), TextEditor(), TextLayer (+27 more)

### Community 5 - "Magic Layers (SAM Masks)"
Cohesion: 0.09
Nodes (33): OcrLine, readLines(), blankCanvas(), ImageLayer, markBox(), runMagicLayers(), findObjects(), load() (+25 more)

### Community 6 - "Recycle Bin & Permissions"
Cohesion: 0.13
Nodes (32): emptyBinAction(), permanentlyDeleteBinItem(), purgeExpiredAction(), restoreBinItem(), emptyBinAction(), permanentlyDeleteBinItem(), BinHeader(), BinItemRow() (+24 more)

### Community 7 - "Postgres Rule Sections"
Cohesion: 0.07
Nodes (25): Supabase Postgres Best Practices Changelog, Schema-Constraints Reference (Safe Migration Patterns), Security Checklist (SECURITY DEFINER, auth.role() deprecation, BOLA), Impact Level Guidelines, Reference Review Checklist, Writing Guidelines for Postgres References, Advanced Features (advanced-), Concurrency & Locking (lock-) (+17 more)

### Community 8 - "Product Sheet XLSX Export"
Cohesion: 0.09
Nodes (37): buildCostTree(), groupBy(), buildProductSheet(), LineHeader, AREA, at(), FORMULAS, Item (+29 more)

### Community 9 - "Hamper Canvas Schema"
Cohesion: 0.10
Nodes (26): PrintPresentationPage(), PrintDeck(), LayersPanel(), Thumb(), SlideThumb(), PageStrip(), toolBtnActive, BRAND_COLORS (+18 more)

### Community 10 - "Tables & Money Formatting"
Cohesion: 0.22
Nodes (28): PriceDiff, RefreshPricesPage(), Search, deleteClient(), saveClient(), ClientForm(), ClientsPage(), one() (+20 more)

### Community 11 - "Studio Toolbar UI"
Cohesion: 0.22
Nodes (24): lucide-react, ContextToolbar(), LayerTools(), MultiTools(), AlignMenu(), PositionMenu(), ImageTools(), LineTools() (+16 more)

### Community 12 - "List Pages & Headers"
Cohesion: 0.13
Nodes (26): ImportPage(), ACTIONS, one(), ProductHistoryPage(), Search, FilterForm(), one(), ProductsPage() (+18 more)

### Community 13 - "Settings & Client Forms"
Cohesion: 0.13
Nodes (25): applyRefresh(), ApplyButton(), ClientSchema, GstDetails, CompanySchema, saveCompany(), saveDefault(), saveSetting() (+17 more)

### Community 14 - "Studio Editor Panels"
Cohesion: 0.15
Nodes (26): DRAG_MIME, Editor, makeText(), PickerProduct, TEXT_COMBOS, TEXT_PRESETS, TextStyle, MagicStep (+18 more)

### Community 15 - "Cost Master Categories"
Cohesion: 0.17
Nodes (25): GET(), deleteCostCategory(), renameCostCategory(), saveCostCategory(), CategoryBlock(), handleAdd(), handleDelete(), handleRename() (+17 more)

### Community 16 - "Product Codes & Colours"
Cohesion: 0.15
Nodes (27): getNextSerialAction(), ColorsCard(), handleAddColor(), handleHexChange(), handleRemoveColor(), save(), saveProductColors(), recolorProduct() (+19 more)

### Community 17 - "Electron Auto-Updater"
Cohesion: 0.10
Nodes (24): { app, BrowserWindow, dialog, shell, utilityProcess, ipcMain, session }, checkForUpdate, freePort(), fs, http, net, path, serverScript() (+16 more)

### Community 18 - "Product Routes & Roles"
Cohesion: 0.16
Nodes (24): CalculatorScreen(), ProductCostingPage(), CostingHistoryPage(), loadCostStages(), loadProductCosting(), ProductPage(), CHANGED_ONLY, HISTORY_SELECT (+16 more)

### Community 19 - "Shared Row Types & Live Merge"
Cohesion: 0.13
Nodes (22): HamperPage(), PreviewItem, HamperTotals(), tone(), HamperRow, HamperRows(), ProductRow, ProductRows() (+14 more)

### Community 20 - "Sheet Import Wizards"
Cohesion: 0.12
Nodes (22): xlsx@https://cdn.sheetjs.com/xlsx-0.20.3/x..., xlsx, pickWorkbook(), SheetWizard(), pick(), run(), Step(), productsForSheet() (+14 more)

### Community 21 - "Cost Tree & Rate Import"
Cohesion: 0.10
Nodes (17): active(), CostRows, Editable, names(), rows, stage, ProductVariant, variantIdsByColor() (+9 more)

### Community 22 - "Dev Dependencies"
Cohesion: 0.10
Nodes (27): devDependencies, cross-env@^10.1.0, electron-builder@^26.15.3, @huggingface/transformers@4.3.1, tailwindcss@^4, @tailwindcss/postcss@^4, @types/node@^20, @types/papaparse@^5.5.2 (+19 more)

### Community 23 - "Settings Page & Nav"
Cohesion: 0.12
Nodes (21): loadCostRows(), setUserRole(), CategoryList(), TOOLS, ToolsTab(), ROLE_HINT, UserRoleForm(), CategoriesTab() (+13 more)

### Community 24 - "Feature Set PDF"
Cohesion: 0.12
Nodes (23): Hamper Builder Feature Set (PDF), Automated tests for pricing and import mapping, Bulk spreadsheet import, Client master, Cost calculator, Dashboard, Detail mode (Show contents / Summary only / Hide), Collision-free document numbers (H###, LLQT-###, LLPI-###) (+15 more)

### Community 25 - "Product Details & Thumbs"
Cohesion: 0.16
Nodes (14): CodeField(), ColorPhotoRows(), matchOrigin(), ORIGINS, ProductDetailsCard(), ProductDetails, useProductDetails(), ProductPhotos (+6 more)

### Community 26 - "Cost Calculator View"
Cohesion: 0.18
Nodes (18): costingSaved(), ApplyPriceDialog(), Sibling, CostCalculatorView(), CostSummary(), LeaveDialog(), useLeaveGuard(), savedLines() (+10 more)

### Community 27 - "Costing Math & MRP"
Cohesion: 0.23
Nodes (18): usePricing(), changeSp(), roundManualSp(), AREA_UNITS, calculateCostSheetTotals(), LINEAR_UNITS, MINUTES_PER, StageOverheads (+10 more)

### Community 28 - "Product Image Storage"
Cohesion: 0.21
Nodes (18): prepareProductImageUpload(), saveUploadedProductImage(), ImageActionResult, uploadProductImage(), IMMUTABLE, optimizeImageForUpload(), toWebp(), uploadDirect() (+10 more)

### Community 29 - "Cost Lines Hierarchy"
Cohesion: 0.16
Nodes (19): byName(), Pick(), PickCells(), Props, TypedCells(), set(), BOUGHT_OUT, createEmptyLine() (+11 more)

### Community 30 - "Cost Master View & Transfers"
Cohesion: 0.16
Nodes (15): filterStages(), CostMasterView(), duplicateCostSubcategory(), moveCostSubcategory(), nameTaken(), nextSortOrder(), Supabase, TransferSubcategoryDialog() (+7 more)

### Community 31 - "Repricing Plan"
Cohesion: 0.17
Nodes (11): VarietyRow, filterComboOptions(), all, current, scaledPrice(), MasterRate, planReprice(), PricedProduct (+3 more)

### Community 32 - "Keybindings"
Cohesion: 0.16
Nodes (19): GROUPS, KeyBindings(), Slot, STUDIO_SHORTCUTS, ActionId, ACTIONS, assign(), Bindings (+11 more)

### Community 33 - "Product Sheet Costing"
Cohesion: 0.15
Nodes (17): numbered(), ExistingCosting, LINE_HEADERS, LINES_SHEET, norm(), numOrNull(), pct(), planSheetCosting() (+9 more)

### Community 34 - "Desktop Release Workflow"
Cohesion: 0.09
Nodes (21): concurrency, cancel-in-progress, group, CLOUDFLARE_ACCOUNT_ID, CLOUDFLARE_API_TOKEN, jobs, release, name (+13 more)

### Community 35 - "Login & Auth Card"
Cohesion: 0.16
Nodes (15): AuthCard(), submit(), Label(), loadClient(), Beam(), LighthouseSvg(), BeamState, useLighthouse() (+7 more)

### Community 36 - "List Row Selection"
Cohesion: 0.19
Nodes (20): DotsItem, DotsMenu(), ListMenu(), one(), ReviewMark(), Row, SelectionCount(), cache (+12 more)

### Community 37 - "Canvas Arrange & Layers"
Cohesion: 0.21
Nodes (16): useArrange(), useLayerEvents(), startMarquee(), alignBoxes(), alignToPage(), boundsOf(), Box, distributeBoxes() (+8 more)

### Community 38 - "Migration & Build Scripts"
Cohesion: 0.11
Nodes (11): CHECK_ONLY, dir, files, ROOT, tables, iconIco, ROOT, STANDALONE (+3 more)

### Community 39 - "Cost Stage Tables"
Cohesion: 0.23
Nodes (16): MrpCell(), NumCell(), OverheadInput(), RemoveButton(), SectionHeader(), UnitCell(), LINE_UNITS, StageLineRow() (+8 more)

### Community 40 - "Pick List Manager"
Cohesion: 0.20
Nodes (14): addPickListItem(), removePickListItem(), PickListManager(), handleAdd(), handleRemove(), saveOptimistic(), AddOptionDialog(), PICK_LIST_CONFIGS (+6 more)

### Community 41 - "TypeScript Config"
Cohesion: 0.10
Nodes (19): compilerOptions, allowImportingTsExtensions, allowJs, esModuleInterop, incremental, isolatedModules, jsx, lib (+11 more)

### Community 42 - "Supabase Agent Skill"
Cohesion: 0.18
Nodes (9): Skill Feedback Issue Template, Supabase Agent Skill Changelog, supabase/agent-skills GitHub repository, Skill Feedback Procedure, Supabase Agent Skill, supabase db advisors / MCP get_advisors, Supabase CLI (discover via --help), Supabase MCP server (+1 more)

### Community 43 - "Legacy Apps Script"
Cohesion: 0.15
Nodes (11): addSelectedHamperToQuote(), categoryCountsAsItem(), duplicateHamper(), getNextHamperCode(), loadHamper(), newHamper(), onEdit(), populateProductRow() (+3 more)

### Community 44 - "Bought-Out Master & Bin Data"
Cohesion: 0.16
Nodes (12): @supabase/supabase-js, loadBinItems(), calculateExpiry(), ExpiryInfo, MS_PER_DAY, RETENTION_DAYS, fail(), findOrCreate() (+4 more)

### Community 45 - "Cost Sheet Save Actions"
Cohesion: 0.27
Nodes (15): saveCostSheetAndProduct(), SaveCostSheetPayload, MasterVariety, StoredLine, syncAllRates(), SyncRatesButton(), sync(), repriceVarieties() (+7 more)

### Community 46 - "Live Refresh Realtime"
Cohesion: 0.19
Nodes (13): LiveRefresh(), useLiveChange(), applyRowChange(), CHILD_TABLES, emitLiveChange(), isOthersChange(), listeners, LiveChange (+5 more)

### Community 47 - "shadcn Components Config"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 48 - "Runtime Dependencies"
Cohesion: 0.11
Nodes (18): dependencies, clsx@^2.1.1, konva@^10.5.0, lucide-react@^1.46.0, next@16.3.4, papaparse@^5.7.0, pptxgenjs@^4.0.1, react@19.2.8 (+10 more)

### Community 49 - "Admin Import Wizard UI"
Cohesion: 0.24
Nodes (12): ImportReport, ImportReportView(), labelFor(), ImportWizard(), pick(), pickMany(), run(), LoadStep() (+4 more)

### Community 50 - "Settings Loader & Print"
Cohesion: 0.24
Nodes (11): CostMasterPage(), Range(), HampersPage(), one(), Search, NewPresentationPage(), PrintQuotePage(), PrintButton() (+3 more)

### Community 51 - "Spreadsheet Import Parsing"
Cohesion: 0.28
Nodes (12): buildImport(), body(), cell(), filled(), ImportInput, isYes(), parseDate(), findDuplicates() (+4 more)

### Community 52 - "Electron Builder Config"
Cohesion: 0.12
Nodes (17): build, appId, directories, extraResources, files, nsis, productName, win (+9 more)

### Community 53 - "Bulk Costing Import"
Cohesion: 0.28
Nodes (14): runImport(), CHUNK, planCostings(), Row, saveSheetCosting(), saveSheetWastage(), Supabase, inChunks() (+6 more)

### Community 54 - "Undo & Shortcuts"
Cohesion: 0.23
Nodes (14): capturing(), NOT_TEXT, Shortcuts(), typingIn(), mayLeave(), actionFor(), comboFromMouse(), subscribeBindings() (+6 more)

### Community 55 - "README Architecture"
Cohesion: 0.23
Nodes (11): Hamper builder, Refresh prices with preview, Lattice Lane README, Combined Order structure (option-only totals), First sign-up becomes admin, later users start as sales, hamper_summary view, Legacy Google Sheet and Apps Script, next_doc_no() row-locked document numbering (+3 more)

### Community 56 - "Image Edit Actions"
Cohesion: 0.31
Nodes (14): addExternalProductImage(), deleteProductImage(), setPrimaryProductImage(), updateProductImageColor(), colorOrNull(), Feedback, useImageActions(), addUrl() (+6 more)

### Community 58 - "Desktop Publish Button"
Cohesion: 0.25
Nodes (12): CheckForUpdatesButton(), ElectronBridge, PublishReleaseButton(), poll(), STEP_LABELS, UpdateAppButton(), desktopReleaseStatus(), githubHeaders() (+4 more)

### Community 59 - "Product Sheet Formulas"
Cohesion: 0.20
Nodes (14): [codes, types, totals], COLUMNS, costFormula(), LINE_COL, SHEET_STAGES, STAGE_COL, formulaSp(), letter() (+6 more)

### Community 60 - "RLS & Least Privilege"
Cohesion: 0.18
Nodes (11): app_readonly role, app_writer role, Apply Principle of Least Privilege, Revoke public schema defaults, auth.uid(), Enable Row Level Security for Multi-Tenant Data, FORCE ROW LEVEL SECURITY, Row Level Security (RLS) (+3 more)

### Community 61 - "Signed Uploads & Canvas Save"
Cohesion: 0.23
Nodes (11): saveHamperCanvas(), storagePathOf(), upload(), uploadHamperBackground(), saveSlide(), Supabase, touch(), uploadPresentationAsset() (+3 more)

### Community 62 - "Landing Page"
Cohesion: 0.26
Nodes (9): CURATED, CURATED_FALLBACKS, DisplayProduct, HeroCard(), getLandingData(), ProductCard(), ScrollingProductColumn(), dynamic (+1 more)

### Community 63 - "Product Images Manager"
Cohesion: 0.27
Nodes (10): ImageCard(), isGeneral(), isOfColor(), Manager(), ProductImagesManager(), Props, tabClass(), SaveFirst() (+2 more)

### Community 64 - "npm Scripts"
Cohesion: 0.15
Nodes (13): scripts, build, build:desktop, build:next, cf:build, cf:deploy, cf:preview, dev (+5 more)

### Community 65 - "pnpm Build Allowlist"
Cohesion: 0.15
Nodes (12): allowBuilds, electron, electron-winstaller, esbuild, onnxruntime-node, protobufjs, tesseract.js, unrs-resolver (+4 more)

### Community 66 - "Stage Context Menus"
Cohesion: 0.40
Nodes (10): BRAND_COLORS, ContextMenu(), hasFill(), isLine(), isShape(), MenuItem, menuItems(), ColorEdit (+2 more)

### Community 67 - "Query Diagnosis (EXPLAIN/VACUUM)"
Cohesion: 0.21
Nodes (10): Buffers shared hit vs read, EXPLAIN ANALYZE, Seq Scan (sequential scan), Use EXPLAIN ANALYZE to Diagnose Slow Queries, work_mem, ANALYZE (planner statistics), Maintain Table Statistics with VACUUM and ANALYZE, pg_stat_user_tables (+2 more)

### Community 68 - "Supabase Env & Proxy"
Cohesion: 0.29
Nodes (6): @supabase/ssr@^0.12.6, @supabase/ssr, SupabaseConfigError, supabaseEnv(), config, proxy()

### Community 69 - "Cost Calculator Tabs"
Cohesion: 0.35
Nodes (8): costingHref(), historyHref(), CostCalculatorPage(), one(), Search, CostTabs(), productCode(), tabClass()

### Community 70 - "Root Layout & Service Worker"
Cohesion: 0.25
Nodes (7): cabin, figtree, geistMono, metadata, RootLayout(), RegisterServiceWorker(), StaleBuildPrompt()

### Community 71 - "Postgres Index Types"
Cohesion: 0.22
Nodes (10): BRIN index, B-tree index, Choose the Right Index Type for Your Data, GIN index, GiST index, Hash index, Drop partition for data retention, Partition Large Tables for Better Performance (+2 more)

### Community 73 - "Combo Menu"
Cohesion: 0.32
Nodes (5): react-dom@19.2.8, react-dom, ComboMenu(), OptionLabel(), MenuCoords

### Community 74 - "Product Photo Uploads"
Cohesion: 0.32
Nodes (6): Pending, uploadFiles(), UploadStatus, useProductPhotos(), afterSave(), uploadForColor()

### Community 75 - "Password Reset"
Cohesion: 0.36
Nodes (5): dynamic, ResetPasswordPage(), Card(), ResetForm(), Status

### Community 76 - "Amount In Words (INR)"
Cohesion: 0.36
Nodes (7): amountInWords(), below100(), inr, joinGroups(), ONES, TENS, wordGroups()

### Community 77 - "Composite & Covering Indexes"
Cohesion: 0.29
Nodes (6): Bitmap scan combining separate indexes, Composite (multi-column) index, Create Composite Indexes for Multi-Column Queries, Covering index (INCLUDE), Index-only scan, Use Covering Indexes to Avoid Table Lookups

### Community 78 - "Data Types & Constraints"
Cohesion: 0.29
Nodes (3): ADD CONSTRAINT IF NOT EXISTS (unsupported), Add Constraints Safely in Migrations, Choose Appropriate Data Types

### Community 79 - "Primary Key Strategy"
Cohesion: 0.29
Nodes (6): bigint generated always as identity, Select Optimal Primary Key Strategy, serial, Time-prefixed text IDs, Random UUIDv4 primary keys, UUIDv7 (pg_uuidv7)

### Community 80 - "OpenNext Cloudflare Files"
Cohesion: 0.29
Nodes (4): @opennextjs/cloudflare@^1.20.6, @opennextjs/cloudflare, Bucket, FILES

### Community 81 - "App Shell & Stale Pages"
Cohesion: 0.43
Nodes (4): AppLayout(), Nav(), signOut(), RefreshStalePages()

### Community 82 - "Lattice Lane Brand Marks"
Cohesion: 0.47
Nodes (6): Lattice Lane Master Logo (black, stacked, with LLP line), Deck Logo (black, stacked, no LLP line), Lattice Lane Lockup (white, stacked, transparent), Lattice Lane Logo Black (stacked, with LLP line), Lattice Lane Horizontal Logo (white, transparent), App Icon (white symbol on sage green)

### Community 83 - "ESLint Config"
Cohesion: 0.33
Nodes (5): eslintConfig, eslint@^9, eslint-config-next@16.3.4, eslint, eslint-config-next

### Community 84 - "Price Summary & Breakdown"
Cohesion: 0.73
Nodes (4): CostBreakdown(), PriceSummary(), CostSheetTotals, formatPct()

### Community 86 - "Foreign Key Indexes"
Cohesion: 0.40
Nodes (5): pg_constraint catalog, Foreign key column index, Index Foreign Key Columns, Missing FK index detection query, ON DELETE CASCADE

### Community 88 - "Lowercase Identifiers"
Cohesion: 0.50
Nodes (3): Compatibility view layer, Quoted mixed-case identifiers, Use Lowercase Identifiers for Compatibility

### Community 89 - "Electron Preload"
Cohesion: 0.50
Nodes (3): { contextBridge, ipcRenderer }, electron@^44.4.1, electron

### Community 90 - "MCP Server Config"
Cohesion: 0.50
Nodes (3): cloudflare-builds, cloudflare-observability, supabase

### Community 93 - "Partial Indexes"
Cohesion: 0.67
Nodes (3): Partial index (WHERE clause), Soft-delete filter (deleted_at is null), Use Partial Indexes for Filtered Queries

### Community 94 - "React Doctor Config"
Cohesion: 0.67
Nodes (3): files, reactDoctor, ignore

### Community 95 - "Template Icons"
Cohesion: 0.67
Nodes (3): File Document Icon, Globe Icon, Browser Window Icon

## Knowledge Gaps
- **46 isolated node(s):** `Query Performance (query-)`, `Monitoring & Diagnostics (monitor-)`, `jsonb_path_ops Operator Class`, `COPY Bulk Load`, `Use EXPLAIN ANALYZE to Diagnose Slow Queries` (+41 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 467 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `react` connect `Canvas Stage & Images` to `Presentation Deck Slides`, `Quote Builder & Pricing`, `Hamper Builder Editor`, `Studio Stage Rendering`, `Recycle Bin & Permissions`, `Hamper Canvas Schema`, `Tables & Money Formatting`, `Studio Toolbar UI`, `List Pages & Headers`, `Settings & Client Forms`, `Studio Editor Panels`, `Cost Master Categories`, `Product Codes & Colours`, `Product Routes & Roles`, `Shared Row Types & Live Merge`, `Dev Dependencies`, `Settings Page & Nav`, `Product Details & Thumbs`, `Cost Calculator View`, `Costing Math & MRP`, `Cost Lines Hierarchy`, `Cost Master View & Transfers`, `Keybindings`, `Login & Auth Card`, `List Row Selection`, `Canvas Arrange & Layers`, `Cost Stage Tables`, `Pick List Manager`, `Cost Sheet Save Actions`, `Live Refresh Realtime`, `Runtime Dependencies`, `Admin Import Wizard UI`, `Undo & Shortcuts`, `Image Edit Actions`, `Desktop Publish Button`, `Product Images Manager`, `Stage Context Menus`, `Root Layout & Service Worker`, `Combo Menu`, `Product Photo Uploads`, `Password Reset`, `App Shell & Stale Pages`, `Unsaved Changes Guard`?**
  _High betweenness centrality (0.215) - this node is a cross-community bridge._
- **What connects `Query Performance (query-)`, `Monitoring & Diagnostics (monitor-)`, `jsonb_path_ops Operator Class` to the rest of the system?**
  _46 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Presentation Deck Slides` be split into smaller, more focused modules?**
  _Cohesion score 0.05408805031446541 - nodes in this community are weakly interconnected._
- **Why does `next` connect `Hamper Builder Editor` to `Presentation Deck Slides`, `Quote Builder & Pricing`, `Canvas Stage & Images`, `Recycle Bin & Permissions`, `Hamper Canvas Schema`, `Tables & Money Formatting`, `List Pages & Headers`, `Settings & Client Forms`, `Cost Master Categories`, `Product Routes & Roles`, `Shared Row Types & Live Merge`, `Sheet Import Wizards`, `Dev Dependencies`, `Settings Page & Nav`, `Product Details & Thumbs`, `Login & Auth Card`, `Cost Sheet Save Actions`, `Live Refresh Realtime`, `Runtime Dependencies`, `Settings Loader & Print`, `Bulk Costing Import`, `Undo & Shortcuts`, `Desktop Publish Button`, `Signed Uploads & Canvas Save`, `Landing Page`, `Product Images Manager`, `Supabase Env & Proxy`, `Cost Calculator Tabs`, `Root Layout & Service Worker`, `Password Reset`, `App Shell & Stale Pages`, `Unsaved Changes Guard`?**
  _High betweenness centrality (0.067) - this node is a cross-community bridge._
- **Should `Quote Builder & Pricing` be split into smaller, more focused modules?**
  _Cohesion score 0.06139512661251792 - nodes in this community are weakly interconnected._
- **Why does `Link()` connect `Product Routes & Roles` to `Presentation Deck Slides`, `Quote Builder & Pricing`, `Hamper Builder Editor`, `Canvas Stage & Images`, `Recycle Bin & Permissions`, `Hamper Canvas Schema`, `Tables & Money Formatting`, `List Pages & Headers`, `Shared Row Types & Live Merge`, `Sheet Import Wizards`, `Settings Page & Nav`, `Product Details & Thumbs`, `Cost Calculator View`, `Admin Import Wizard UI`, `Settings Loader & Print`, `Landing Page`, `Cost Calculator Tabs`, `Password Reset`, `App Shell & Stale Pages`?**
  _High betweenness centrality (0.044) - this node is a cross-community bridge._
- **Should `Hamper Builder Editor` be split into smaller, more focused modules?**
  _Cohesion score 0.06234177215189873 - nodes in this community are weakly interconnected._