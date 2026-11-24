/**
 * 任务: every piece of background work in one list — on an M0 server the jobs API, with the
 * provisional "tasks" feature also downloads, transcodes, AI work, assistant runs and
 * automations. What waits for the user comes first and is marked the way the skin marks
 * "needs you"; running work follows with its progress; finished and failed work last.
 */
import { Button, Label, ListBox, Select, Spinner, ToggleButton, ToggleButtonGroup, toast } from "@heroui/react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Check, Eye, Info, ListChecks, Lock, SearchX } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import { useEventStreamStatus } from "@/api/events";
import { useJobs } from "@/api/queries/jobs";
import { RelativeTime } from "@/components/relative-time";
import { errorText } from "@/lib/errors";
import { useApproveTask, useProvisionalTasks } from "@/provisional/queries/tasks";
import type { Task } from "@/provisional/types";
import { useSecureSurface } from "@/skins/secure";
import { useSlot } from "@/skins/scope";
import { CountMark } from "@/ui/count-mark";
import { PageFrame } from "@/ui/frame";
import { AttentionMark } from "@/ui/marks";
import { PageHeader } from "@/ui/page";
import { Section, SectionRows, useRowInset, useSectionVariant } from "@/ui/section";
import { EmptyState, ErrorState, LoadingRows } from "@/ui/states";
import { RunRow } from "./run-row";
import { groupOf, matchesFilter, TASK_FILTERS, type TaskFilter, type TaskGroup } from "./task-filters";
import { TaskDrawer } from "./task-drawer";
import { TASK_KINDS, useTaskLabels } from "./task-kinds";
import { defaultSelection } from "./use-task-inbox";
import { isActiveTask, useTaskFeed, type TaskSource } from "./use-task-feed";

const ALL_KINDS = "all";

/** Kinds present in the list, known kinds first in their usual order. */
function kindsOf(tasks: Task[]): string[] {
  const present = new Set(tasks.map((task) => task.kind));
  const known: string[] = TASK_KINDS.filter((kind) => present.has(kind));
  return [...known, ...[...present].filter((kind) => !known.includes(kind)).sort()];
}

/** Whether confirming straight from the list is safe: high-risk requests are reviewed first. */
function confirmableInList(task: Task): boolean {
  return !task.result.some((block) => block.type === "approval" && block.risk === "high");
}

/* ---------------------------------------------------------------- rows */

function WaitingRow({ task, onOpen }: { task: Task; onOpen: () => void }) {
  const { t } = useTranslation();
  const { title } = useTaskLabels();
  const attention = useSlot("attention");
  const inset = useRowInset();
  const variant = useSectionVariant();
  const secure = useSecureSurface();
  const approve = useApproveTask();
  const tint = attention === "tint";
  const name = title(task);
  const confirm = () =>
    approve.mutate(
      { id: task.id, selectedRowIds: defaultSelection(task) },
      { onSuccess: () => toast.success(t("tasks.approved")), onError: (error) => toast.danger(errorText(t, error)) },
    );
  return (
    <li
      {...secure}
      className={cn(
        "flex flex-wrap items-center gap-x-4 gap-y-3 py-3.5 sm:flex-nowrap",
        tint ? "rounded-tile border border-attention-line bg-attention-soft px-4" : inset,
        variant === "plain" && !tint && "rounded-tile",
      )}
    >
      <div className="flex min-w-0 flex-1 items-start gap-3.5">
        <AttentionMark previews={task.previews} title={name} />
        <div className="grid min-w-0 flex-1 gap-0.5">
          <h3 className="text-md font-mid text-fg">
            <button type="button" onClick={onOpen} className="text-start hover:underline">
              {name}
            </button>
          </h3>
          <p className="text-sm text-fg-muted">
            <span className="font-mid text-attention">{t("attention.waiting")}</span>
            <span aria-hidden> · </span>
            <RelativeTime value={task.createdAt} />
            {task.scope ? (
              <>
                <span aria-hidden> · </span>
                {task.scope}
              </>
            ) : null}
          </p>
          {task.message ? <p className="text-sm text-fg-muted">{task.message}</p> : null}
        </div>
      </div>
      <div className="flex shrink-0 flex-wrap items-center gap-1.5 max-sm:w-full max-sm:justify-end">
        <Button size="sm" variant="outline" onPress={onOpen}>
          <Eye aria-hidden />
          {t("attention.review")}
        </Button>
        {confirmableInList(task) ? (
          <Button size="sm" variant="primary" className="button--confirm" onPress={confirm} isPending={approve.isPending}>
            {approve.isPending ? <Spinner size="sm" color="current" /> : <Check aria-hidden />}
            {t("attention.confirm")}
          </Button>
        ) : null}
      </div>
    </li>
  );
}

