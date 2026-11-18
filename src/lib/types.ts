/**
 * Row shapes for the tables and views in supabase/migrations.
 *
 * Hand-written rather than generated, because generating requires a live
 * project. Once the project exists you can replace this file with:
 *   npx supabase gen types typescript --project-id <id> > src/lib/types.ts
 */

export type UserRole = "admin" | "manager" | "sales";
export type DocType = "quotation" | "proforma_invoice";

export type Profile = {
  id: string;
  full_name: string | null;
  role: UserRole;
  created_at: string;
};

export type Category = {
  id: string;
  name: string;
  code?: string | null;
  counts_as_item: boolean;
  sort_order: number;
  is_active: boolean;
};

export type ProductImage = {
  id: string;
  product_id: string;
  url: string;
  storage_path?: string | null;
  color?: string | null; // e.g. "Walnut", "Natural", "Black" or null for General/All
  color_code?: string | null; // e.g. "WL", "NT", "BL"
  is_primary: boolean;
  sort_order: number;
  caption?: string | null;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
};

export type Product = {
  id: string;
  code: string;
  category_id: string | null;
  name: string;
  source: string | null;
  cost_price: number;
  markup_pct?: number;
  target_margin: number;
  default_sp: number;
  colors?: string[];
  image_url?: string | null;
  images?: ProductImage[];
  is_active: boolean;
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
};

/** Product joined to its category, the shape the builder needs. */
export type ProductWithCategory = Product & {
  categories: { name: string; counts_as_item: boolean } | null;
};

export type CostStage = {
  id: string;
  code: string;
  name: string;
  calculation_type: "dimension" | "piece" | "time_based";
  sort_order: number;
  is_active: boolean;
};

export type CostCategory = {
  id: string;
  stage_id: string;
  name: string;
  sort_order: number;
  is_active: boolean;
};

export type CostSubcategory = {
  id: string;
  category_id: string;
  name: string;
  sort_order: number;
  is_active: boolean;
};

export type CostVariety = {
  id: string;
  subcategory_id: string;
  name: string; // e.g. '8mm', '12mm', '10x2mm'
  default_rate: number;
  unit: string;
  default_wastage_pct: number;
  notes: string | null;
  sort_order: number;
  is_active: boolean;
};

export type CostSubcategoryWithVarieties = CostSubcategory & {
  varieties: CostVariety[];
};

export type CostCategoryWithSubcategories = CostCategory & {
  subcategories: CostSubcategoryWithVarieties[];
};

export type CostStageWithHierarchy = CostStage & {
  categories: CostCategoryWithSubcategories[];
};

export type ProductCostLine = {
  id?: string;
  sheet_id?: string;
  stage_code: string; // 'material' | 'hardware' | 'finishing' | 'machine' | 'miscellaneous' | 'bought_out'
  category_name: string; // e.g. 'Woodbased'
  subcategory_name?: string | null; // e.g. 'Birch'
  variety_name?: string | null; // e.g. '8mm'
  item_name: string; // legacy or combined display
  cost_item_id?: string | null;
  cost_variety_id?: string | null;
  length?: number | null;
  breadth?: number | null;
  dimension_unit?: string; // 'inch' | 'mm' | 'cm'
  unit: string;
  rate: number;
  duration_minutes?: number | null;
  qty: number;
  wastage_pct: number;
  calculated_area?: number | null;
  line_total: number;
  /** MRP printed on a bought-out item, per piece and GST included; the SP must keep within it. */
  mrp?: number | null;
  sort_order?: number;
};

export type ProductCostSheet = {
  id: string;
  product_id: string | null;
  product_code: string | null;
  product_name: string;
  material_total: number;
  hardware_total: number;
  finishing_total: number;
  machine_total: number;
  total_cost: number;
  markup_pct: number;
  calculated_sp: number;
  /** Overhead % per stage code, already included in the stage totals. */
  stage_overheads?: Record<string, number> | null;
  notes: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
  lines?: ProductCostLine[];
};

export type * from "./types-docs.ts";
