use super::*;

impl WorldState {
    pub fn current_time_label(&self) -> String {
        format_clock_time(self.current_time_minutes)
    }

    pub fn current_day_number(&self) -> u32 {
        (self.current_time_minutes / MINUTES_PER_DAY) + 1
    }

    pub fn current_time_note(&self) -> String {
        format!("It is {}.", self.current_time_label())
    }
}

fn format_clock_time(total_minutes: u32) -> String {
    let hour24 = (total_minutes / 60) % 24;
    let minute = total_minutes % 60;
    let meridiem = if hour24 >= 12 { "PM" } else { "AM" };
    let hour12 = match hour24 % 12 {
        0 => 12,
        hour => hour,
    };
    format!("{hour12}:{minute:02} {meridiem}")
}
