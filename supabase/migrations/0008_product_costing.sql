-- 0008_product_costing.sql
-- Dynamic Costing Hierarchy, Product Cost Calculator, and Cost Breakdown Lines

-- ---------------------------------------------------------------
-- 1. STAGES (Material, Hardware, Finishing, Machine Cost, etc.)
-- ---------------------------------------------------------------

create table if not exists cost_stages (
  id               uuid primary key default gen_random_uuid(),
  code             text not null unique,
  name             text not null,
  calculation_type text not null default 'dimension', -- 'dimension', 'piece', 'time_based'
  sort_order       int not null default 0,
  is_active        boolean not null default true,
  created_at       timestamptz not null default now(),
  updated_at       timestamptz not null default now()
);

create index if not exists cost_stages_active_idx on cost_stages (is_active);

drop trigger if exists cost_stages_touch on cost_stages;
create trigger cost_stages_touch before update on cost_stages
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 2. CATEGORIES UNDER EACH STAGE
-- ---------------------------------------------------------------

create table if not exists cost_categories (
  id         uuid primary key default gen_random_uuid(),
  stage_id   uuid not null references cost_stages(id) on delete cascade,
  name       text not null,
  sort_order int not null default 0,
  is_active  boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (stage_id, name)
);

create index if not exists cost_categories_stage_idx on cost_categories (stage_id);
create index if not exists cost_categories_active_idx on cost_categories (is_active);

drop trigger if exists cost_categories_touch on cost_categories;
create trigger cost_categories_touch before update on cost_categories
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 3. ITEMS / SUBCATEGORIES / VARIETIES UNDER EACH CATEGORY
-- ---------------------------------------------------------------

create table if not exists cost_items (
  id                  uuid primary key default gen_random_uuid(),
  category_id         uuid not null references cost_categories(id) on delete cascade,
  name                text not null,
  default_rate        numeric not null default 0,
  unit                text not null default 'sq ft', -- 'sq ft', 'sq inch', 'sq mm', 'piece', 'running ft', 'min', 'hour'
  default_wastage_pct numeric not null default 0,    -- 10 = 10%
  notes               text,
  sort_order          int not null default 0,
  is_active           boolean not null default true,
  created_at          timestamptz not null default now(),
  updated_at          timestamptz not null default now(),
  unique (category_id, name)
);

create index if not exists cost_items_category_idx on cost_items (category_id);
create index if not exists cost_items_active_idx on cost_items (is_active);

drop trigger if exists cost_items_touch on cost_items;
create trigger cost_items_touch before update on cost_items
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 4. PRODUCT ALTERATION (Markup % and Colors)
-- ---------------------------------------------------------------

alter table products
  add column if not exists markup_pct numeric not null default 0,
  add column if not exists colors text[] not null default '{}';

-- ---------------------------------------------------------------
-- 5. PRODUCT COST SHEETS (Saved BOM / Cost calculation sheets)
-- ---------------------------------------------------------------

create table if not exists product_cost_sheets (
  id              uuid primary key default gen_random_uuid(),
  product_id      uuid references products(id) on delete set null,
  product_code    text,
  product_name    text not null,
  material_total  numeric not null default 0,
  hardware_total  numeric not null default 0,
  finishing_total numeric not null default 0,
  machine_total   numeric not null default 0,
  total_cost      numeric not null default 0,
  markup_pct      numeric not null default 0,
  calculated_sp   numeric not null default 0,
  notes           text,
  created_by      uuid references profiles(id) on delete set null,
  created_at      timestamptz not null default now(),
  updated_at      timestamptz not null default now()
);

create index if not exists product_cost_sheets_product_idx on product_cost_sheets (product_id);

drop trigger if exists product_cost_sheets_touch on product_cost_sheets;
create trigger product_cost_sheets_touch before update on product_cost_sheets
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 6. PRODUCT COST LINES (Detailed lines per stage)
-- ---------------------------------------------------------------

create table if not exists product_cost_lines (
  id               uuid primary key default gen_random_uuid(),
  sheet_id         uuid not null references product_cost_sheets(id) on delete cascade,
  stage_code       text not null, -- 'material', 'hardware', 'finishing', 'machine'
  category_name    text not null,
  item_name        text not null,
  cost_item_id     uuid references cost_items(id) on delete set null,
  length           numeric,
  breadth          numeric,
  dimension_unit   text not null default 'inch', -- 'inch', 'mm', 'cm'
  unit             text not null default 'sq ft',
  rate             numeric not null default 0,
  duration_minutes numeric,                      -- for machine
  qty              numeric not null default 1,
  wastage_pct      numeric not null default 0,   -- 10 = 10%
  calculated_area  numeric,
  line_total       numeric not null default 0,
  sort_order       int not null default 0,
  created_at       timestamptz not null default now()
);

