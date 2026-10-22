//! Terminal descriptions of cached harness capacity. Reads never sample the host.
use ejm_harness::ejm_harness_exec::RestrictionState;
use ejm_harness::{CapacityMode, CapacitySnapshot};

pub fn mode(mode: CapacityMode) -> &'static str {
    match mode {
        CapacityMode::Auto => "auto",
        CapacityMode::Fixed => "fixed",
    }
}

pub fn describe(snapshot: &CapacitySnapshot) -> String {
    let target = snapshot.operating.target;
    let memory_mib = target.memory_bytes / (1024 * 1024);
    let ceiling_mib = snapshot.hard_resources.memory_bytes / (1024 * 1024);
    let native = &snapshot.native;
    let owner = native
        .owner_pid
        .map_or_else(|| "none cached".into(), |pid| pid.to_string());
    let last_dead_epoch = native
        .last_dead_epoch
        .map_or_else(|| "none cached".into(), |epoch| epoch.to_string());
    format!(
        "Capacity: {}\nScope: {}\nCoordination: {}\nModels: target {}, ceiling {}; {} active, {} queued\nTools: target {}, ceiling {}\nAdmitted memory: target {} MiB, ceiling {} MiB\nWait reason: {}\nProvider: target {}; {} active, {} queued; wait reason {}\nObservation age: {} ms; CPU {:?}, memory {:?}, ancestry {:?}\nLearning: {:?} ({} samples)\nNative owner lock: {}; loss detection: {}; domain placement: {}\nNative cleanup: descendants {}; owner death {}; recovery {}\nObserved owner deaths: {}; last dead epoch: {}\nLast authenticated owner PID: {}; local lifetime links: {} (cached; liveness not checked)",
        mode(snapshot.mode),
        snapshot.scope,
        if snapshot.coordinated && snapshot.operating.contact_valid {
            "available"
        } else {
            "unavailable; new admission may wait"
        },
        snapshot.model_target,
        snapshot.hard_model_requests,
        snapshot.model_running,
        snapshot.model_waiting,
        target.max_jobs,
        snapshot.hard_resources.max_jobs,
        memory_mib,
        ceiling_mib,
        snapshot.reason.as_deref().unwrap_or("none reported"),
        snapshot.provider.target,
        snapshot.provider.in_flight,
        snapshot.provider.waiting,
        snapshot
            .provider
            .reason
            .as_deref()
            .unwrap_or("none reported"),
        snapshot.observations.sampled_at.elapsed().as_millis(),
        snapshot.observations.cpu_usage_percent.availability,
        snapshot.observations.available_memory_bytes.availability,
        snapshot.observations.domain_visibility,
        snapshot.learning.state,
        snapshot.learning.samples,
        restriction(native.owner_exclusivity),
        restriction(native.owner_loss_detection),
        restriction(native.domain_placement),
        restriction(native.descendant_cleanup),
        restriction(native.owner_death_cleanup),
        restriction(native.recovery),
        native.owner_deaths,
        last_dead_epoch,
        owner,
        native.active_local_scopes,
    )
}

fn restriction(state: RestrictionState) -> &'static str {
    match state {
        RestrictionState::Enforced => "enforced",
        RestrictionState::Monitored => "monitored",
        RestrictionState::Unsupported => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use ejm_harness::{LaneSnapshot, LearningStatus, NativeHelperSnapshot, ProviderSnapshot};
    use ejm_harness_exec::{
        AdmissionPolicy, HostObservations, Observation, ObservationAvailability, ObservationSource,
        OperatingSnapshot, OperatingTarget, ResourceLimits, native::NativeSnapshot,
    };

    use super::*;

    fn unavailable<T>() -> Observation<T> {
        Observation {
            value: None,
            source: ObservationSource::NativeSystem,
            availability: ObservationAvailability::Unavailable,
            age_ms: 0,
            max_age_ms: 5000,
        }
    }

    fn snapshot() -> CapacitySnapshot {
        let hard = ResourceLimits::default();
        let mut target = OperatingTarget::from_limits(&hard);
        target.max_jobs = 1;
        target.memory_bytes = 128 * 1024 * 1024;
        CapacitySnapshot {
            mode: CapacityMode::Auto,
            scope: "fixture host".into(),
            coordinated: true,
            hard_model_requests: 8,
            model_target: 2,
            model_running: 1,
            model_waiting: 3,
            hard_resources: hard,
            operating: OperatingSnapshot {
                policy: AdmissionPolicy::Automatic,
                generation: 1,
                target,
                debt: OperatingTarget::default(),
                memory_leases: 0,
                contact_valid: true,
                contact_remaining: None,
            },
            observations: HostObservations {
                sampled_at: Instant::now(),
                cpu_millis: unavailable(),
                cpu_usage_percent: unavailable(),
                memory_bytes: unavailable(),
                available_memory_bytes: unavailable(),
                process_limit: unavailable(),
                descriptor_limit: unavailable(),
                domains: vec![],
                domain_visibility: ObservationAvailability::Unsupported,
            },
            reason: Some("shared host budget".into()),
            learning: LearningStatus::default(),
            models: LaneSnapshot::default(),
            tools: LaneSnapshot::default(),
            provider: ProviderSnapshot {
                target: 1,
                in_flight: 1,
                waiting: 2,
                reason: Some("provider reset pending".into()),
                ..ProviderSnapshot::default()
            },
            native: NativeSnapshot::default(),
            native_helpers: NativeHelperSnapshot::default(),
        }
    }

    #[test]
    fn status_distinguishes_targets_ceilings_and_provider_waits() {
        let snapshot = snapshot();
        let text = describe(&snapshot);
        assert!(text.contains("Models: target 2, ceiling 8; 1 active, 3 queued"));
        assert!(text.contains("Tools: target 1, ceiling 4"));
        assert!(text.contains("target 128 MiB, ceiling 512 MiB"));
        assert!(text.contains("Wait reason: shared host budget"));
        assert!(text.contains(
            "Provider: target 1; 1 active, 2 queued; wait reason provider reset pending"
        ));
    }

    #[test]
    fn status_keeps_missing_observations_and_lost_contact_explicit() {
        let mut snapshot = snapshot();
        snapshot.operating.contact_valid = false;
        snapshot.native.descendant_cleanup = RestrictionState::Monitored;
        snapshot.native.owner_death_cleanup = RestrictionState::Unsupported;
        let text = describe(&snapshot);
        assert!(text.contains("Coordination: unavailable"));
        assert!(text.contains("CPU Unavailable, memory Unavailable, ancestry Unsupported"));
        assert!(text.contains("Learning: Cold (0 samples)"));
        assert!(text.contains("descendants monitored; owner death unsupported"));
    }
}
