export interface TimeService {
  formatRelativeTime(timestamp: number): string;
  formatShortRelativeTime(dateStr: string): string;
}

interface TimeUnit {
  seconds: number;
  singular: string;
  plural: string;
  short: string;
}

const TIME_UNITS: TimeUnit[] = [
  {
    seconds: 365 * 24 * 60 * 60,
    singular: "year",
    plural: "years",
    short: "y",
  },
  {
    seconds: 30 * 24 * 60 * 60,
    singular: "month",
    plural: "months",
    short: "mo",
  },
  { seconds: 7 * 24 * 60 * 60, singular: "week", plural: "weeks", short: "w" },
  { seconds: 24 * 60 * 60, singular: "day", plural: "days", short: "d" },
  { seconds: 60 * 60, singular: "hour", plural: "hours", short: "h" },
  { seconds: 60, singular: "min", plural: "mins", short: "m" },
];

export const timeService: TimeService = {
  formatRelativeTime(timestamp: number): string {
    const now = Date.now() / 1000;
    const diff = now - timestamp;

    if (diff < 60) {
      return "just now";
    }

    for (const unit of TIME_UNITS) {
      if (diff >= unit.seconds) {
        const count = Math.floor(diff / unit.seconds);
        return `${count} ${count === 1 ? unit.singular : unit.plural} ago`;
      }
    }

    return "just now";
  },

  formatShortRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSecs = Math.floor(diffMs / 1000);

    if (diffSecs < 60) {
      return "just now";
    }

    for (const unit of TIME_UNITS) {
      if (diffSecs >= unit.seconds) {
        const count = Math.floor(diffSecs / unit.seconds);
        // For months and beyond, use date format
        if (unit.short === "mo" || unit.short === "y") {
          return date.toLocaleDateString("en-US", {
            month: "short",
            day: "numeric",
          });
        }
        return `${count}${unit.short} ago`;
      }
    }

    return "just now";
  },
};
