-- 0009_cost_subcategories_and_varieties.sql
-- 4-level hierarchy: Stage -> Category -> Subcategory -> Variety

-- ---------------------------------------------------------------
-- 1. COST SUBCATEGORIES TABLE
-- ---------------------------------------------------------------

create table if not exists cost_subcategories (
  id          uuid primary key default gen_random_uuid(),
  category_id uuid not null references cost_categories(id) on delete cascade,
  name        text not null,
  sort_order  int not null default 0,
  is_active   boolean not null default true,
  created_at  timestamptz not null default now(),
  updated_at  timestamptz not null default now(),
  unique (category_id, name)
);

create index if not exists cost_subcategories_category_idx on cost_subcategories (category_id);
create index if not exists cost_subcategories_active_idx on cost_subcategories (is_active);

drop trigger if exists cost_subcategories_touch on cost_subcategories;
create trigger cost_subcategories_touch before update on cost_subcategories
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 2. COST VARIETIES TABLE
-- ---------------------------------------------------------------

create table if not exists cost_varieties (
  id                  uuid primary key default gen_random_uuid(),
  subcategory_id      uuid not null references cost_subcategories(id) on delete cascade,
  name                text not null, -- e.g. '8mm', '12mm', '10x2mm', 'Matt'
  default_rate        numeric not null default 0,
  unit                text not null default 'sq ft',
  default_wastage_pct numeric not null default 0,
  notes               text,
  sort_order          int not null default 0,
  is_active           boolean not null default true,
  created_at          timestamptz not null default now(),
  updated_at          timestamptz not null default now(),
  unique (subcategory_id, name)
);

create index if not exists cost_varieties_subcategory_idx on cost_varieties (subcategory_id);
create index if not exists cost_varieties_active_idx on cost_varieties (is_active);

drop trigger if exists cost_varieties_touch on cost_varieties;
create trigger cost_varieties_touch before update on cost_varieties
  for each row execute function touch_updated_at();

-- ---------------------------------------------------------------
-- 3. UPDATE PRODUCT COST LINES
-- ---------------------------------------------------------------

alter table product_cost_lines
  add column if not exists subcategory_name text,
  add column if not exists variety_name text,
  add column if not exists cost_variety_id uuid references cost_varieties(id) on delete set null;

create index if not exists product_cost_lines_variety_idx on product_cost_lines (cost_variety_id);

-- ---------------------------------------------------------------
-- 4. RLS POLICIES
-- ---------------------------------------------------------------

alter table cost_subcategories enable row level security;
alter table cost_varieties enable row level security;

drop policy if exists cost_subcategories_read on cost_subcategories;
create policy cost_subcategories_read on cost_subcategories
  for select to authenticated using (true);

drop policy if exists cost_subcategories_write on cost_subcategories;
create policy cost_subcategories_write on cost_subcategories
  for all to authenticated using (is_manager()) with check (is_manager());

drop policy if exists cost_varieties_read on cost_varieties;
create policy cost_varieties_read on cost_varieties
  for select to authenticated using (true);

drop policy if exists cost_varieties_write on cost_varieties;
create policy cost_varieties_write on cost_varieties
  for all to authenticated using (is_manager()) with check (is_manager());

-- ---------------------------------------------------------------
-- 5. SEED SUBCATEGORIES AND VARIETIES
-- ---------------------------------------------------------------

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

  -- Subcategory IDs
  v_sub_birch uuid;
  v_sub_rub   uuid;
  v_sub_acac  uuid;
  v_sub_cast  uuid;
  v_sub_mir_a uuid;
  v_sub_c_gls uuid;
  v_sub_t_gls uuid;
  v_sub_f_gls uuid;
  v_sub_p_mir uuid;
  v_sub_b_mir uuid;
  v_sub_f_lea uuid;
  v_sub_g_lea uuid;

  v_sub_r_mag uuid;
  v_sub_c_mag uuid;
  v_sub_s_mag uuid;
  v_sub_b_hin uuid;
  v_sub_c_hin uuid;
  v_sub_p_hin uuid;
  v_sub_a_lat uuid;
  v_sub_s_lat uuid;
  v_sub_r_bsh uuid;
  v_sub_m_bsh uuid;

  v_sub_pu_p  uuid;
  v_sub_mel_p uuid;
  v_sub_lac_p uuid;
  v_sub_oil_p uuid;
  v_sub_duc_p uuid;
  v_sub_stn_p uuid;

  v_sub_l_cut uuid;
  v_sub_l_eng uuid;
  v_sub_r_2d  uuid;
  v_sub_r_3d  uuid;
  v_sub_uv_p  uuid;
