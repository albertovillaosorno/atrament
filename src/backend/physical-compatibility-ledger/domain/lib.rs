// Copyright:
//   - Copyright © 2026 Alberto Villa Osorno.
// SPDX-License-Identifier:
//   - MIT
// Confidential:
//   - false
// License-File:
//   - LICENSE-MIT
//
// Boundary-Contract:
// - Owns:
//   - Transport-neutral physical compatibility evidence records.
// - Must-Not:
//   - Probe hardware, infer support, choose adapter tiers, validate
//     calibration,
//     contact devices, serialize records, or authorize physical motion.
// - Allows:
//   - Inputs: Caller-owned exact device/setup identities and acceptance
//     evidence.
//   - Outputs: One inspectable compatibility record with explicit limitations.
//   - Side effects: None.
// - Split-When:
//   - Compatibility admission or persistence gains executable authority.
// - Merge-When:
//   - Compatibility evidence becomes inseparable from one hardware adapter.
// - Summary:
//   - Keeps hardware support claims tied to exact physical evidence.
// - Description:
//   - Records device, transport, media, pen, setup, limitations, and
//     acceptance.
// - Usage:
//   - Retain evidence for one exact physically tested compatibility
//     combination.
// - Defaults:
//   - No generic device-family support or implied recertification is inferred.
//

//! Physical compatibility evidence without device-control authority.

/// Exact device and adapter identity for one compatibility record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceCompatibilityIdentity<
    AdapterTier,
    Firmware,
    Model,
    Transport,
> {
    /// Caller-owned accepted adapter tier.
    pub adapter_tier: AdapterTier,
    /// Exact firmware identity observed during acceptance.
    pub firmware: Firmware,
    /// Exact physical device model identity.
    pub model: Model,
    /// Exact tested transport or adapter mechanism.
    pub transport: Transport,
}

/// Exact physical writing setup covered by one compatibility record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalCompatibilitySetup<Paper, Pen, Settings, UsableArea> {
    /// Exact tested paper/media identity.
    pub paper: Paper,
    /// Exact tested pen identity.
    pub pen: Pen,
    /// Caller-owned tested device/adapter settings.
    pub settings: Settings,
    /// Measured usable area accepted for this setup.
    pub usable_area: UsableArea,
}

/// Complete physical compatibility evidence for one exact tested combination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalCompatibilityRecord<
    AcceptanceDate,
    DeviceIdentity,
    Evidence,
    KnownLimitations,
    Setup,
> {
    /// Exact model/firmware/transport/adapter-tier identity.
    pub device: DeviceIdentity,
    /// Physical or protocol acceptance evidence supporting this record.
    pub evidence: Evidence,
    /// Known limitations retained alongside the support claim.
    pub known_limitations: KnownLimitations,
    /// Caller-owned date of the most recent accepted physical evidence.
    pub last_acceptance_date: AcceptanceDate,
    /// Exact paper/pen/usable-area/settings setup.
    pub setup: Setup,
}
