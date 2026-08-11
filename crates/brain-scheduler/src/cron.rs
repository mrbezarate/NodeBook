//! Cron-планировщик.
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    Daily { hour: u32, minute: u32 },
    Weekly { day: chrono::Weekday, hour: u32, minute: u32 },
    Interval { seconds: u64 },
    Once { at: DateTime<Utc> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType { EveningReview, WeeklyReport, BackupVault, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String, pub name: String, pub schedule: Schedule,
    pub next_run: DateTime<Utc>, pub job_type: JobType,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CronScheduler { pub jobs: Vec<ScheduledJob> }

impl CronScheduler {
    pub fn new() -> Self { Self::default() }
    pub fn add_job(&mut self, job: ScheduledJob) { self.jobs.push(job); }

    pub fn check_due(&self) -> Vec<&ScheduledJob> {
        let now = Utc::now();
        self.jobs.iter().filter(|j| j.next_run <= now).collect()
    }
}
