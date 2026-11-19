-- Default settings.
--
-- Where the Apps Script pinned a value, that value is used here:
--   "Absorb into Box & Packaging"  addToQuote(), column J
--   "Combined Order"               the =IF(B7="Combined Order",...) totals
--   "15 Days"                      newQuote(), cell I4
--   0.18                           newQuote(), cell H27
--   "Quotation" / "Proforma Invoice" and the LLQT-/LLPI- prefixes  saveQuote()
--
-- The pick-lists are editable in Settings, and the importer adds any value it
-- finds in the spreadsheet that is missing here - so real-world strings win.

insert into app_settings (key, value) values
  ('company', jsonb_build_object(
     'name', 'Lattice Lane',
     'address', '',
     'gstin', '',
     'phone', '',
     'email', '',
     'website', ''
   )),

  ('gst_rate', to_jsonb(0.18)),

  ('doc_prefixes', jsonb_build_object(
     'hamper', 'H',
     'quotation', 'LLQT-',
     'proforma_invoice', 'LLPI-'
   )),

  ('hamper_statuses',   '["Draft","Approved","Active","Discontinued"]'::jsonb),
  ('quote_statuses',    '["Draft","Sent","Negotiation","Won","Lost","Expired"]'::jsonb),
  ('quote_structures',  '["Combined Order","Option Based"]'::jsonb),
  ('validity_options',  '["7 Days","15 Days","30 Days"]'::jsonb),
  ('collections',       '[]'::jsonb),
  ('sources',           '[]'::jsonb),

  -- Settings!H2 in the sheet. Controls whether a quotation itemises the
  -- contents of each hamper. The imported H2 value replaces this default.
  ('detail_modes',      '["Show Contents","Summary Only","Hide Contents"]'::jsonb),
  ('default_detail_mode', '"Show Contents"'::jsonb),

  ('packaging_treatments',
     '["Absorb into Box & Packaging","Charge Separately","Show as Line Item"]'::jsonb),
  ('default_packaging_treatment', '"Absorb into Box & Packaging"'::jsonb),

  ('default_validity', '"15 Days"'::jsonb),

  ('quote_terms',
     to_jsonb('1. Prices are valid for the stated validity period.
2. Taxes as applicable.
3. Delivery timelines confirmed on receipt of purchase order.
4. Images are indicative; contents may vary subject to availability.'::text))
on conflict (key) do nothing;

-- Numbering starts where the spreadsheet left off once data is imported;
-- see sync_doc_counters(). Until then these begin at 001.
insert into doc_counters (prefix, last_number) values
  ('H', 0), ('LLQT-', 0), ('LLPI-', 0)
on conflict (prefix) do nothing;
