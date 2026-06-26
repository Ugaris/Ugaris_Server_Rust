use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::tick::TICKS_PER_SECOND;

pub const TIMER_CHUNK_SIZE: usize = 16_384;
pub const MAX_SCHEDULED_TASKS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerPayload(pub [i32; 5]);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerEvent {
    pub due: u64,
    pub name: String,
    pub payload: TimerPayload,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TimerQueue {
    timers: Vec<TimerEvent>,
}

impl TimerQueue {
    pub fn set_timer(&mut self, due: u64, name: impl Into<String>, payload: TimerPayload) -> bool {
        let event = TimerEvent {
            due,
            name: name.into(),
            payload,
        };
        let pos = self.timers.partition_point(|timer| timer.due < due);
        self.timers.insert(pos, event);
        true
    }

    pub fn tick(&mut self, current_tick: u64) -> Vec<TimerEvent> {
        let due_count = self
            .timers
            .partition_point(|timer| timer.due <= current_tick);
        self.timers.drain(..due_count).collect()
    }

    pub fn used_timers(&self) -> usize {
        self.timers.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub name: String,
    pub interval_ticks: u64,
    pub last_run_tick: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TaskScheduler {
    tasks: Vec<ScheduledTask>,
}

impl TaskScheduler {
    pub fn add_task(
        &mut self,
        current_tick: u64,
        interval_seconds: u64,
        name: impl Into<String>,
        run_immediately: bool,
    ) -> bool {
        if self.tasks.len() >= MAX_SCHEDULED_TASKS {
            return false;
        }
        self.tasks.push(ScheduledTask {
            name: name.into(),
            interval_ticks: interval_seconds * TICKS_PER_SECOND,
            last_run_tick: if run_immediately { 0 } else { current_tick },
        });
        true
    }

    pub fn due_tasks(&mut self, current_tick: u64) -> VecDeque<String> {
        let mut due = VecDeque::new();
        for task in &mut self.tasks {
            if current_tick.saturating_sub(task.last_run_tick) >= task.interval_ticks {
                due.push_back(task.name.clone());
                task.last_run_tick = current_tick;
            }
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_queue_fires_sorted_due_events() {
        let mut timers = TimerQueue::default();
        timers.set_timer(10, "late", TimerPayload([0; 5]));
        timers.set_timer(5, "early", TimerPayload([1; 5]));
        assert_eq!(timers.used_timers(), 2);
        let due = timers.tick(5);
        assert_eq!(due[0].name, "early");
        assert_eq!(timers.used_timers(), 1);
    }

    #[test]
    fn scheduled_tasks_convert_seconds_to_ticks() {
        let mut scheduler = TaskScheduler::default();
        assert!(scheduler.add_task(100, 5, "five_seconds", false));
        assert!(scheduler
            .due_tasks(100 + 5 * TICKS_PER_SECOND - 1)
            .is_empty());
        assert_eq!(
            scheduler
                .due_tasks(100 + 5 * TICKS_PER_SECOND)
                .pop_front()
                .as_deref(),
            Some("five_seconds")
        );
    }
}