function WaitingList({ tasks, onOpen }: { tasks: Task[]; onOpen: (id: string) => void }) {
  const attention = useSlot("attention");
  const variant = useSectionVariant();
  return (
    <ul className={cn(attention === "tint" ? "grid gap-2" : "divide-y divide-line", variant === "panel" && attention !== "tint" && "border-t border-line", variant === "panel" && attention === "tint" && "px-3 pb-3")}>
      {tasks.map((task) => (
        <WaitingRow key={task.id} task={task} onOpen={() => onOpen(task.id)} />
      ))}
    </ul>
  );
}

function TaskRows({ tasks, onOpen }: { tasks: Task[]; onOpen: (id: string) => void }) {
  return (
    <SectionRows>
      {tasks.map((task) => (
        <li key={task.id} className="transition-colors hover:bg-hover">
          <RunRow task={task} onOpen={() => onOpen(task.id)} detailed />
        </li>
      ))}
    </SectionRows>
  );
}

/* ---------------------------------------------------------------- filters */

function Filters({
  filter,
  counts,
  kind,
  kinds,
  source,
  onFilter,
  onKind,
}: {
  filter: TaskFilter;
  counts: Record<TaskFilter, number>;
  kind: string | undefined;
  kinds: string[];
  source: TaskSource;
  onFilter: (filter: TaskFilter) => void;
  onKind: (kind: string | undefined) => void;
}) {
  const { t } = useTranslation();
  const { kindLabel } = useTaskLabels();
  // Plain M0 jobs never wait for confirmation; the filter would only ever be empty.
  const filters = source === "jobs" ? TASK_FILTERS.filter((value) => value !== "awaiting") : TASK_FILTERS;
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
      <div className="no-scrollbar -mx-4 max-w-[calc(100%+2rem)] overflow-x-auto px-4 md:mx-0 md:max-w-full md:px-0">
        <ToggleButtonGroup
          aria-label={t("tasks.filterLabel")}
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={[filter]}
          onSelectionChange={(keys) => onFilter(([...keys][0] as TaskFilter | undefined) ?? "all")}
          className="pv-segmented"
        >
          {filters.map((value) => (
            <ToggleButton key={value} id={value} className="shrink-0 whitespace-nowrap">
              {t(`tasks.filters.${value}`)}
              {value === "awaiting" && counts.awaiting > 0 ? (
                <CountMark count={counts.awaiting} />
              ) : (
                <span className="text-fg-muted tabular-nums">{counts[value]}</span>
              )}
            </ToggleButton>
          ))}
        </ToggleButtonGroup>
      </div>
      {kinds.length > 1 ? (
        <Select
          aria-label={t("tasks.kindLabel")}
          value={kind ?? ALL_KINDS}
          onChange={(key) => onKind(key === null || key === ALL_KINDS ? undefined : String(key))}
          className="w-full sm:ms-auto sm:w-48"
        >
          <Label className="sr-only">{t("tasks.kindLabel")}</Label>
          <Select.Trigger>
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover>
            <ListBox>
              {[ALL_KINDS, ...kinds].map((value) => {
                const text = value === ALL_KINDS ? t("tasks.allKinds") : kindLabel(value);
                return (
                  <ListBox.Item key={value} id={value} textValue={text}>
                    {text}
                    <ListBox.ItemIndicator />
                  </ListBox.Item>
                );
              })}
            </ListBox>
          </Select.Popover>
        </Select>
      ) : null}
    </div>
  );
}

/* ---------------------------------------------------------------- page */

const GROUP_TITLES = {
  awaiting: "tasks.groups.awaiting",
  active: "tasks.groups.active",
  finished: "tasks.groups.finished",
} as const satisfies Record<TaskGroup, string>;