begin
  select id into v_stage_mat from cost_stages where code = 'material';
  select id into v_stage_hw  from cost_stages where code = 'hardware';
  select id into v_stage_fin from cost_stages where code = 'finishing';
  select id into v_stage_mac from cost_stages where code = 'machine';

  select id into v_cat_wood  from cost_categories where stage_id = v_stage_mat and name = 'Woodbased';
  select id into v_cat_acryl from cost_categories where stage_id = v_stage_mat and name = 'Acrylic';
  select id into v_cat_glass from cost_categories where stage_id = v_stage_mat and name = 'Glass';
  select id into v_cat_mirr  from cost_categories where stage_id = v_stage_mat and name = 'Mirror';
  select id into v_cat_leath from cost_categories where stage_id = v_stage_mat and name = 'Leather';

  select id into v_cat_mag   from cost_categories where stage_id = v_stage_hw and name = 'Magnets';
  select id into v_cat_hinge from cost_categories where stage_id = v_stage_hw and name = 'Hinges';
  select id into v_cat_latch from cost_categories where stage_id = v_stage_hw and name = 'Latch';
  select id into v_cat_bush  from cost_categories where stage_id = v_stage_hw and name = 'Bush';

  select id into v_cat_pol   from cost_categories where stage_id = v_stage_fin and name = 'Polish';
  select id into v_cat_paint from cost_categories where stage_id = v_stage_fin and name = 'Paint';

  select id into v_cat_laser from cost_categories where stage_id = v_stage_mac and name = 'Laser';
  select id into v_cat_rout  from cost_categories where stage_id = v_stage_mac and name = 'Router';
  select id into v_cat_uv    from cost_categories where stage_id = v_stage_mac and name = 'UV Printer';

  -- 1. Woodbased: Subcategories Birch, Rubberwood, Acacia
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_wood, 'Birch', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_birch;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_wood, 'Rubberwood', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_rub;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_wood, 'Acacia', 3) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_acac;

  -- Birch Varieties: 3mm, 4mm, 8mm, 12mm
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_birch, '3mm', 45, 'sq ft', 10, 1),
    (v_sub_birch, '4mm', 55, 'sq ft', 10, 2),
    (v_sub_birch, '8mm', 85, 'sq ft', 10, 3),
    (v_sub_birch, '12mm', 120, 'sq ft', 10, 4)
  on conflict (subcategory_id, name) do nothing;

  -- Rubberwood Varieties: 12mm, 18mm
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_rub, '12mm', 140, 'sq ft', 12, 1),
    (v_sub_rub, '18mm', 190, 'sq ft', 12, 2)
  on conflict (subcategory_id, name) do nothing;

  -- Acacia Varieties: 10mm, 15mm
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_acac, '10mm', 160, 'sq ft', 15, 1),
    (v_sub_acac, '15mm', 220, 'sq ft', 15, 2)
  on conflict (subcategory_id, name) do nothing;

  -- 2. Acrylic: Subcategories Cast Acrylic, Mirror Acrylic
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_acryl, 'Cast Acrylic', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_cast;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_acryl, 'Mirror Acrylic', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_mir_a;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_cast, '2mm Clear', 85, 'sq ft', 10, 1),
    (v_sub_cast, '3mm Clear/Color', 120, 'sq ft', 10, 2),
    (v_sub_cast, '5mm Clear/Color', 190, 'sq ft', 10, 3)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_mir_a, '2mm Gold Mirror', 140, 'sq ft', 10, 1),
    (v_sub_mir_a, '2mm Rose Gold Mirror', 150, 'sq ft', 10, 2)
  on conflict (subcategory_id, name) do nothing;

  -- 3. Glass: Clear Glass, Tinted Glass, Frosted Glass
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_glass, 'Clear Glass', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_c_gls;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_glass, 'Tinted Glass', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_t_gls;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_c_gls, '4mm', 45, 'sq ft', 8, 1),
    (v_sub_c_gls, '5mm', 60, 'sq ft', 8, 2)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_t_gls, '4mm Bronze', 65, 'sq ft', 10, 1),
    (v_sub_t_gls, '4mm Grey', 65, 'sq ft', 10, 2)
  on conflict (subcategory_id, name) do nothing;

  -- 4. Mirror: Plain Mirror, Tinted Mirror
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_mirr, 'Plain Mirror', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_p_mir;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_p_mir, '4mm', 55, 'sq ft', 8, 1)
  on conflict (subcategory_id, name) do nothing;

  -- 5. Leather: Faux Leather, Genuine Leather
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_leath, 'Faux Leather / Rexine', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_f_lea;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_f_lea, 'Tan / Brown Sheet', 65, 'sq ft', 10, 1),
    (v_sub_f_lea, 'Black Sheet', 65, 'sq ft', 10, 2)
  on conflict (subcategory_id, name) do nothing;

  -- 6. Hardware: Magnets
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_mag, 'Round Magnet', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_r_mag;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_mag, 'Cylinder Magnet', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_c_mag;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_r_mag, '8x2mm', 5, 'piece', 5, 1),
    (v_sub_r_mag, '10x2mm', 6, 'piece', 5, 2),
    (v_sub_r_mag, '12x3mm', 8, 'piece', 5, 3)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_c_mag, '5x10mm', 8, 'piece', 5, 1),
    (v_sub_c_mag, '6x12mm', 10, 'piece', 5, 2)
  on conflict (subcategory_id, name) do nothing;

  -- 7. Hardware: Hinges
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_hinge, 'Small Brass Hinge', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_b_hin;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_hinge, 'Concealed Hinge', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_c_hin;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_b_hin, '1.5 inch Brass', 25, 'piece', 0, 1),
    (v_sub_c_hin, '10mm Barrel', 45, 'piece', 0, 1)
  on conflict (subcategory_id, name) do nothing;

  -- 8. Hardware: Latch & Bush
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_latch, 'Antique Latch', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_a_lat;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_a_lat, 'Small Vintage Brass', 35, 'piece', 0, 1)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_bush, 'Rubber Bush', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_r_bsh;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_r_bsh, 'Standard (Pack of 4)', 12, 'piece', 0, 1)
  on conflict (subcategory_id, name) do nothing;

  -- 9. Finishing: Polish & Paint
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_pol, 'PU Polish', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_pu_p;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_pol, 'Melamine Polish', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_mel_p;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_pu_p, 'Matt', 35, 'sq ft', 8, 1),
    (v_sub_pu_p, 'Gloss', 40, 'sq ft', 8, 2)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_mel_p, 'Matt Finish', 25, 'sq ft', 8, 1)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_paint, 'Wood Stain', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_stn_p;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_stn_p, 'Walnut Stain', 25, 'sq ft', 5, 1),
    (v_sub_stn_p, 'Teak Stain', 25, 'sq ft', 5, 2)
  on conflict (subcategory_id, name) do nothing;

  -- 10. Machine: Laser, Router, UV Printer
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_laser, 'Laser Cutting', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_l_cut;
  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_laser, 'Laser Engraving', 2) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_l_eng;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_l_cut, '3mm / 4mm Sheet', 12, 'min', 5, 1),
    (v_sub_l_cut, '8mm / 12mm Sheet', 16, 'min', 5, 2)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_l_eng, 'Surface Vector', 12, 'min', 5, 1),
    (v_sub_l_eng, 'Deep Raster Fill', 15, 'min', 5, 2)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_rout, 'CNC 2D Profile', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_r_2d;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_r_2d, '12mm / 18mm Wood Cut', 18, 'min', 5, 1)
  on conflict (subcategory_id, name) do nothing;

  insert into cost_subcategories (category_id, name, sort_order) values
    (v_cat_uv, 'UV Flatbed Print', 1) on conflict (category_id, name) do update set name = excluded.name returning id into v_sub_uv_p;
  insert into cost_varieties (subcategory_id, name, default_rate, unit, default_wastage_pct, sort_order) values
    (v_sub_uv_p, 'Standard Color CMYK', 25, 'min', 5, 1),
    (v_sub_uv_p, 'CMYK + White Layer', 30, 'min', 5, 2)
  on conflict (subcategory_id, name) do nothing;

end $$;
