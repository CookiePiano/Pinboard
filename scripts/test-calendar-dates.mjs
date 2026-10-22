import assert from "node:assert/strict";
import { countDays, dateKey, dayRange, monthDays } from "../src/lib/calendar/dates.ts";

const originalZone = process.env.TZ;
try {
  process.env.TZ = "Asia/Seoul";
  assert.equal(dateKey(new Date("2026-10-20T15:00:00Z")), "2026-10-21");
  const localDay = dayRange(new Date("2026-10-21T00:30:00+09:00"));
  assert.equal(localDay.start, Date.parse("2026-10-20T15:00:00Z"));
  assert.equal(localDay.end, Date.parse("2026-10-21T15:00:00Z"));
  const counts = countDays([
    { id: "a", createdAt: Date.parse("2026-10-20T15:01:00Z") },
    { id: "b", createdAt: Date.parse("2026-10-21T14:59:59Z") },
    { id: "c", createdAt: Date.parse("2026-10-21T15:00:00Z") },
    { id: "welcome", createdAt: 0 }, { id: "invalid", createdAt: NaN },
  ]);
  assert.equal(counts.get("2026-10-21"), 2);
  assert.equal(counts.get("2026-10-22"), 1);
  assert.equal(counts.size, 2);
  const october = monthDays(new Date(2026, 9, 1));
  assert.equal(october.length, 42);
  assert.equal(october[0].getDay(), 0);
  assert.equal(dateKey(october[0]), "2026-09-27");
  assert.equal(dateKey(october.at(-1)), "2026-11-07");
  assert.equal(monthDays(new Date(2028, 1, 1)).filter(d => d.getMonth() === 1).length, 29);

  process.env.TZ = "America/New_York";
  assert.equal(dateKey(new Date("2026-10-20T15:00:00Z")), "2026-10-20");
  const spring = dayRange(new Date(2026, 2, 8, 12));
  const autumn = dayRange(new Date(2026, 10, 1, 12));
  assert.equal(spring.end - spring.start, 23 * 60 * 60 * 1000);
  assert.equal(autumn.end - autumn.start, 25 * 60 * 60 * 1000);
  console.log("Calendar date checks passed (KST, time zone changes, DST, leap year, month boundaries).");
} finally {
  if (originalZone === undefined) delete process.env.TZ;
  else process.env.TZ = originalZone;
}
