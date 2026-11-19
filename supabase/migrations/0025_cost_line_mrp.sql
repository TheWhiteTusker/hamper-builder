-- 0025_cost_line_mrp.sql
-- MRP printed on a bought-out item, per piece and GST included. Null when the
-- item has none. The product's selling price must keep within it.

alter table product_cost_lines add column if not exists mrp numeric check (mrp > 0);
