import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import Link from "@/components/link";
import { requireUser } from "@/lib/supabase/server";
import { PageHeader } from "@/components/ui";

type Row = {
  id: string;
  title: string;
  updated_at: string;
  presentation_slides: { count: number }[];
};

export default async function PresentationsPage() {
  const { supabase } = await requireUser();
  const { data } = await supabase
    .from("presentations")
    .select("id, title, updated_at, presentation_slides(count)")
    .order("updated_at", { ascending: false })
    .returns<Row[]>();
  const rows = data ?? [];

  return (
    <>
      <PageHeader title="Presentations" subtitle="Client decks: a cover, a slide per hamper or product, and a closing slide">
        <Link href="/presentations/new" className="btn-primary">
          New presentation
        </Link>
      </PageHeader>

      <div className="card overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Title</TableHead>
              <TableHead className="num">Slides</TableHead>
              <TableHead>Last edited</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={3} className="py-10 text-center text-sm text-[var(--color-muted)]">
                  No presentations yet.
                  <Link href="/presentations/new" className="ml-2 underline">
                    Make the first one
                  </Link>
                </TableCell>
              </TableRow>
            ) : (
              rows.map((p) => (
                <TableRow key={p.id}>
                  <TableCell>
                    <Link href={`/presentations/${p.id}`} className="font-medium hover:underline">
                      {p.title}
                    </Link>
                  </TableCell>
                  <TableCell className="num">{p.presentation_slides[0]?.count ?? 0}</TableCell>
                  <TableCell className="text-[var(--color-muted)]">
                    {new Date(p.updated_at).toLocaleString("en-IN", { dateStyle: "medium", timeStyle: "short" })}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </>
  );
}
