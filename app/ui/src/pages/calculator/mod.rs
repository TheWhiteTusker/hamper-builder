//! The Product Cost Calculator (port of `cost-calculator/_calculator/*`):
//! product details with the auto-built code, one table per cost stage with
//! Category → Subcategory → Variety pickers, markup ↔ SP, the MRP cap, colour
//! rows with photos, and Save. Every total is worked out on the device as you type.

mod save;
mod view;

use crate::data::{ProductCosting, Sibling, load_product_costing, use_categories, use_cost_rows, use_settings};
use crate::db::live_with;
use crate::kit::decode_code;
use lattice_core::api::ApiError;
use lattice_core::colors::resolve_colors;
use lattice_core::cost_tree::{StageNode, build_cost_tree};
use lattice_core::costing::{
    CostSheetTotals, ProductCostLine, StageOverheads, calculate_cost_sheet_totals, calculate_line_cost,
};
use lattice_core::images::ProductImage;
use lattice_core::js::{self, dec, f};
use lattice_core::mrp::{mrp_limit, round_sp};
use lattice_core::numbers::{parse_num, round_up_to_next10};
use lattice_core::product_code::{ParsedCode, parse_product_code};
use lattice_core::records::Category;
use lattice_core::{Decimal, colors::ProductColor};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
pub use save::{line_row, reprice_varieties};
use std::collections::BTreeMap;
pub use view::Tabs;

pub const ORIGINS: [&str; 3] = ["In-house", "Outsource", "Hybrid"];
pub const BOUGHT_OUT: &str = "bought_out";

/// A saved number as JS `String(n)` printed it, for an input's text.
pub fn num_text(d: Decimal) -> String {
    d.normalize().to_string()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub key: u64,
    pub l: ProductCostLine,
}

