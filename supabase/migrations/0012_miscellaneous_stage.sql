-- 0012_miscellaneous_stage.sql
-- Adds the 5th costing stage. No categories/subcategories under it for now:
-- the calculator renders a free-text Description column for stages with no
-- categories, so Miscellaneous works as a plain name/rate/qty line.

insert into cost_stages (code, name, calculation_type, sort_order) values
  ('miscellaneous', 'Miscellaneous', 'piece', 5)
on conflict (code) do update set
  name = excluded.name,
  calculation_type = excluded.calculation_type,
  sort_order = excluded.sort_order,
  is_active = true;
