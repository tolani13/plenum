// Leaderboards — reps / items / customers, each an instrument table with a
// footer totals row (the anchor surface) and a client-side CSV of the current,
// RLS-scoped, filtered view. Fetch-all (≤200 rows) means client ranking, user
// sorting, and CSV are all honest against the same complete result set.

import { useSearchParams } from "react-router";
import { Download } from "lucide-react";
import {
  parseBasis,
  parseGroup,
  parseKind,
  parsePeriod,
  parseTab,
} from "../lib/params";
import type { Basis, Group, Kind } from "../lib/params";
import { useCustomers, useItems, useLeaderboard } from "../lib/queries";
import { ascNumNullsLast, ascStr, chain, descBasis, ranked } from "../lib/rank";
import type { CustomerRow, ItemRow, LeaderboardRow } from "../lib/types";
import { csvFilename, downloadCsv } from "../lib/csv";
import { useScreenReady } from "../lib/useScreenReady";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/states";
import { Controls, type ControlsState } from "./Controls";
import { DataTable } from "./DataTable";
import {
  customerColumns,
  customerCsv,
  customerFooter,
  itemColumns,
  itemCsv,
  itemFooter,
  repColumns,
  repCsv,
  repFooter,
  type CustomerX,
  type ItemX,
  type RepX,
} from "./columns";

type Status = "loading" | "error" | "empty" | "ready";

function TabPanel({
  status,
  count,
  onRetry,
  onExport,
  children,
}: {
  status: Status;
  count: number;
  onRetry: () => void;
  onExport: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div className="nameplate text-2xs text-text-dim" data-testid="row-count">
          {status === "ready" || status === "empty" ? `${count} rows` : ""}
        </div>
        <button
          onClick={onExport}
          disabled={status !== "ready"}
          data-testid="export-csv"
          className="inline-flex items-center gap-1.5 rounded border border-seam px-2.5 py-1.5 text-2xs text-text-dim transition-colors hover:bg-surface-2 hover:text-text disabled:opacity-40"
        >
          <Download size={13} strokeWidth={2} />
          <span className="nameplate">Export CSV</span>
        </button>
      </div>
      {status === "loading" && <LoadingPanel label="Loading table" />}
      {status === "error" && <ErrorPanel onRetry={onRetry} />}
      {status === "empty" && (
        <EmptyPanel message="No rows in scope for this period." />
      )}
      {status === "ready" && children}
    </div>
  );
}

function statusOf(
  isPending: boolean,
  isError: boolean,
  rowCount: number,
): Status {
  if (isPending) return "loading";
  if (isError) return "error";
  if (rowCount === 0) return "empty";
  return "ready";
}

function RepsTab({
  period,
  basis,
  kind,
}: {
  period: string;
  basis: Basis;
  kind: Kind;
}) {
  const query = useLeaderboard(period, kind);
  useScreenReady(query.isSuccess || query.isError);
  const data: RepX[] = ranked(
    query.data?.items ?? [],
    chain(
      descBasis<LeaderboardRow>(basis),
      ascNumNullsLast((r) => r.leakage_pct),
      ascStr((r) => r.rep_name),
    ),
  ).map(({ rank, row }) => ({ ...row, _rank: rank }));

  return (
    <TabPanel
      status={statusOf(query.isPending, query.isError, data.length)}
      count={data.length}
      onRetry={() => query.refetch()}
      onExport={() =>
        downloadCsv(csvFilename("reps", period, basis, kind), repCsv, data)
      }
    >
      <DataTable columns={repColumns(basis)} data={data} footer={repFooter(data, basis)} />
    </TabPanel>
  );
}

function CustomersTab({
  period,
  basis,
  kind,
}: {
  period: string;
  basis: Basis;
  kind: Kind;
}) {
  const query = useCustomers(period, kind);
  useScreenReady(query.isSuccess || query.isError);
  const data: CustomerX[] = ranked(
    query.data?.items ?? [],
    chain(descBasis<CustomerRow>(basis), ascStr((r) => r.account_name)),
  ).map(({ rank, row }) => ({ ...row, _rank: rank }));

  return (
    <TabPanel
      status={statusOf(query.isPending, query.isError, data.length)}
      count={data.length}
      onRetry={() => query.refetch()}
      onExport={() =>
        downloadCsv(
          csvFilename("customers", period, basis, kind),
          customerCsv,
          data,
        )
      }
    >
      <DataTable
        columns={customerColumns(basis)}
        data={data}
        footer={customerFooter(data, basis)}
      />
    </TabPanel>
  );
}

function ItemsTab({
  period,
  basis,
  kind,
  group,
}: {
  period: string;
  basis: Basis;
  kind: Kind;
  group: Group;
}) {
  const query = useItems(period, kind, group);
  useScreenReady(query.isSuccess || query.isError);
  const data: ItemX[] = ranked(
    query.data?.items ?? [],
    chain(
      descBasis<ItemRow>(basis),
      group === "family"
        ? ascStr((r) => r.family)
        : ascStr((r) => r.product_sku ?? ""),
    ),
  ).map(({ rank, row }) => ({ ...row, _rank: rank }));

  return (
    <TabPanel
      status={statusOf(query.isPending, query.isError, data.length)}
      count={data.length}
      onRetry={() => query.refetch()}
      onExport={() =>
        downloadCsv(
          csvFilename("items", period, basis, kind, group),
          itemCsv,
          data,
        )
      }
    >
      <DataTable
        columns={itemColumns(basis, group)}
        data={data}
        footer={itemFooter(data, group)}
      />
    </TabPanel>
  );
}

export function Leaderboards() {
  const [params, setParams] = useSearchParams();
  const state: ControlsState = {
    tab: parseTab(params.get("tab")),
    period: parsePeriod(params.get("period"), "2026"),
    basis: parseBasis(params.get("basis")),
    kind: parseKind(params.get("kind")),
    group: parseGroup(params.get("group")),
  };

  const patch = (p: Partial<ControlsState>) => {
    const sp = new URLSearchParams(params);
    for (const [k, v] of Object.entries(p)) sp.set(k, String(v));
    setParams(sp, { replace: true });
  };

  return (
    <div className="mx-auto max-w-[1600px] space-y-4">
      <header>
        <h1 className="nameplate-strong text-xl text-text">Leaderboards</h1>
        <div className="nameplate text-2xs text-text-dim">
          Ranked by chosen basis · server order held in the rank column
        </div>
      </header>

      <Controls state={state} onChange={patch} />

      {state.tab === "reps" && (
        <RepsTab period={state.period} basis={state.basis} kind={state.kind} />
      )}
      {state.tab === "items" && (
        <ItemsTab
          period={state.period}
          basis={state.basis}
          kind={state.kind}
          group={state.group}
        />
      )}
      {state.tab === "customers" && (
        <CustomersTab
          period={state.period}
          basis={state.basis}
          kind={state.kind}
        />
      )}
    </div>
  );
}
