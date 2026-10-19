# Spec delta

## MODIFIED Requirements

### Requirement: Bounded free launch
The default deployment SHALL use the provider's free plan, bounded beta admission and published per-account limits. It SHALL reserve capacity for account, billing and cleanup work and fail visibly when capacity is unavailable. Paid service entitlement SHALL NOT override a hosting capacity limit.

#### Scenario: Free capacity is exhausted
- **WHEN** application admission limits or provider quotas prevent a connection
- **THEN** the client reports temporary relay capacity exhaustion and the host session continues locally

#### Scenario: Operator considers upgrading
- **WHEN** hosting guidance describes a paid upgrade
- **THEN** it separates the provider's base charge and usage charges from payment fees and makes no promise of unlimited free hosting or an absolute application spending cap


#### Scenario: Scheduled work exceeds one free invocation
- **WHEN** cleanup or rights work has more pending records than one free-plan invocation can process
- **THEN** the service processes a bounded oldest-first batch within database query and external subrequest limits and leaves the remaining work durable for later invocations

#### Scenario: Initial production admission
- **WHEN** the operator deploys a new relay
- **THEN** admission remains closed until authentication and operational notices are configured, then starts with a conservative account limit and disabled unconfigured commercial and payment features