create index if not exists product_cost_lines_sheet_idx on product_cost_lines (sheet_id);
create index if not exists product_cost_lines_item_idx on product_cost_lines (cost_item_id);

-- ---------------------------------------------------------------
-- 7. ROW LEVEL SECURITY (RLS)
-- ---------------------------------------------------------------

alter table cost_stages enable row level security;
alter table cost_categories enable row level security;
alter table cost_items enable row level security;
alter table product_cost_sheets enable row level security;
alter table product_cost_lines enable row level security;

-- Drop existing policies if re-running
drop policy if exists cost_stages_read on cost_stages;
create policy cost_stages_read on cost_stages
  for select to authenticated using (true);

drop policy if exists cost_categories_read on cost_categories;
create policy cost_categories_read on cost_categories
  for select to authenticated using (true);

drop policy if exists cost_items_read on cost_items;
create policy cost_items_read on cost_items
  for select to authenticated using (true);

drop policy if exists product_cost_sheets_read on product_cost_sheets;
create policy product_cost_sheets_read on product_cost_sheets
  for select to authenticated using (true);

drop policy if exists product_cost_lines_read on product_cost_lines;
create policy product_cost_lines_read on product_cost_lines
  for select to authenticated using (true);

-- Write policies (manager & admin can configure and save costings)
drop policy if exists cost_stages_write on cost_stages;
create policy cost_stages_write on cost_stages
  for all to authenticated using (is_manager()) with check (is_manager());

drop policy if exists cost_categories_write on cost_categories;
create policy cost_categories_write on cost_categories
  for all to authenticated using (is_manager()) with check (is_manager());

drop policy if exists cost_items_write on cost_items;
create policy cost_items_write on cost_items
  for all to authenticated using (is_manager()) with check (is_manager());

drop policy if exists product_cost_sheets_write on product_cost_sheets;
create policy product_cost_sheets_write on product_cost_sheets
  for all to authenticated using (is_manager()) with check (is_manager());

drop policy if exists product_cost_lines_write on product_cost_lines;
create policy product_cost_lines_write on product_cost_lines
  for all to authenticated using (is_manager()) with check (is_manager());

-- ---------------------------------------------------------------
-- 8. SEED DEFAULT STAGES, CATEGORIES, AND VARIETIES
-- ---------------------------------------------------------------

insert into cost_stages (code, name, calculation_type, sort_order) values
  ('material',  'Material',     'dimension',  1),
  ('hardware',  'Hardware',     'dimension',  2),
  ('finishing', 'Finishing',    'dimension',  3),
  ('machine',   'Machine Cost', 'time_based', 4)
on conflict (code) do update set name = excluded.name, calculation_type = excluded.calculation_type;

-- Categories & Items helper insertion
do $$
declare
  v_stage_mat uuid;
  v_stage_hw  uuid;
  v_stage_fin uuid;
  v_stage_mac uuid;

  v_cat_wood  uuid;
  v_cat_acryl uuid;
  v_cat_glass uuid;
  v_cat_mirr  uuid;
  v_cat_leath uuid;

  v_cat_mag   uuid;
  v_cat_hinge uuid;
  v_cat_latch uuid;
  v_cat_bush  uuid;

  v_cat_pol   uuid;
  v_cat_paint uuid;

  v_cat_laser uuid;
  v_cat_rout  uuid;
  v_cat_uv    uuid;
