import { CircleCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNow } from "@/lib/use-now";
import { RowGroupLabel, SectionRows } from "@/ui/section";
import { EmptyState, LoadingRows } from "@/ui/states";
import { RunRow } from "../run-row";
import { isActiveTask, useTaskFeed } from "../use-task-feed";

const RECENT_MS = 24 * 3600_000;

/** 正在进行: running work, then what finished most recently. */
export default function ActiveTasksWidget() {
  const { t } = useTranslation();
  const { tasks, isPending } = useTaskFeed();
  const now = useNow(60_000);
  if (isPending) return <LoadingRows rows={3} className="px-5 pb-4" />;
  const active = tasks.filter(isActiveTask);
  const recent = tasks
    .filter((task) => task.state === "succeeded" && task.finishedAt && now - Date.parse(task.finishedAt) < RECENT_MS)
    .slice(0, 1);
  if (active.length === 0 && recent.length === 0) return <EmptyState compact icon={CircleCheck} title={t("widgets.noTasks")} detail={t("widgets.noTasksDetail")} />;
  return (
    <div>
      <SectionRows>
        {active.map((task) => (
          <li key={task.id}>
            <RunRow task={task} />
          </li>
        ))}
      </SectionRows>
      {recent.length > 0 ? (
        <>
          <RowGroupLabel className="border-t border-line">{t("widgets.recentlyFinished")}</RowGroupLabel>
          <SectionRows className="border-t-0">
            {recent.map((task) => (
              <li key={task.id}>
                <RunRow task={task} />
              </li>
            ))}
          </SectionRows>
        </>
      ) : null}
    </div>
  );
}