/// A photo picked before the product's first save, uploaded right after it.
#[derive(Clone)]
pub struct Pending {
    pub key: u64,
    pub color: String,
    pub file: web_sys::File,
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UploadStatus {
    pub color: String,
    pub busy: bool,
    pub error: Option<String>,
    pub ok: Option<String>,
}

/// Every piece of the calculator's state. All signals, so it is `Copy`.
#[derive(Clone, Copy)]
pub struct Calc {
    pub stages: StoredValue<Vec<StageNode>>,
    pub categories: StoredValue<Vec<Category>>,
    pub palette: StoredValue<Vec<ProductColor>>,
    pub siblings: StoredValue<Vec<Sibling>>,
    pub lines: RwSignal<Vec<Line>>,
    next_key: StoredValue<u64>,
    pub editing: bool,
    pub own_color: StoredValue<String>,
    pub sibling_colors: StoredValue<Vec<String>>,
    pub initial: StoredValue<Option<ProductCosting>>,
    pub product_id: RwSignal<String>,
    pub code: RwSignal<String>,
    pub name: RwSignal<String>,
    pub category_id: RwSignal<String>,
    pub source: RwSignal<String>,
    pub collapsed: RwSignal<Vec<String>>,
    pub colors: RwSignal<Vec<String>>,
    pub is_active: RwSignal<bool>,
    pub notes: RwSignal<String>,
    pub fetching_code: RwSignal<bool>,
    pub markup: RwSignal<String>,
    pub manual_sp: RwSignal<String>,
    pub overheads: RwSignal<BTreeMap<String, String>>,
    pub gst: Decimal,
    pub totals: Memo<CostSheetTotals>,
    pub mrp_max: Memo<Option<Decimal>>,
    pub images: RwSignal<Vec<ProductImage>>,
    pub pending: RwSignal<Vec<Pending>, LocalStorage>,
    pub upload: RwSignal<Option<UploadStatus>>,
    pub variant_ids: RwSignal<BTreeMap<String, String>>,
    pub busy: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub success: RwSignal<Option<String>>,
    pub asking_price: RwSignal<bool>,
}

/// The saved origin matched to one of [`ORIGINS`]; In-house when unknown.
fn match_origin(raw: Option<&str>) -> String {
    let raw = raw.unwrap_or("In-house").to_lowercase();
    ORIGINS.iter().find(|o| o.to_lowercase() == raw).unwrap_or(&"In-house").to_string()
}

/// A starter row: nothing picked yet, the unit the stage usually prices by.
pub fn empty_line(stage: &StageNode, key: u64) -> Line {
    let code = stage.row.code.as_str();
    let mut l = ProductCostLine {
        stage_code: code.into(),
        dimension_unit: Some("inch".into()),
        unit: match code {
            "machine" => "min",
            BOUGHT_OUT => "piece",
            _ if !stage.categories.is_empty() => "sq ft",
            _ => "piece",
        }
        .into(),
        qty: Some(Decimal::ONE),
        duration_minutes: (code == "machine").then(|| Decimal::from(15)),
        calculated_area: Some(Decimal::ONE),
        ..Default::default()
    };
    l.line_total = calculate_line_cost(&l).line_total;
    Line { key, l }
}

/// A saved line brought up to its master variety: current names, rate, unit
/// and wastage, found by id so a rename in the master still matches.
fn with_master(mut l: ProductCostLine, stages: &[StageNode]) -> ProductCostLine {
    let Some(vid) = l.cost_variety_id.clone() else { return l };
    for stage in stages {
        for cat in &stage.categories {
            for sub in &cat.subcategories {
                if let Some(v) = sub.varieties.iter().find(|v| v.id == vid) {
                    l.category_name.clone_from(&cat.row.name);
                    l.subcategory_name = Some(sub.row.name.clone());
                    l.variety_name = Some(v.name.clone());
                    l.item_name = format!("{} {}", sub.row.name, v.name);
                    l.rate = v.default_rate;
                    l.unit.clone_from(&v.unit);
                    l.wastage_pct = if l.stage_code == BOUGHT_OUT { Decimal::ZERO } else { v.default_wastage_pct };
                    let calc = calculate_line_cost(&l);
                    l.calculated_area = Some(calc.calculated_area);
                    l.line_total = calc.line_total;
                    return l;
                }
            }
        }
    }
    l
}

/// The lines a save sends: starter rows left empty are skipped.
pub fn saved_lines(lines: &[Line]) -> Vec<ProductCostLine> {
    lines
        .iter()
        .filter(|x| !x.l.category_name.is_empty() || !x.l.item_name.is_empty() || x.l.rate > Decimal::ZERO)
        .map(|x| ProductCostLine { id: None, sheet_id: None, sort_order: None, cost_item_id: None, ..x.l.clone() })
        .collect()
}

pub struct Init {
    pub stages: Vec<StageNode>,
    pub categories: Vec<Category>,
    pub palette: Vec<ProductColor>,
    pub gst: Decimal,
    pub costing: Option<ProductCosting>,
}

impl Calc {
    pub fn new(init: Init) -> Self {
        let Init { stages, categories, palette, gst, costing } = init;
        let product = costing.as_ref().map(|c| c.product.clone());
        let sheet = costing.as_ref().and_then(|c| c.sheet.clone());
        let editing = product.is_some();
        let own_color = costing.as_ref().map(|c| c.own_color.clone()).unwrap_or_default();
        let siblings = costing.as_ref().map(|c| c.siblings.clone()).unwrap_or_default();
        let mut key = 0;
        let lines: Vec<Line> = match sheet.as_ref().map(|s| &s.lines).filter(|l| !l.is_empty()) {
            // Saved lines show the master's current names and rates.
            Some(saved) => saved
                .iter()
                .map(|l| {
                    key += 1;
                    Line { key, l: with_master(l.clone(), &stages) }
                })
                .collect(),
            // One empty row per stage, ready to be filled from the dropdowns.
            None => stages
                .iter()
                .map(|s| {
                    key += 1;
                    empty_line(s, key)
                })
                .collect(),
        };
        let source = match_origin(product.as_ref().and_then(|p| p.source.as_deref()));
        let collapsed = if product
            .as_ref()
            .and_then(|p| p.source.as_deref())
            .is_some_and(|s| s.eq_ignore_ascii_case("outsource"))
        {
            stages.iter().filter(|s| s.row.code != BOUGHT_OUT).map(|s| s.row.code.clone()).collect()
        } else {
            Vec::new()
        };
        let markup = product
            .as_ref()
            .and_then(|p| p.markup_pct)
            .or(sheet.as_ref().map(|s| s.sheet.markup_pct))
            .map_or_else(|| "50".to_string(), num_text);
        let overheads: BTreeMap<String, String> = sheet
            .as_ref()
            .map(|s| s.sheet.stage_overheads.iter().map(|(k, v)| (k.clone(), num_text(*v))).collect())
            .unwrap_or_default();

        let lines = RwSignal::new(lines);
        let markup = RwSignal::new(markup);
        let overheads = RwSignal::new(overheads);
        let parsed_overheads = move || -> StageOverheads {
            overheads.with(|o| o.iter().map(|(k, v)| (k.clone(), parse_num(v))).collect())
        };
        let totals = Memo::new(move |_| {
            let ls: Vec<ProductCostLine> = lines.with(|l| l.iter().map(|x| x.l.clone()).collect());
            calculate_cost_sheet_totals(&ls, parse_num(&markup.get()), &parsed_overheads())
        });
        let mrp_max = Memo::new(move |_| {
            let ls: Vec<ProductCostLine> = lines.with(|l| l.iter().map(|x| x.l.clone()).collect());
            mrp_limit(&ls, Some(&parsed_overheads()), totals.get().total_cost, gst)
        });

        let calc = Self {
            stages: StoredValue::new(stages),
            categories: StoredValue::new(categories),
            palette: StoredValue::new(palette),
            siblings: StoredValue::new(siblings.clone()),
            lines,
            next_key: StoredValue::new(key),
            editing,
            own_color: StoredValue::new(own_color.clone()),
            sibling_colors: StoredValue::new(siblings.iter().map(|s| s.color.clone()).collect()),
            initial: StoredValue::new(costing.clone()),
            product_id: RwSignal::new(product.as_ref().map(|p| p.id.clone()).unwrap_or_default()),
            code: RwSignal::new(
                product
                    .as_ref()
                    .map(|p| p.code.clone())
                    .or(sheet.as_ref().and_then(|s| s.sheet.product_code.clone()))
                    .unwrap_or_default(),
            ),
            name: RwSignal::new(
                product
                    .as_ref()
                    .map(|p| p.name.clone())
                    .or(sheet.as_ref().map(|s| s.sheet.product_name.clone()))
                    .unwrap_or_default(),
            ),
            category_id: RwSignal::new(product.as_ref().and_then(|p| p.category_id.clone()).unwrap_or_default()),
            source: RwSignal::new(source),
            collapsed: RwSignal::new(collapsed),
            // Editing: this product's colour first (fixed), then any new ones added.
            colors: RwSignal::new(if editing {
                [own_color].into_iter().filter(|c| !c.is_empty()).collect()
            } else {
                vec!["Walnut".into()]
            }),
            is_active: RwSignal::new(product.as_ref().is_none_or(|p| p.is_active)),
            notes: RwSignal::new(sheet.as_ref().and_then(|s| s.sheet.notes.clone()).unwrap_or_default()),
            fetching_code: RwSignal::new(false),
            markup,
            manual_sp: RwSignal::new(String::new()),
            overheads,
            gst,
            totals,
            mrp_max,
            images: RwSignal::new(costing.as_ref().map(|c| c.images.clone()).unwrap_or_default()),
            pending: RwSignal::new_local(Vec::new()),
            upload: RwSignal::new(None),
            variant_ids: RwSignal::new(costing.as_ref().map(|c| c.variant_ids.clone()).unwrap_or_default()),
            busy: RwSignal::new(false),
            error: RwSignal::new(None),
            success: RwSignal::new(None),
            asking_price: RwSignal::new(false),
        };

        // Start with the price the current rates give, as changing the markup
        // would; keep the saved price for products without costing lines.
        let t = calc.totals.get_untracked();
        let sp = if t.total_cost > Decimal::ZERO {
            num_text(calc.sp_at(&calc.markup.get_untracked()).unwrap_or(t.calculated_sp))
        } else {
            let saved = product
                .as_ref()
                .map(|p| p.default_sp)
                .or(sheet.as_ref().map(|s| s.sheet.calculated_sp))
                .unwrap_or_default();
            let saved = round_up_to_next10(saved);
            if saved > Decimal::ZERO { num_text(saved) } else { String::new() }
        };
        calc.manual_sp.set(sp);
        calc
    }

