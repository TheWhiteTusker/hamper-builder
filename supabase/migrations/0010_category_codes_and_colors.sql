-- 0010_category_codes_and_colors.sql
-- Add category code column, backfill 2-letter codes, and set standard colors

-- ---------------------------------------------------------------
-- 1. ADD CODE TO CATEGORIES
-- ---------------------------------------------------------------

alter table categories add column if not exists code text;

-- Backfill codes for standard categories
update categories set code = 'LC' where name ilike '%light%' or name ilike '%candle%';
update categories set code = 'ED' where name ilike 'edible%';
update categories set code = 'DC' where name ilike 'decor%';
update categories set code = 'UT' where name ilike 'utilit%';
update categories set code = 'GM' where name ilike 'game%';
update categories set code = 'BP' where name ilike 'box%packaging%';
update categories set code = 'IP' where name ilike 'inside%packaging%';
update categories set code = 'MS' where name ilike 'misc%';
update categories set code = 'DW' where name ilike 'drink%';

-- Fallback for any other categories: first 2 letters uppercase
update categories set code = upper(substring(regexp_replace(name, '[^a-zA-Z]', '', 'g') from 1 for 2))
where code is null or code = '';

-- Ensure index on category code
create index if not exists categories_code_idx on categories (code);

-- ---------------------------------------------------------------
-- 2. UPDATE APP_SETTINGS FOR 3 STANDARD COLORS ONLY
-- ---------------------------------------------------------------

insert into app_settings (key, value)
values ('product_colors', '["Walnut", "Natural", "Black"]'::jsonb)
on conflict (key) do update set value = excluded.value;
