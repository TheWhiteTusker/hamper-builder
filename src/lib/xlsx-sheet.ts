/**
 * An .xlsx whose tabs each have a coloured, frozen header row.
 *
 * SheetJS community edition cannot write styles or frozen panes, so the few
 * XML parts are written here and zipped with the CFB library SheetJS ships.
 */
import { CFB } from "xlsx";

/** A formula (no leading "=") and the value it currently works out to, if known. */
export type Formula = { f: string; v?: string | number };
export type Cell = string | number | null | undefined | Formula;
/** `formula` marks a calculated column: grey heading, and filled down the blank rows. */
export type Column = { header: string; width: number; format?: "percent" | "money"; formula?: (row: number) => string };

// Style ids into cellXfs below: 0 plain, 1 header, 2 percent, 3 money, 4 calculated header.
const STYLE = { percent: 2, money: 3 } as const;

const esc = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const col = (i: number) => String.fromCharCode(65 + i); // ponytail: A–Z only, fine below 27 columns

function cellXml(value: Cell, ref: string, style: number) {
  if (value === null || value === undefined || value === "") return "";
  const s = style ? ` s="${style}"` : "";
  if (typeof value === "object") {
    const { f, v } = value;
    const cached = v === undefined ? "" : `<v>${typeof v === "number" ? v : esc(v)}</v>`;
    return `<c r="${ref}"${s}${typeof v === "number" ? "" : ` t="str"`}><f>${esc(f)}</f>${cached}</c>`;
  }
  return typeof value === "number"
    ? `<c r="${ref}"${s}><v>${value}</v></c>`
    : `<c r="${ref}"${s} t="inlineStr"><is><t xml:space="preserve">${esc(value)}</t></is></c>`;
}

const NS = `xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"`;
const REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PKG = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT = "application/vnd.openxmlformats-officedocument.spreadsheetml";
const XML = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n`;

const STYLES = `${XML}<styleSheet ${NS}>
<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font></fonts>
<fills count="4"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF1F4E78"/><bgColor indexed="64"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FF6B7280"/><bgColor indexed="64"/></patternFill></fill></fills>
<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="5">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/>
<xf numFmtId="10" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="4" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="0" fontId="1" fillId="3" borderId="0" xfId="0" applyFont="1" applyFill="1"/>
</cellXfs>
<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>`;

/** An empty row `r` that still carries the column formulas, so a row typed into works out like the rest. */
export const formulaRow = (columns: Column[], r: number): Cell[] => columns.map((c) => (c.formula ? { f: c.formula(r) } : null));

function sheetXml(columns: Column[], data: Cell[][], blankRows: number) {
  const styles = columns.map((c) => (c.format ? STYLE[c.format] : 0));
  const header = `<row r="1">${columns.map((c, i) => cellXml(c.header, `${col(i)}1`, c.formula ? 4 : 1)).join("")}</row>`;
  const blanks = Array.from({ length: blankRows }, (_, b) => formulaRow(columns, data.length + b + 2));
  const body = [...data, ...blanks]
    .map((row, r) => {
      const n = r + 2;
      return `<row r="${n}">${row.map((v, i) => cellXml(v, `${col(i)}${n}`, styles[i] ?? 0)).join("")}</row>`;
    })
    .join("");
  // Column styles make cells typed into a blank template pick up the formats too.
  const cols = columns
    .map((c, i) => `<col min="${i + 1}" max="${i + 1}" width="${c.width}" customWidth="1"${styles[i] ? ` style="${styles[i]}"` : ""}/>`)
    .join("");
  return `${XML}<worksheet ${NS}><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/><selection pane="bottomLeft" activeCell="A2" sqref="A2"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight="15"/><cols>${cols}</cols><sheetData>${header}${body}</sheetData></worksheet>`;
}

/** One tab: `rows` (row 2 onwards), then `blankRows` empty rows holding the formulas. */
export type Sheet = { name: string; columns: Column[]; rows: Cell[][]; blankRows?: number };

/** Bytes of a workbook with one tab per sheet, in order. Formulas may refer across tabs. */
export function buildWorkbook(sheets: Sheet[]): Uint8Array {
  const tabs = sheets.map((s, i) => ({ ...s, n: i + 1 }));
  const parts: Record<string, string> = {
    "[Content_Types].xml": `${XML}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="${CT}.sheet.main+xml"/>${tabs.map(({ n }) => `<Override PartName="/xl/worksheets/sheet${n}.xml" ContentType="${CT}.worksheet+xml"/>`).join("")}<Override PartName="/xl/styles.xml" ContentType="${CT}.styles+xml"/></Types>`,
    "_rels/.rels": `${XML}<Relationships xmlns="${PKG}"><Relationship Id="rId1" Type="${REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>`,
    "xl/workbook.xml": `${XML}<workbook ${NS} xmlns:r="${REL}"><sheets>${tabs.map(({ name, n }) => `<sheet name="${esc(name)}" sheetId="${n}" r:id="rId${n}"/>`).join("")}</sheets><calcPr fullCalcOnLoad="1"/></workbook>`,
    "xl/_rels/workbook.xml.rels": `${XML}<Relationships xmlns="${PKG}">${tabs.map(({ n }) => `<Relationship Id="rId${n}" Type="${REL}/worksheet" Target="worksheets/sheet${n}.xml"/>`).join("")}<Relationship Id="rId${tabs.length + 1}" Type="${REL}/styles" Target="styles.xml"/></Relationships>`,
    "xl/styles.xml": STYLES,
  };
  for (const t of tabs) parts[`xl/worksheets/sheet${t.n}.xml`] = sheetXml(t.columns, t.rows, t.blankRows ?? 300);

  const zip = CFB.utils.cfb_new({ root: "R" });
  const utf8 = new TextEncoder();
  for (const [path, xml] of Object.entries(parts)) CFB.utils.cfb_add(zip, path, utf8.encode(xml));
  return new Uint8Array(CFB.write(zip, { fileType: "zip", type: "array", compression: true }) as number[]);
}

/** A one-tab workbook. */
export const buildXlsx = (name: string, columns: Column[], rows: Cell[][], blankRows = 300) =>
  buildWorkbook([{ name, columns, rows, blankRows }]);