    pub fn key(&self) -> u64 {
        let k = self.next_key.get_value() + 1;
        self.next_key.set_value(k);
        k
    }

    pub fn parsed_code(&self) -> ParsedCode {
        parse_product_code(&self.code.get())
    }

    // ------------------------------------------------------------ lines

    /// Edits one line and recomputes its total.
    pub fn update(&self, key: u64, edit: impl FnOnce(&mut ProductCostLine)) {
        self.lines.update(|ls| {
            if let Some(x) = ls.iter_mut().find(|x| x.key == key) {
                edit(&mut x.l);
                let calc = calculate_line_cost(&x.l);
                x.l.calculated_area = Some(calc.calculated_area);
                x.l.line_total = calc.line_total;
            }
        });
    }

    /// Category changed: subcategory and variety are picked again in turn.
    pub fn set_category(&self, key: u64, name: String) {
        self.update(key, |l| {
            l.category_name = name;
            l.subcategory_name = Some(String::new());
            l.variety_name = Some(String::new());
            l.item_name = String::new();
            l.cost_variety_id = None;
            l.rate = Decimal::ZERO;
            l.wastage_pct = Decimal::ZERO;
        });
    }

    pub fn set_subcategory(&self, key: u64, name: String) {
        self.update(key, |l| {
            l.subcategory_name = Some(name.clone());
            l.variety_name = Some(String::new());
            l.item_name = name;
            l.cost_variety_id = None;
            l.rate = Decimal::ZERO;
            l.wastage_pct = Decimal::ZERO;
        });
    }

