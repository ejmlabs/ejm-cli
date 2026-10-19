# Spec Delta

## Purpose

Let admitted CLI changes test their exact harness dependency while verifying that the source was published under the official protected harness main branch.

## ADDED Requirements

### Requirement: Dependency selection uses admitted source and protected authority

After caller adoption, trusted preparation SHALL select the harness pin from the exact admitted CLI Git commit. It SHALL accept only a full lowercase commit SHA in an ordinary small Git blob and verify that the selected commit equals or is an ancestor of protected main in the fixed official harness repository.

#### Scenario: A submitted dependency is published

- **WHEN** the admitted CLI pin names official protected harness main or an ancestor
- **THEN** preparation selects that exact dependency for the submitted CLI source and lockfile
- **AND** no branch movement substitutes a different dependency

#### Scenario: Source authority cannot be established

- **WHEN** the pin is missing, malformed or redirected through a symlink, or authority metadata is inaccessible, malformed, unprotected or divergent
- **THEN** preparation fails before executing submitted product code

### Requirement: Build input is bound to the verified dependency

Preparation SHALL verify the checked-out dependency commit and tree against the selected authority and bind its source archive digest into the admitted context. Build and review jobs SHALL receive that same source binding without checkout credentials or added write authority.

#### Scenario: Checkout differs from verified source

- **WHEN** a harness checkout has a different commit or tree from the selected authority
- **THEN** preparation rejects the source handoff

#### Scenario: Preparation succeeds

- **WHEN** the dependency checkout matches the verified selection
- **THEN** context identifies the dependency commit, tree, protected main and archive digest
- **AND** the existing credential-free quality and static review boundaries remain intact

### Requirement: Caller adoption preserves independent gates

The caller change SHALL update all three immutable shared references together, include both complete bounded canonical implementations as static source context and preserve the current harness pin, lockfile and product behavior. Existing exact-head admission, two complete actual reviews, approval, quality and protected merge gates SHALL remain required.

#### Scenario: The caller adoption PR passes

- **WHEN** the PR is checked through the previously installed workflow
- **THEN** those checks establish acceptance only for that exact caller change
- **AND** installed-selector execution and later dependency updates remain subject to fresh exact-head checks
