use crate::config::Config;
use windows::Win32::System::SystemInformation::GetLocalTime;

// ── Local time snapshot ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct LocalTime {
    /// 0 = Sunday … 6 = Saturday
    pub day_of_week: u16,
    pub hour: u16,
    pub minute: u16,
}

impl LocalTime {
    pub fn now() -> Self {
        unsafe {
            // In windows 0.58, GetLocalTime() returns SYSTEMTIME directly
            let st = GetLocalTime();
            Self {
                day_of_week: st.wDayOfWeek,
                hour: st.wHour,
                minute: st.wMinute,
            }
        }
    }

    fn total_minutes(&self) -> i32 {
        self.hour as i32 * 60 + self.minute as i32
    }
}

// ── Progress result ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct TimeProgress {
    /// 0.0 (not started) … 1.0 (done/weekend)
    pub daily_pct: f64,
    /// 0.0 (Monday 8 am) … 1.0 (Friday end/weekend)
    pub weekly_pct: f64,
    /// true on Saturday (6) and Sunday (0)
    pub is_weekend: bool,
    /// Minutes remaining today — reserved for future tooltip/hover display
    #[allow(dead_code)]
    pub daily_remaining_mins: Option<i32>,
    /// Minutes remaining this week — reserved for future tooltip/hover display
    #[allow(dead_code)]
    pub weekly_remaining_mins: Option<i32>,
}

// ── Core calculation ───────────────────────────────────────────────────────

pub fn calculate_progress(cfg: &Config) -> TimeProgress {
    let now = LocalTime::now();

    let is_weekend = now.day_of_week == 0 || now.day_of_week == 6;

    let start_mins = cfg.schedule.start_hour as i32 * 60 + cfg.schedule.start_minute as i32;
    let end_mins   = cfg.schedule.end_hour   as i32 * 60 + cfg.schedule.end_minute   as i32;
    let day_total  = (end_mins - start_mins).max(1); // guard div/0
    let cur_mins   = now.total_minutes();

    // ── Daily ──────────────────────────────────────────────────────────────
    let (daily_pct, daily_remaining_mins) = if is_weekend {
        (1.0, None)
    } else if cur_mins < start_mins {
        // Before work starts → show 0 %
        (0.0, Some(end_mins - cur_mins))
    } else if cur_mins >= end_mins {
        // After work ends → show 100 %
        (1.0, None)
    } else {
        let elapsed = cur_mins - start_mins;
        let pct     = elapsed as f64 / day_total as f64;
        let left    = end_mins - cur_mins;
        (pct.clamp(0.0, 1.0), Some(left))
    };

    // ── Weekly ─────────────────────────────────────────────────────────────
    //
    // Full work-week span: Monday start_time → Friday end_time
    //
    //   Total minutes = 4 * 1440 + day_total
    //     (Monday start → Tuesday start = 1440 min, × 4 days, + one work-day)
    //
    // Current offset = days_since_monday * 1440 + (cur_mins - start_mins)
    //   clamped 0 … total

    let (weekly_pct, weekly_remaining_mins) = if is_weekend {
        (1.0, None)
    } else {
        // day_of_week: 1=Mon … 5=Fri  (we already excluded 0 and 6 above)
        let days_since_monday = (now.day_of_week as i32 - 1).max(0);

        let week_total  = 4 * 1440 + day_total;
        let week_offset = days_since_monday * 1440 + (cur_mins - start_mins);
        let week_offset = week_offset.max(0);

        let pct  = (week_offset as f64 / week_total as f64).clamp(0.0, 1.0);
        let left = (week_total - week_offset).max(0);

        (pct, Some(left))
    };

    TimeProgress {
        daily_pct,
        weekly_pct,
        is_weekend,
        daily_remaining_mins,
        weekly_remaining_mins,
    }
}

// ── Formatting helpers — reserved for future tooltip display ───────────────

/// Format a minute count as "Xh Ym" (e.g. "2h 30m" or "45m")
#[allow(dead_code)]
pub fn format_mins(total_mins: i32) -> String {
    let total = total_mins.max(0);
    let h = total / 60;
    let m = total % 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else {
        format!("{m}m")
    }
}