    /// A variety brings its default rate, unit and wastage.
    pub fn set_variety(&self, key: u64, name: String) {
        let Some(line) = self.lines.with_untracked(|ls| ls.iter().find(|x| x.key == key).map(|x| x.l.clone())) else {
            return;
        };
        let v = self.stages.with_value(|stages| {
            stages
                .iter()
                .find(|s| s.row.code == line.stage_code)
                .and_then(|s| s.categories.iter().find(|c| c.row.name == line.category_name))
                .and_then(|c| c.subcategories.iter().find(|s| Some(&s.row.name) == line.subcategory_name.as_ref()))
                .map(|s| (s.row.name.clone(), s.varieties.iter().find(|v| v.name == name).cloned()))
        });
        self.update(key, |l| {
            l.variety_name = Some(name.clone());
            match v {
                Some((sub, Some(v))) => {
                    l.item_name = format!("{sub} {}", v.name);
                    l.cost_variety_id = Some(v.id);
                    l.unit = v.unit;
                    l.rate = v.default_rate;
                    l.wastage_pct = if l.stage_code == BOUGHT_OUT { Decimal::ZERO } else { v.default_wastage_pct };
                }
                Some((sub, None)) => {
                    l.item_name = format!("{sub} {name}");
                    l.cost_variety_id = None;
                }
                None => {
                    l.item_name.clone_from(&name);
                    l.cost_variety_id = None;
                }
            }
        });
    }

    pub fn duplicate(&self, key: u64) {
        let new = self.key();
        self.lines.update(|ls| {
            if let Some(i) = ls.iter().position(|x| x.key == key)
                && let Some(copy) = ls.get(i).map(|x| Line { key: new, l: x.l.clone() })
            {
                ls.insert(i + 1, copy);
            }
        });
    }

    pub fn remove(&self, key: u64) {
        self.lines.update(|ls| ls.retain(|x| x.key != key));
    }

    pub fn add_to_stage(&self, code: &str) {
        let key = self.key();
        if let Some(line) =
            self.stages.with_value(|s| s.iter().find(|s| s.row.code == code).map(|s| empty_line(s, key)))
        {
            self.lines.update(|ls| ls.push(line));
        }
    }

    // ---------------------------------------------------------- pricing

    pub fn overhead_for(&self, stage: &str) -> String {
        self.overheads.with(|o| o.get(&stage.to_lowercase()).cloned().unwrap_or_default())
    }

    pub fn set_overhead(&self, stage: &str, pct: String) {
        self.overheads.update(|o| {
            o.insert(stage.to_lowercase(), pct);
        });
    }

    /// The SP a markup gives, or None at 100%+ where there is no finite price.
    pub fn sp_at(&self, pct: &str) -> Option<Decimal> {
        let divisor = 1.0 - f(parse_num(pct)) / 100.0;
        (divisor > 0.0).then(|| round_sp(dec(f(self.totals.get().total_cost) / divisor), self.mrp_max.get()))
    }

    pub fn effective_sp(&self) -> Decimal {
        let manual = self.manual_sp.get();
        if manual.is_empty() {
            self.sp_at(&self.markup.get()).unwrap_or(self.totals.get().calculated_sp)
        } else {
            round_sp(parse_num(&manual), self.mrp_max.get())
        }
    }

