/**
 * ISO weekday (1 = Monday ... 7 = Sunday) mapped onto the shared
 * `common.labels.weekday` catalog keys. Recurrence and reactivation use the
 * ISO range; the availability profile keeps its own 0..6 range, so the two
 * stay distinct.
 */
export const ISO_WEEKDAY_KEYS = [
  "monday",
  "tuesday",
  "wednesday",
  "thursday",
  "friday",
  "saturday",
  "sunday",
] as const;

/** Catalog key for an ISO weekday (1..7). */
export function isoWeekdayKey(iso: number): (typeof ISO_WEEKDAY_KEYS)[number] {
  const index = iso - 1;
  return ISO_WEEKDAY_KEYS[index] ?? ISO_WEEKDAY_KEYS[0];
}

/**
 * ISO weekday of a local `Date` (Sunday is 7, Monday is 1). The value is the
 * active local weekday of the browser clock.
 */
export function localIsoWeekday(now: Date): number {
  const day = now.getDay();
  return day === 0 ? 7 : day;
}

/**
 * ISO weekday of the local day that follows `now`'s weekday. Used as the
 * reactivation default after 23:00 local time, when the current day is
 * already over.
 */
export function nextIsoWeekday(now: Date): number {
  const iso = localIsoWeekday(now);
  return iso === 7 ? 1 : iso + 1;
}