begin
  select id into v_stage_mat from cost_stages where code = 'material';
  select id into v_stage_hw  from cost_stages where code = 'hardware';
  select id into v_stage_fin from cost_stages where code = 'finishing';
  select id into v_stage_mac from cost_stages where code = 'machine';

  -- Material Categories
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mat, 'Woodbased', 1) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_wood;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mat, 'Acrylic', 2) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_acryl;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mat, 'Glass', 3) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_glass;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mat, 'Mirror', 4) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_mirr;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mat, 'Leather', 5) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_leath;

  -- Hardware Categories
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_hw, 'Magnets', 1) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_mag;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_hw, 'Hinges', 2) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_hinge;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_hw, 'Latch', 3) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_latch;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_hw, 'Bush', 4) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_bush;

  -- Finishing Categories
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_fin, 'Polish', 1) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_pol;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_fin, 'Paint', 2) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_paint;

  -- Machine Categories
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mac, 'Laser', 1) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_laser;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mac, 'Router', 2) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_rout;
  insert into cost_categories (stage_id, name, sort_order) values
    (v_stage_mac, 'UV Printer', 3) on conflict (stage_id, name) do update set name = excluded.name returning id into v_cat_uv;

  -- Woodbased Subcategories / Varieties
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_wood, '3mm Birch', 45, 'sq ft', 10, 1),
    (v_cat_wood, '4mm Birch', 55, 'sq ft', 10, 2),
    (v_cat_wood, '8mm Birch', 85, 'sq ft', 10, 3),
    (v_cat_wood, '12mm Rubberwood', 140, 'sq ft', 12, 4),
    (v_cat_wood, '10mm Acacia', 160, 'sq ft', 15, 5)
  on conflict (category_id, name) do nothing;

  -- Acrylic Subcategories
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_acryl, '2mm Clear Acrylic', 85, 'sq ft', 10, 1),
    (v_cat_acryl, '3mm Cast Acrylic', 120, 'sq ft', 10, 2),
    (v_cat_acryl, '5mm Cast Acrylic', 190, 'sq ft', 10, 3)
  on conflict (category_id, name) do nothing;

  -- Glass Subcategories
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_glass, '4mm Clear Glass', 45, 'sq ft', 10, 1),
    (v_cat_glass, '4mm Tinted Glass', 65, 'sq ft', 10, 2),
    (v_cat_glass, 'Frosted Glass', 75, 'sq ft', 10, 3)
  on conflict (category_id, name) do nothing;

  -- Mirror Subcategories
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_mirr, '4mm Plain Mirror', 55, 'sq ft', 8, 1),
    (v_cat_mirr, '4mm Bronze Mirror', 85, 'sq ft', 10, 2)
  on conflict (category_id, name) do nothing;

  -- Leather Subcategories
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_leath, 'Faux Leather / Rexine', 65, 'sq ft', 10, 1),
    (v_cat_leath, 'Genuine Leather Sheet', 180, 'sq ft', 15, 2)
  on conflict (category_id, name) do nothing;

  -- Hardware: Magnets
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_mag, 'Round Magnet (Neodymium)', 6, 'piece', 5, 1),
    (v_cat_mag, 'Cylinder Magnet', 10, 'piece', 5, 2),
    (v_cat_mag, 'Sheet Magnet Strip', 15, 'piece', 5, 3)
  on conflict (category_id, name) do nothing;

  -- Hardware: Hinges
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_hinge, 'Small Brass Hinge', 25, 'piece', 0, 1),
    (v_cat_hinge, 'Concealed Barrel Hinge', 45, 'piece', 0, 2),
    (v_cat_hinge, 'Pivot Hinge', 35, 'piece', 0, 3)
  on conflict (category_id, name) do nothing;

  -- Hardware: Latch
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_latch, 'Antique Brass Latch', 35, 'piece', 0, 1),
    (v_cat_latch, 'Slide Lock Latch', 40, 'piece', 0, 2)
  on conflict (category_id, name) do nothing;

  -- Hardware: Bush
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_bush, 'Rubber Bush (Pack of 4)', 12, 'piece', 0, 1),
    (v_cat_bush, 'Metal/Brass Bush', 20, 'piece', 0, 2)
  on conflict (category_id, name) do nothing;

  -- Finishing: Polish
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_pol, 'PU Polish Matt/Gloss', 35, 'sq ft', 8, 1),
    (v_cat_pol, 'Melamine Polish', 25, 'sq ft', 8, 2),
    (v_cat_pol, 'Clear Lacquer Spray', 20, 'sq ft', 5, 3),
    (v_cat_pol, 'Natural Oil Finish', 30, 'sq ft', 5, 4)
  on conflict (category_id, name) do nothing;

  -- Finishing: Paint
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_paint, 'Duco Paint Solid Color', 45, 'sq ft', 10, 1),
    (v_cat_paint, 'Enamel Spray Paint', 30, 'sq ft', 8, 2),
    (v_cat_paint, 'Walnut / Teak Wood Stain', 25, 'sq ft', 5, 3)
  on conflict (category_id, name) do nothing;

  -- Machine: Laser
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_laser, 'Laser Cutting (Birch/Wood)', 12, 'min', 5, 1),
    (v_cat_laser, 'Laser Cutting (Acrylic)', 15, 'min', 5, 2),
    (v_cat_laser, 'Laser Surface Engraving', 14, 'min', 5, 3)
  on conflict (category_id, name) do nothing;

  -- Machine: Router
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_rout, 'CNC 2D Profile Cutting', 18, 'min', 5, 1),
    (v_cat_rout, 'CNC 3D Carving / Relief', 22, 'min', 5, 2)
  on conflict (category_id, name) do nothing;

  -- Machine: UV Printer
  insert into cost_items (category_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_cat_uv, 'UV Flatbed Color Printing', 25, 'min', 5, 1),
    (v_cat_uv, 'UV White + Color Printing', 30, 'min', 5, 2)
  on conflict (category_id, name) do nothing;

end $$;

-- ---------------------------------------------------------------
-- 9. SEED PRODUCT COLORS IN APP SETTINGS
-- ---------------------------------------------------------------

insert into app_settings (key, value) values
  ('product_colors', '["Walnut", "Natural", "Teak", "Raw", "Dark Oak", "White", "Black"]'::jsonb)
on conflict (key) do nothing;
