use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    thread::JoinHandle,
};

use crate::{
    implementation::{
        dds::dds_domain_participant::DdsDomainParticipant,
        rtps::types::GuidPrefix,
        utils::{
            condvar::DdsCondvar,
            shared_object::{DdsRwLock, DdsShared},
        },
    },
    infrastructure::{instance::InstanceHandle, time::DurationKind},
};

use super::domain_participant_factory::THE_DDS_DOMAIN_PARTICIPANT_FACTORY;

struct PeriodicTask {
    func: Box<dyn Fn(&mut DdsDomainParticipant) + Send + Sync>,
    duration: std::time::Duration,
    start_instant: std::time::Instant,
}

impl PeriodicTask {
    fn new(
        duration: std::time::Duration,
        func: impl Fn(&mut DdsDomainParticipant) + 'static + Send + Sync,
    ) -> Self {
        Self {
            func: Box::new(func),
            duration,
            start_instant: std::time::Instant::now(),
        }
    }

    fn is_elapsed(&self) -> bool {
        std::time::Instant::now() - self.start_instant > self.duration
    }

    fn reset(&mut self) {
        self.start_instant = std::time::Instant::now();
    }

    fn remaining_duration(&self) -> std::time::Duration {
        self.duration
            .saturating_sub(std::time::Instant::now() - self.start_instant)
    }
}

pub struct Timer {
    instance_task_list: HashMap<(GuidPrefix, InstanceHandle), PeriodicTask>,
    timer_condvar: DdsCondvar,
}

impl Timer {
    fn new(timer_condvar: DdsCondvar) -> Self {
        Self {
            instance_task_list: HashMap::new(),
            timer_condvar,
        }
    }

    pub fn start_timer(
        &mut self,
        duration: DurationKind,
        guid_prefix: GuidPrefix,
        id: InstanceHandle,
        func: impl Fn(&mut DdsDomainParticipant) + 'static + Send + Sync,
    ) {
        if let DurationKind::Finite(duration) = duration {
            let duration = std::time::Duration::new(duration.sec() as u64, duration.nanosec());
            self.instance_task_list
                .insert((guid_prefix, id), PeriodicTask::new(duration, func));
            self.timer_condvar.notify_all();
        }
    }

    pub fn cancel_timers(&mut self) {
        self.instance_task_list.clear();
    }
}

pub struct TimerFactory {
    timer_list: DdsShared<DdsRwLock<Vec<DdsShared<DdsRwLock<Timer>>>>>,
    timer_condvar: DdsCondvar,
    thread_handle: Option<JoinHandle<()>>,
    should_stop: DdsShared<std::sync::atomic::AtomicBool>,
}

impl Default for TimerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerFactory {
    pub fn new() -> Self {
        let timer_list = DdsShared::new(DdsRwLock::new(Vec::<DdsShared<DdsRwLock<Timer>>>::new()));
        let should_stop = DdsShared::new(AtomicBool::new(false));
        let timer_condvar = DdsCondvar::new();

        let timer_list_clone = timer_list.clone();
        let should_stop_clone = should_stop.clone();
        let timer_condvar_clone = timer_condvar.clone();

        let thread_handle = std::thread::spawn(move || loop {
            if should_stop_clone.load(Ordering::Relaxed) {
                break;
            }

            for timer in timer_list_clone.read_lock().iter() {
                for ((guid_prefix, _), v) in timer.write_lock().instance_task_list.iter_mut() {
                    if v.is_elapsed() {
                        v.reset();
                        THE_DDS_DOMAIN_PARTICIPANT_FACTORY.get_participant_mut(guid_prefix, |dp| {
                            if let Some(dp) = dp {
                                (v.func)(dp)
                            }
                        })
                    }
                }
            }

            let min_remaining_duration = timer_list_clone
                .read_lock()
                .iter()
                .map(|x| {
                    x.read_lock()
                        .instance_task_list
                        .values()
                        .map(|x| x.remaining_duration())
                        .min()
                })
                .min();

            if let Some(Some(d)) = min_remaining_duration {
                timer_condvar_clone.wait_timeout(d.into()).ok();
            } else {
                timer_condvar_clone.wait().ok();
            }
        });

        Self {
            timer_list,
            timer_condvar,
            thread_handle: Some(thread_handle),
            should_stop,
        }
    }

    pub fn create_timer(&self) -> DdsShared<DdsRwLock<Timer>> {
        let timer = DdsShared::new(DdsRwLock::new(Timer::new(self.timer_condvar.clone())));

        self.timer_list.write_lock().push(timer.clone());

        timer
    }

    pub fn _delete_timer(&self, timer: &DdsShared<DdsRwLock<Timer>>) {
        self.timer_list.write_lock().retain(|x| x != timer);
    }
}

impl Drop for TimerFactory {
    fn drop(&mut self) {
        self.should_stop.store(true, Ordering::Relaxed);
        self.timer_list.write_lock().clear();
        self.timer_condvar.notify_all();
        if let Some(t) = self.thread_handle.take() {
            t.join().ok();
        }
    }
}