    pub fn effective_margin(&self) -> Decimal {
        let (sp, cp) = (f(self.effective_sp()), f(self.totals.get().total_cost));
        dec(if sp > 0.0 { (sp - cp) / sp } else { 0.0 })
    }

    pub fn over_mrp(&self) -> bool {
        self.mrp_max.get().is_some_and(|m| self.effective_sp() > m)
    }

    /// Markup and SP follow each other: a new markup sets the SP.
    pub fn change_markup(&self, v: String) {
        self.markup.set(v.clone());
        if let Some(sp) = self.sp_at(&v) {
            self.manual_sp.set(num_text(sp));
        }
    }

    fn markup_from(&self, sp: Decimal) -> String {
        let (sp, cp) = (f(sp), f(self.totals.get_untracked().total_cost));
        num_text(dec(js::round2((sp - cp) / sp * 100.0)))
    }

    /// …and a typed SP sets the markup.
    pub fn change_sp(&self, v: String) {
        self.manual_sp.set(v.clone());
        let sp = parse_num(&v);
        if self.totals.get_untracked().total_cost > Decimal::ZERO && sp > Decimal::ZERO {
            self.markup.set(self.markup_from(sp));
        }
    }

    /// On leaving the SP box: round it up to the next 10, within the MRP limit.
    pub fn round_manual_sp(&self) {
        let manual = self.manual_sp.get_untracked();
        if manual.is_empty() {
            return;
        }
        let rounded = round_sp(parse_num(&manual), self.mrp_max.get_untracked());
        if rounded > Decimal::ZERO && num_text(rounded) != manual {
            self.manual_sp.set(num_text(rounded));
            if self.totals.get_untracked().total_cost > Decimal::ZERO {
                self.markup.set(self.markup_from(rounded));
            }
        }
    }

    /// The overheads to save: numbers, blanks and zeros left out.
    pub fn stage_overheads(&self) -> StageOverheads {
        self.overheads.with_untracked(|o| {
            o.iter().map(|(k, v)| (k.clone(), parse_num(v))).filter(|(_, v)| !v.is_zero()).collect()
        })
    }

    // ---------------------------------------------------------- details

    pub fn change_origin(&self, origin: String) {
        let collapsed = if origin == "Outsource" {
            self.stages
                .with_value(|s| s.iter().filter(|s| s.row.code != BOUGHT_OUT).map(|s| s.row.code.clone()).collect())
        } else {
            Vec::new()
        };
        self.source.set(origin);
        self.collapsed.set(collapsed);
    }

    pub fn toggle_stage(&self, code: &str) {
        self.collapsed.update(|c| {
            if let Some(i) = c.iter().position(|x| x == code) {
                c.remove(i);
            } else {
                c.push(code.to_string());
            }
        });
    }

    fn suffix(&self, colors: &[String]) -> String {
        colors.first().map_or_else(
            || Some(self.parsed_code().color_code).filter(|c| !c.is_empty()).unwrap_or_else(|| "WL".into()),
            |c| lattice_core::colors::color_code(c),
        )
    }

    /// The category is part of the code: choosing one takes the next free serial in it.
    pub fn change_category(&self, id: String) {
        if self.editing {
            return;
        }
        self.category_id.set(id.clone());
        let Some(cat) = self.categories.with_value(|c| c.iter().find(|c| c.id == id).cloned()) else { return };
        let cat_code =
            cat.code.filter(|c| !c.is_empty()).unwrap_or_else(|| lattice_core::colors::derive_category_code(&cat.name));
        let current = self.parsed_code();
        let this = *self;
        if current.category_code == cat_code && current.serial.parse::<u32>().is_ok_and(|n| n > 0) {
            let colors = self.colors.get_untracked();
            self.code.set(lattice_core::product_code::format_product_code(
                &cat_code,
                &current.serial,
                &this.suffix(&colors),
            ));
            return;
        }
        self.fetching_code.set(true);
        leptos::task::spawn_local(async move {
            let serial = save::next_serial(&cat_code).await;
            // By now the colours may have changed: use the latest.
            let colors = this.colors.get_untracked();
            this.code.set(lattice_core::product_code::format_product_code(&cat_code, &serial, &this.suffix(&colors)));
            this.fetching_code.set(false);
        });
    }