export function TasksPage() {
  const { t } = useTranslation();
  const attention = useSlot("attention");
  const navigate = useNavigate({ from: "/tasks/" });
  const search = useSearch({ from: "/_app/tasks/" });
  const { tasks, source, isPending } = useTaskFeed();
  const live = useEventStreamStatus() === "open";
  const activeCount = tasks.filter(isActiveTask).length;
  // Without the event stream, poll the jobs API while something is running.
  const jobs = useJobs({ enabled: source === "jobs", refetchInterval: !live && activeCount > 0 ? 3000 : false });
  const rich = useProvisionalTasks(source === "tasks");
  const feed = source === "jobs" ? jobs : source === "tasks" ? rich : null;

  const filter: TaskFilter = search.filter ?? "all";
  const kind = search.kind;
  const setSearch = (next: Partial<typeof search>) => void navigate({ search: (current) => ({ ...current, ...next }), replace: true });
  const open = (id: string) => setSearch({ task: id });

  const kinds = kindsOf(tasks);
  const ofKind = kind ? tasks.filter((task) => task.kind === kind) : tasks;
  const counts = Object.fromEntries(TASK_FILTERS.map((value) => [value, ofKind.filter((task) => matchesFilter(task, value)).length])) as Record<TaskFilter, number>;
  const visible = ofKind.filter((task) => matchesFilter(task, filter));
  const groups: Record<TaskGroup, Task[]> = { awaiting: [], active: [], finished: [] };
  for (const task of visible) groups[groupOf(task)].push(task);
  const selected = search.task ? tasks.find((task) => task.id === search.task) : undefined;

  const all = Object.fromEntries(TASK_FILTERS.map((value) => [value, tasks.filter((task) => matchesFilter(task, value)).length])) as Record<TaskFilter, number>;
  const summary =
    tasks.length === 0
      ? undefined
      : [
          all.awaiting > 0 ? t("tasks.summary.awaiting", { count: all.awaiting }) : null,
          all.active > 0 ? t("tasks.summary.active", { count: all.active }) : t("tasks.summary.idle"),
          all.failed > 0 ? t("tasks.summary.failed", { count: all.failed }) : null,
        ]
          .filter(Boolean)
          .join(t("tasks.summary.separator"));

  const titleOf = (group: TaskGroup) => (filter === "all" ? t(GROUP_TITLES[group]) : t(`tasks.filters.${filter}`));

  let body;
  if (source === "none") {
    body = <EmptyState icon={Lock} title={t("tasks.adminOnly")} detail={t("tasks.adminOnlyDetail")} />;
  } else if (isPending) {
    body = (
      <Section title={t("tasks.groups.active")} bare>
        <LoadingRows rows={5} label={t("tasks.loading")} className="px-(--pv-pad-panel) py-4" />
      </Section>
    );
  } else if (feed?.isError && tasks.length === 0) {
    body = <ErrorState error={feed.error} onRetry={() => void feed.refetch()} />;
  } else if (tasks.length === 0) {
    body = <EmptyState icon={ListChecks} title={t("tasks.empty")} detail={t("tasks.emptyDetail")} />;
  } else if (visible.length === 0) {
    body = (
      <EmptyState
        icon={SearchX}
        title={t("tasks.emptyFiltered")}
        detail={t("tasks.emptyFilteredDetail")}
        action={
          <Button size="sm" variant="outline" onPress={() => setSearch({ filter: undefined, kind: undefined })}>
            {t("tasks.clearFilters")}
          </Button>
        }
      />
    );
  } else {
    body = (
      <>
        {groups.awaiting.length > 0 ? (
          <Section title={titleOf("awaiting")} count={groups.awaiting.length} raised={attention === "lamp"}>
            <WaitingList tasks={groups.awaiting} onOpen={open} />
          </Section>
        ) : null}
        {groups.active.length > 0 ? (
          <Section title={titleOf("active")} count={groups.active.length}>
            <TaskRows tasks={groups.active} onOpen={open} />
          </Section>
        ) : null}
        {groups.finished.length > 0 ? (
          <Section title={titleOf("finished")} count={groups.finished.length}>
            <TaskRows tasks={groups.finished} onOpen={open} />
          </Section>
        ) : null}
      </>
    );
  }

  return (
    <PageFrame>
      <PageHeader title={t("modules.tasks.title")} summary={summary} />
      {source !== "none" && !isPending && tasks.length > 0 ? (
        <Filters
          filter={filter}
          counts={counts}
          kind={kind}
          kinds={kinds}
          source={source}
          onFilter={(value) => setSearch({ filter: value === "all" ? undefined : value })}
          onKind={(value) => setSearch({ kind: value })}
        />
      ) : null}
      {body}
      {source === "jobs" && !isPending ? (
        <p className="flex max-w-2xl items-start gap-2 text-sm text-fg-muted">
          <Info className="mt-0.5 size-4 shrink-0" aria-hidden />
          {t("tasks.m0Note")}
        </p>
      ) : null}
      <TaskDrawer
        open={search.task !== undefined}
        task={selected}
        loading={isPending}
        interactive={source === "tasks"}
        onClose={() => setSearch({ task: undefined })}
      />
    </PageFrame>
  );
}
