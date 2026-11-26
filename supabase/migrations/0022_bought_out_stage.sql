-- 0022_bought_out_stage.sql
-- Adds Bought Out Items as a 6th costing stage/type in Rates & Hierarchy.
-- Supports Category -> Subcategory -> Variety hierarchy and dropdowns.

insert into cost_stages (code, name, calculation_type, sort_order) values
  ('bought_out', 'Bought Out Items', 'piece', 6)
on conflict (code) do update set
  name = excluded.name,
  calculation_type = excluded.calculation_type,
  sort_order = excluded.sort_order,
  is_active = true;