    /// A product comes in finishes; the code's suffix follows the first one.
    pub fn toggle_color(&self, name: &str) {
        if self.editing && self.own_color.with_value(|o| o == name) {
            return;
        }
        let mut next = self.colors.get_untracked();
        if let Some(i) = next.iter().position(|c| c == name) {
            next.remove(i);
        } else {
            next.push(name.to_string());
        }
        self.colors.set(next.clone());
        if !self.editing {
            let current = self.parsed_code();
            if let Some(first) = next.first()
                && !current.category_code.is_empty()
            {
                self.code.set(lattice_core::product_code::format_product_code(
                    &current.category_code,
                    &current.serial,
                    &lattice_core::colors::color_code(first),
                ));
            }
        }
    }

    /// Everything a save would send, for the leave guard.
    pub fn snapshot(&self) -> String {
        let lines = saved_lines(&self.lines.get());
        format!(
            "{:?}",
            (
                self.name.get(),
                self.category_id.get(),
                self.source.get(),
                self.colors.get(),
                self.is_active.get(),
                self.notes.get(),
                self.markup.get(),
                self.manual_sp.get(),
                self.stage_overheads(),
                lines,
                self.pending.with(Vec::len),
            )
        )
    }
}

/// `/products/new`: a blank calculator.
#[component]
pub fn NewProduct() -> impl IntoView {
    view! { <Screen code=None /> }
}

/// `/cost-calculator/LC/0001/WL`: the calculator loaded with one product.
#[component]
pub fn ProductCostingPage() -> impl IntoView {
    let params = use_params_map();
    let code = Memo::new(move |_| decode_code(&params.with(|p| p.get("code").unwrap_or_default())));
    view! { {move || view! { <Screen code=Some(code.get()) /> }} }
}

#[component]
fn Screen(code: Option<String>) -> impl IntoView {
    let rows = use_cost_rows();
    let categories = use_categories();
    let settings = use_settings();
    let costing = code.clone().map(|c| {
        live_with(format!("product:{c}"), move || {
            let c = c.clone();
            async move {
                load_product_costing(&c)
                    .await
                    .and_then(|p| p.ok_or_else(|| ApiError::NotFound("No such product.".into())))
            }
        })
    });
    // The calculator starts from the first complete copy (cached or fresh) and
    // then owns its state; later refreshes must not wipe what is being typed.
    let ready = Memo::new(move |_| {
        let rows = rows.data.get()?;
        let cats = categories.data.get()?;
        let s = settings.data.get()?;
        let pc = match &costing {
            Some(c) => Some(c.data.get()?),
            None => None,
        };
        Some((rows, cats, s, pc))
    });
    // Created once, under this screen's owner: a background refresh must not
    // rebuild it (or dispose its signals) while someone is typing.
    let calc = RwSignal::new(None::<Calc>);
    let owner = Owner::current();
    Effect::new(move |_| {
        if calc.get_untracked().is_some() {
            return;
        }
        let Some((rows, cats, s, pc)) = ready.get() else { return };
        let hex = s.color_hex.clone().into_iter().collect();
        let init = Init {
            stages: build_cost_tree(&rows),
            categories: cats,
            palette: resolve_colors(&s.product_colors, &hex),
            gst: s.gst_rate,
            costing: pc,
        };
        let made = match &owner {
            Some(o) => o.with(|| Calc::new(init)),
            None => Calc::new(init),
        };
        calc.set(Some(made));
    });
    let error =
        move || costing.as_ref().and_then(|c| c.error.get()).or_else(|| rows.error.get()).map(|e| e.to_string());

    view! {
        <crate::components::PageHeader title="Product Cost Calculator" subtitle="Work out what a product costs: materials, hardware, finishing and machine time">
            <view::Tabs master=false />
        </crate::components::PageHeader>
        // Mounted once: only `calc` (set a single time) is tracked here.
        <Show
            when=move || calc.with(Option::is_some)
            fallback=move || match error() {
                Some(e) => view! { <div class="card p-6 text-sm text-red-800">{e}</div> }.into_any(),
                None => view! { <div class="card h-96 animate-pulse bg-white/60"></div> }.into_any(),
            }
        >
            {move || calc.get().map(|c| view! { <view::CalculatorView calc=c /> })}
        </Show>
    }
}
