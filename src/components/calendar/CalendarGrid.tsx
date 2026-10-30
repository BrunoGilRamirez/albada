import { Link } from "react-router";
import { useTranslation } from "react-i18next";
import { formatDate, formatNumber } from "@/lib/format";

// Localized grid header: Sunday-first short weekday names rendered through the
// shared Intl formatter (a fixed reference Sunday keeps the mapping explicit).
const REFERENCE_SUNDAY = new Date(2000, 0, 2);

function weekdayShort(index: number): string {
  const date = new Date(
    REFERENCE_SUNDAY.getFullYear(),
    REFERENCE_SUNDAY.getMonth(),
    REFERENCE_SUNDAY.getDate() + index,
  );
  return formatDate(date, { weekday: "short" });
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

function isoDate(year: number, month: number, day: number): string {
  return `${year}-${pad(month + 1)}-${pad(day)}`;
}

export interface CalendarDayInfo {
  date: string;
  hasPlan: boolean;
  hasBlocks: boolean;
  hasDeadline: boolean;
  isToday: boolean;
  isCurrentMonth: boolean;
}

interface CalendarGridProps {
  year: number;
  month: number;
  mode: "month" | "week";
  today: string;
  planDates: Set<string>;
  blockDates: Set<string>;
  deadlineDates: Set<string>;
  onSelectDate: (date: string) => void;
}

function CalendarGrid({
  year,
  month,
  mode,
  today,
  planDates,
  blockDates,
  deadlineDates,
  onSelectDate,
}: CalendarGridProps) {
  const { t } = useTranslation();
  const firstWeekday = new Date(year, month, 1).getDay();
  const daysInMonth = new Date(year, month + 1, 0).getDate();

  const weekdayLabels = Array.from({ length: 7 }, (_, index) => weekdayShort(index));

  const infoFor = (day: number, currentMonth: boolean): CalendarDayInfo => {
    const date = isoDate(year, month, day);
    return {
      date,
      hasPlan: planDates.has(date),
      hasBlocks: blockDates.has(date),
      hasDeadline: deadlineDates.has(date),
      isToday: date === today,
      isCurrentMonth: currentMonth,
    };
  };

  const dayCell = (info: CalendarDayInfo) => (
    <DayCell
      key={info.date}
      info={info}
      onSelectDate={onSelectDate}
      dayLabel={formatNumber(Number(info.date.slice(8)))}
      dotLabels={{
        plan: t("calendar.dots.plan"),
        blocks: t("calendar.dots.blocks"),
        deadline: t("calendar.dots.deadline"),
      }}
    />
  );

  if (mode === "week") {
    const monday = new Date(year, month, 1);
    const dayOfWeek = (monday.getDay() + 6) % 7;
    monday.setDate(monday.getDate() - dayOfWeek);
    return (
      <div className="flex h-full flex-col">
        <div className="grid grid-cols-7 gap-2">
          {weekdayLabels.map((label, index) => (
            <div key={index} className="joi-text--muted text-center text-xs font-medium">
              {label}
            </div>
          ))}
        </div>
        <div className="mt-2 grid min-h-0 flex-1 grid-cols-7 auto-rows-fr gap-2">
          {Array.from({ length: 7 }, (_, index) => {
            const day = monday.getDate() + index;
            const cellDate = isoDate(monday.getFullYear(), monday.getMonth(), day);
            const inCurrentMonth = monday.getMonth() === month;
            return dayCell({
              date: cellDate,
              hasPlan: planDates.has(cellDate),
              hasBlocks: blockDates.has(cellDate),
              hasDeadline: deadlineDates.has(cellDate),
              isToday: cellDate === today,
              isCurrentMonth: inCurrentMonth,
            });
          })}
        </div>
      </div>
    );
  }

  const leadingBlanks = Array.from({ length: firstWeekday }, (_, index) => index);
  const days = Array.from({ length: daysInMonth }, (_, index) => index + 1);
  const cells: (number | null)[] = [...leadingBlanks.map(() => null), ...days];

  return (
    <div className="flex h-full flex-col">
      <div className="grid grid-cols-7 gap-2">
        {weekdayLabels.map((label, index) => (
          <div key={index} className="joi-text--muted text-center text-xs font-medium">
            {label}
          </div>
        ))}
      </div>
      <div className="mt-2 grid min-h-0 flex-1 grid-cols-7 auto-rows-fr gap-2">
        {cells.map((day, index) => {
          if (!day) {
            return <div key={`blank-${index}`} />;
          }
          return dayCell(infoFor(day, true));
        })}
      </div>
    </div>
  );
}

interface DayCellProps {
  info: CalendarDayInfo;
  onSelectDate: (date: string) => void;
  dayLabel: string;
  dotLabels: { plan: string; blocks: string; deadline: string };
}

function DayCell({ info, onSelectDate, dayLabel, dotLabels }: DayCellProps) {
  const dots: { key: keyof DayCellProps["dotLabels"]; className: string; label: string }[] = [];
  if (info.hasPlan) {
    dots.push({ key: "plan", className: "joi-dot joi-dot--info", label: dotLabels.plan });
  }
  if (info.hasBlocks) {
    dots.push({ key: "blocks", className: "joi-dot joi-dot--warning", label: dotLabels.blocks });
  }
  if (info.hasDeadline) {
    dots.push({ key: "deadline", className: "joi-dot joi-dot--danger", label: dotLabels.deadline });
  }

  return (
    <Link
      to={`/?date=${info.date}`}
      onClick={() => onSelectDate(info.date)}
      className={`joi-interactive flex min-h-24 flex-col gap-1 rounded-lg border p-2 text-left transition-colors ${
        info.isToday
          ? "joi-surface--raised joi-border--strong"
          : info.isCurrentMonth
            ? "joi-surface--base joi-border--default"
            : "joi-surface--sunken joi-border--default opacity-50"
      }`}
    >
      <span className={`text-xs font-medium ${info.isToday ? "joi-text" : "joi-text--muted"}`}>
        {dayLabel}
      </span>
      <div className="flex gap-1">
        {dots.map((dot) => (
          <span key={dot.key} title={dot.label} className={dot.className} />
        ))}
      </div>
    </Link>
  );
}

export default CalendarGrid;
