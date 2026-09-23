//! What this repository's `host-inventory.md` states about this crate, held to the crate.
use symbiote_contract_read::{figure, region};
use symbiote_domain::{CommandId, HostId, Timestamp};
use symbiote_host_inventory::HostPulse;

/// The sample lifetime `host-inventory.md` states is the one this crate stamps onto a pulse, in
/// milliseconds. The figure is read from the sentence it is written in and the stamp is **driven
/// through the crate's own surface**: a pulse's expiry minus its observation is the lifetime the
/// declaration beside the stamping constructors carries, so a lifetime moved in either place reds
/// here by name. Nothing is read out of the crate's source text: the value has a declaration and
/// this case drives the behaviour that applies it.
///
/// What it does not read: the prose around the figure — the one-second read cache, the ceiling on
/// usable TTL, telemetry being off by default — which the crate's own cases drive, and the
/// observation path's stamp, which carries the same declaration and whose probe the linux cases
/// drive against a real machine.
#[test]
fn the_contract_states_the_sample_lifetime_this_crate_stamps() {
    let contract = include_str!("../../../docs/contracts/host-inventory.md");

    let stated: u64 = figure(region(contract, "Samples expire after ", "."));
    let at = Timestamp(1_000);
    let pulse = HostPulse::telemetry_disabled(
        HostId::new("host-a").unwrap(),
        CommandId::new("observation-a").unwrap(),
        at,
    )
    .unwrap();
    assert_eq!(
        pulse.expires_at.0 - at.0,
        stated * 1_000,
        "the contract states a {stated}-second sample lifetime, and this crate stamps {} milliseconds",
        pulse.expires_at.0 - at.0
    );
}
