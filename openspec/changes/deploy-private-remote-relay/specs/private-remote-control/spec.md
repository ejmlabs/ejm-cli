# Spec delta

## ADDED Requirements

### Requirement: Exact queued completion

Queued remote prompts SHALL remain host-owned and revocable until admission. Admission SHALL follow successful completion of the exact target run and the end of its bounded completion observers. The host SHALL recheck the live share, expiry and target before starting queued work.

#### Scenario: Completion observers are still running

- **WHEN** the target run has a durable successful completion while a run-end observer is blocked
- **THEN** accepted queued prompts remain pending and start once each after the completion barrier, including when the target came from the local queue

#### Scenario: Sharing stops during completion

- **WHEN** sharing stops while accepted prompts wait for completion observers
- **THEN** stop acknowledgement removes those prompts before any later admission, including after the observer returns

#### Scenario: Local input and signals during cleanup

- **WHEN** a completion observer is blocked or host startup owns protected input
- **THEN** local revocation, session changes, exit and termination remain responsive, restore the terminal and prevent queued remote admission; trusted local stop also cancels retained local successors

#### Scenario: A fast local successor overtakes replay

- **WHEN** the target's driver hands off to a queued local successor that finishes before the host observes completion
- **THEN** the immutable completion decision causes the dependent remote chain to be discarded, with no admission based on apparent idleness

### Requirement: Compatible prompt byte limit

Native text mutations SHALL reject blank text and text above 16 KiB of UTF-8 bytes before acknowledgement. A rejected oversized request SHALL NOT discard valid queued prompts or start work.

#### Scenario: A prompt exceeds the limit by one byte

- **WHEN** a controller submits 16,385 UTF-8 bytes while a valid prompt is queued
- **THEN** the oversized request is rejected before queue acknowledgement and the valid prompt can still execute

#### Scenario: Multibyte prompt reaches the limit

- **WHEN** nonblank text contains multibyte characters and is exactly 16,384 UTF-8 bytes
- **THEN** the byte limit permits the request while the remaining host authority checks still apply
